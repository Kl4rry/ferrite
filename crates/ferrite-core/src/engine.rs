use std::{
    collections::HashMap,
    env,
    io::{self, Read},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, atomic::Ordering, mpsc},
    time::{Duration, Instant},
};

use anyhow::Result;
use ferrite_cli::Args;
use ferrite_geom::point::Point;
use ferrite_utility::{line_ending, trim::trim_path, url};
use linkify::{LinkFinder, LinkKind};
use ropey::Rope;

use crate::{
    buffer::{self, Buffer, ViewId, encoding::get_encoding},
    buffer_watcher::BufferWatcher,
    byte_size::format_byte_size,
    clipboard,
    cmd::Cmd,
    config::{
        Config,
        editor::Editor,
        keymap::{Keymap, Keymapping},
        languages::Languages,
    },
    event_loop_proxy::{EventLoopControlFlow, EventLoopProxy, UserEvent, set_proxy},
    file_explorer::FileExplorer,
    git::branch::BranchWatcher,
    indent::Indentation,
    job_manager::{JobHandle, JobManager, Progress, Progressor},
    jobs::{SaveBufferJob, ShellJobHandle},
    keymap::InputContext,
    layout::panes::{PaneKind, Panes, Rect},
    logger::{LogMessage, LoggerState},
    palette::{
        CommandPalette, PaletteMode, PalettePromptEvent,
        cmd_parser::{self, generic_cmd::CmdTemplateArg},
        completer::CompleterContext,
    },
    picker::{
        Picker,
        buffer_picker::{BufferFindProvider, BufferItem},
        file_picker::FileFindProvider,
        file_previewer::{FilePreviewer, is_text_file},
        file_scanner::FileScanner,
        global_search_picker::{GlobalSearchMatch, GlobalSearchPreviewer, GlobalSearchProvider},
    },
    spinner::Spinner,
    theme::EditorTheme,
    timer::Timer,
    watcher::FileWatcher,
    workspace::{BufferId, Workspace},
};

pub struct Engine {
    pub workspace: Workspace,
    pub themes: HashMap<String, Arc<EditorTheme>>,
    pub config: Config,
    pub palette: CommandPalette,
    pub file_picker: Option<Picker<String>>,
    pub buffer_picker: Option<Picker<BufferItem>>,
    pub global_search_picker: Option<Picker<GlobalSearchMatch>>,
    pub proxy: Box<dyn EventLoopProxy<UserEvent>>,
    pub file_scanner: Option<FileScanner>,
    pub job_manager: JobManager,
    pub save_jobs: Vec<(BufferId, JobHandle<Result<SaveBufferJob>>)>,
    pub shell_jobs: Vec<(Option<BufferId>, ShellJobHandle)>,
    pub spinner: Spinner,
    pub logger_state: LoggerState,
    pub chord: Option<String>,
    pub repeat: Option<String>,
    pub last_render_time: Duration,
    pub start_of_events: Instant,
    pub closed_buffers: Vec<PathBuf>,
    pub branch_watcher: BranchWatcher,
    pub buffer_watcher: Option<BufferWatcher>,
    pub buffer_area: Rect,
    pub force_redraw: bool,
    pub scale: f32,
    pub trim_timer: Timer,
}

#[profiling::all_functions]
impl Engine {
    pub fn new(
        args: &Args,
        proxy: Box<dyn EventLoopProxy<UserEvent>>,
        recv: mpsc::Receiver<LogMessage>,
    ) -> Result<Self> {
        set_proxy(proxy.dup());
        let mut palette = CommandPalette::new(proxy.dup());

        let config_path = Editor::get_default_location().ok();
        let mut config = match Editor::load_from_default_location() {
            Ok(config) => config,
            Err(err) => {
                palette.set_error(err);
                Editor::default()
            }
        };

        let mut config_watcher = None;
        if let Some(ref config_path) = config_path {
            match FileWatcher::new(config_path, proxy.dup()) {
                Ok(watcher) => config_watcher = Some(watcher),
                Err(err) => tracing::error!("Error starting editor config watcher: {err}"),
            }
        }

        let languages_path = Languages::get_default_location().ok();
        let languages = match Languages::load_from_default_location() {
            Ok(languages) => languages,
            Err(err) => {
                palette.set_error(err);
                Languages::default()
            }
        };

        let mut languages_watcher = None;
        if let Some(ref languages_path) = languages_path {
            match FileWatcher::new(languages_path, proxy.dup()) {
                Ok(watcher) => languages_watcher = Some(watcher),
                Err(err) => tracing::error!("Error starting language config watcher: {err}"),
            }
        }

        let keymap = Keymap::from_editor(&config);

        if config.local_clipboard {
            clipboard::set_local_clipboard(true);
        }

        let themes = EditorTheme::load_themes();
        if !themes.contains_key(&config.theme) {
            config.theme = "default".into();
        }

        let job_manager = JobManager::new(proxy.dup());

        let workspace = match Workspace::load_workspace(true, proxy.dup()) {
            Ok(workspace) => workspace,
            Err(err) => {
                tracing::error!("Error loading workspace: {err}");
                Workspace::default()
            }
        };

        let branch_watcher = BranchWatcher::new(proxy.dup())?;

        let buffer_watcher = if config.watch_open_files {
            BufferWatcher::new(proxy.dup()).ok()
        } else {
            None
        };

        let config = Config {
            editor: Arc::new(config),
            editor_path: config_path,
            editor_watcher: config_watcher,
            languages,
            languages_path,
            languages_watcher,
            keymap,
        };

        let mut engine = Self {
            workspace,
            themes,
            config,
            palette,
            file_picker: None,
            buffer_picker: None,
            global_search_picker: None,
            branch_watcher,
            proxy,
            file_scanner: None,
            job_manager,
            save_jobs: Default::default(),
            shell_jobs: Default::default(),
            spinner: Default::default(),
            chord: None,
            repeat: None,
            logger_state: LoggerState::new(recv),
            last_render_time: Duration::ZERO,
            start_of_events: Instant::now(),
            closed_buffers: Vec::new(),
            buffer_watcher,
            buffer_area: Rect {
                x: 0,
                y: 0,
                width: 10,
                height: 10,
            },
            force_redraw: false,
            scale: 1.0,
            trim_timer: Timer::default(),
        };

        let mut files_from_args = false;
        for (i, file) in args.files.iter().enumerate() {
            let file_str = file.to_string_lossy();

            if i == 0
                && let ("file", body) = url::parse_scheme(&file_str)
                && file.is_dir()
            {
                engine.cd(body);
                engine.open_file_picker();
                continue;
            }

            engine.open_url(file, false, true);
            files_from_args = true;
        }

        if files_from_args
            && let Some((current_buffer_id, view_id)) = engine.get_current_buffer_id()
        {
            let buffer = &mut engine.workspace.buffers[current_buffer_id];
            buffer.goto(view_id, args.line as i64);
            engine.workspace.panes = Panes::new(current_buffer_id, view_id);
        }

        Ok(engine)
    }

    pub fn do_polling(&mut self, control_flow: &mut EventLoopControlFlow) {
        self.logger_state.update();

        if self.config.editor.picker.file_picker_auto_reload && self.file_picker.is_none() {
            self.file_scanner = None;
        }

        if !self.config.editor.watch_open_files {
            self.buffer_watcher = None;
        } else if let Some(buffer_watcher) = &mut self.buffer_watcher {
            buffer_watcher.update(&mut self.workspace.buffers);
        } else {
            self.buffer_watcher = BufferWatcher::new(self.proxy.dup()).ok();
        }

        if let Some(config_watcher) = &mut self.config.editor_watcher
            && let Some(result) = config_watcher.poll_update()
        {
            match result {
                Ok(mut editor) => {
                    if !self.themes.contains_key(&editor.theme) {
                        editor.theme = "default".into();
                    }
                    self.config.editor = Arc::new(editor);
                    self.palette.set_msg("Reloaded editor config");
                    self.config.keymap = Keymap::from_editor(&self.config.editor);
                }
                Err(err) => self.palette.set_error(err),
            }
        }

        if let Some(config_watcher) = &mut self.config.languages_watcher
            && let Some(result) = config_watcher.poll_update()
        {
            match result {
                Ok(languages) => {
                    self.config.languages = languages;
                    self.palette.set_msg("Reloaded languages");
                }
                Err(err) => self.palette.set_error(err),
            }
        }

        if let Some(config_watcher) = &mut self.workspace.config_watcher
            && let Some(result) = config_watcher.poll_update()
        {
            match result {
                Ok(config) => {
                    self.workspace.config = config;
                    self.palette.set_msg("Reloaded workspace config");
                }
                Err(err) => self.palette.set_error(err),
            }
        }

        let mut new_buffers = Vec::new();
        for (_, buffer) in &mut self.workspace.buffers {
            if let Some(path) = buffer.file() {
                match self
                    .workspace
                    .buffer_extra_data
                    .iter_mut()
                    .find(|buffer| buffer.path == path)
                {
                    Some(buffer_data) => {
                        if let Some(view_id) = buffer.get_last_used_view() {
                            buffer.write_buffer_data(view_id, buffer_data);
                        }
                    }
                    None => {
                        if let Some(view_id) = buffer.get_last_used_view()
                            && let Some(buffer_data) = buffer.get_buffer_data(view_id)
                        {
                            new_buffers.push(buffer_data);
                        }
                    }
                }
            }
        }
        self.workspace
            .buffer_extra_data
            .extend_from_slice(&new_buffers);

        for (_, job) in &mut self.save_jobs {
            if let Ok(result) = job.try_recv() {
                match result {
                    Ok(job) => {
                        if let Some(buffer) = self.workspace.buffers.get_mut(job.buffer_id) {
                            if job.last_edit <= buffer.get_last_edit() {
                                buffer.mark_saved();
                            } else {
                                buffer.mark_history_dirty();
                            }
                        }

                        let path = job.path.file_name().unwrap_or_default().to_string_lossy();
                        self.palette.set_msg(format!(
                            "`{}` written: {}",
                            path,
                            format_byte_size(job.written)
                        ));
                    }

                    Err(e) => self.palette.set_msg(e),
                }
            }
        }
        self.save_jobs.retain(|(_, job)| !job.is_finished());

        for (buffer_id, job) in &mut self.shell_jobs {
            let mut i = 0;
            let mut dirty_buffer_id = None;
            while let Ok(result) = job.poll_progress() {
                i += 1;
                if i > 10 {
                    break;
                }
                match result {
                    Progress::End(Ok((buffer_id, rope))) => {
                        if let Some(buffer_id) = buffer_id {
                            if let Some(buffer) = self.workspace.buffers.get_mut(buffer_id) {
                                buffer.replace_rope(rope);
                                dirty_buffer_id = Some(buffer_id);
                            }
                        } else {
                            self.palette.set_msg(rope.to_string());
                        }
                    }
                    Progress::End(Err(e)) => self.palette.set_error(e),
                    Progress::Progress((buffer_id, rope)) => {
                        if let Some(buffer) = self.workspace.buffers.get_mut(buffer_id) {
                            buffer.replace_rope(rope);
                            dirty_buffer_id = Some(buffer_id);
                        }
                    }
                }
            }

            if let Some(buffer_id) = dirty_buffer_id
                && let Some(buffer) = self.workspace.buffers.get_mut(buffer_id)
            {
                buffer.auto_detect_language();
                buffer.queue_syntax_update();
            }

            if let Some(buffer_id) = buffer_id
                && !self.workspace.buffers.contains_key(*buffer_id)
            {
                job.kill();
            }
        }

        // Its kinda hard to clean up all views created correctly. Here we just
        // find all views not connected to a pane and we just remove them.
        for (buffer_id, buffer) in &mut self.workspace.buffers {
            for view_id in buffer.views.keys().collect::<Vec<_>>() {
                if !self
                    .workspace
                    .panes
                    .contains(PaneKind::Buffer(buffer_id, view_id))
                {
                    buffer.views.remove(view_id);
                }
            }
        }

        self.shell_jobs.retain(|job| !job.1.is_finished());

        self.job_manager.poll_jobs();

        if self.trim_timer.every(Duration::from_secs(20)) {
            crate::malloc::trim(0);
        }

        let duration = self
            .spinner
            .update(!self.save_jobs.is_empty() || !self.shell_jobs.is_empty());
        *control_flow = EventLoopControlFlow::WaitMax(duration);
    }

    pub fn handle_input_command(&mut self, input: Cmd, control_flow: &mut EventLoopControlFlow) {
        if let Some(repeat) = &mut self.repeat {
            match input {
                Cmd::Char { ch } if ch.is_ascii_digit() => {
                    repeat.push(ch);
                }
                _ => {
                    let number = match self
                        .repeat
                        .take()
                        .map(|s| if s.is_empty() { String::from("0") } else { s })
                        .unwrap()
                        .parse::<u16>()
                    {
                        Ok(number) => number,
                        Err(err) => {
                            self.palette.set_error(err);
                            return;
                        }
                    };
                    if input.is_repeatable() {
                        self.palette.set_msg(format!("Repeated: {input}"));
                        for _ in 0..number {
                            self.handle_single_input_command(input.clone(), control_flow);
                        }
                    } else {
                        self.handle_single_input_command(input, control_flow);
                        self.repeat = None;
                    }
                }
            }
        } else {
            self.handle_single_input_command(input, control_flow);
        }

        if let Some(repeat) = &self.repeat {
            self.palette.set_msg(format!("Repeat: {repeat}"));
        }
    }

    pub fn handle_single_input_command(
        &mut self,
        input: Cmd,
        control_flow: &mut EventLoopControlFlow,
    ) {
        if !matches!(input, Cmd::InputMode { .. }) {
            self.chord = None;
        }
        match input {
            Cmd::ForceRedraw => self.force_redraw = true,
            Cmd::RotateFile => {
                if let Some((buffer, _)) = self.get_current_buffer() {
                    match buffer.get_next_file() {
                        Ok(file) => {
                            self.open_file(file, false);
                        }
                        Err(err) => self.palette.set_error(err),
                    };
                };
            }
            Cmd::Repeat => {
                self.repeat = Some(String::new());
            }
            Cmd::ReopenBuffer => self.reopen_last_closed_buffer(),
            Cmd::UrlOpen => self.open_selected_url(),
            Cmd::OpenShellPalette => {
                self.hide_pickers();
                self.palette.focus(
                    "$ ",
                    PaletteMode::Shell,
                    CompleterContext::new(
                        self.themes.keys().cloned().collect(),
                        self.get_action_names(),
                        true,
                        Some(CmdTemplateArg::Path),
                    ),
                );
            }
            Cmd::InputMode { name } => {
                if name == "normal" {
                    self.chord = None;
                } else {
                    self.chord = Some(name);
                }
            }
            Cmd::GrowPane => {
                self.workspace.panes.grow_current(self.buffer_area);
            }
            Cmd::ShrinkPane => {
                self.workspace.panes.shrink_current(self.buffer_area);
            }
            Cmd::Quit => {
                self.quit(control_flow);
            }
            Cmd::FocusPalette if !self.palette.has_focus() => {
                self.hide_pickers();
                self.palette.focus(
                    "> ",
                    PaletteMode::Command,
                    CompleterContext::new(
                        self.themes.keys().cloned().collect(),
                        self.get_action_names(),
                        false,
                        None,
                    ),
                );
            }
            Cmd::PromptGoto => {
                self.hide_pickers();
                self.palette.focus(
                    "goto: ",
                    PaletteMode::Goto,
                    CompleterContext::new(
                        self.themes.keys().cloned().collect(),
                        self.get_action_names(),
                        false,
                        None,
                    ),
                );
            }
            Cmd::Search => self.search(),
            Cmd::Replace => self.start_replace(),
            Cmd::GlobalSearch => self.global_search(),
            Cmd::CaseInsensitive => {
                Arc::make_mut(&mut self.config.editor).case_insensitive_search =
                    !self.config.editor.case_insensitive_search;
                if let Some(PaletteMode::Search) = self.palette.mode() {
                    self.palette.update_prompt(self.get_search_prompt(false));
                }
                if let Some(PaletteMode::GlobalSearch) = self.palette.mode() {
                    self.palette.update_prompt(self.get_search_prompt(true));
                }
            }
            Cmd::Escape => {
                if self.repeat.is_some() {
                    self.repeat = None;
                } else if self.palette.has_focus() {
                    self.palette.reset();
                } else if self.chord.is_some()
                    || self.file_picker.is_some()
                    || self.buffer_picker.is_some()
                    || self.global_search_picker.is_some()
                {
                    self.chord = None;
                    self.hide_pickers();
                } else if let PaneKind::FileExplorer(_) = self.workspace.panes.get_current_pane() {
                    self.force_close_current_buffer();
                } else if let Some((buffer, view_id)) = self.get_current_buffer_mut() {
                    let _ = buffer.handle_input(view_id, Cmd::Escape);
                }
            }
            Cmd::OpenFilePicker => self.open_file_picker(),
            Cmd::OpenBufferPicker => self.open_buffer_picker(),
            Cmd::OpenFileExplorer { path } => self.open_file_explorer(path),
            Cmd::FilePickerReload => {
                self.file_scanner = Some(FileScanner::new(
                    env::current_dir().unwrap_or(PathBuf::from(".")),
                    &self.config.editor,
                ));
            }
            Cmd::ReplaceAll { text } => {
                if let Some((buffer, view_id)) = self.get_current_buffer_mut() {
                    buffer.replace_all(view_id, text);
                }
            }
            Cmd::SortLines { ascending } => {
                if let Some((buffer, view_id)) = self.get_current_buffer_mut() {
                    buffer.sort_lines(view_id, ascending);
                }
            }
            Cmd::Path => match self.try_get_current_buffer_path() {
                Some(path) => self.palette.set_msg(path.to_string_lossy()),
                None => self
                    .palette
                    .set_error("No path has been set for the current buffer"),
            },
            Cmd::About => {
                self.palette.set_msg(format!(
                    "ferrite\nVersion: {}\nCommit: {}",
                    crate::about::version(),
                    crate::about::git_hash(),
                ));
            }
            Cmd::Pwd => match env::current_dir() {
                Ok(path) => self.palette.set_msg(path.to_string_lossy()),
                Err(err) => self.palette.set_error(err),
            },
            Cmd::Cd { path } => {
                self.cd(path);
            }
            Cmd::Split { direction } => {
                let (buffer_id, view_id) = match self.workspace.panes.get_current_pane() {
                    PaneKind::Buffer(buffer_id, _) => {
                        let view_id = self.workspace.buffers[buffer_id].create_view();
                        self.load_view_data(buffer_id, view_id);
                        (buffer_id, view_id)
                    }
                    _ => self.get_next_buffer(),
                };

                self.workspace
                    .panes
                    .split(PaneKind::Buffer(buffer_id, view_id), direction);
            }
            Cmd::RunShellCmd { args, pipe } => {
                let cmd = args
                    .into_iter()
                    .map(|s| String::from(s.to_string_lossy()))
                    .collect::<Vec<_>>()
                    .join(" ");
                self.run_shell_command(cmd, None, pipe, false);
            }
            Cmd::FormatSelection => self.format_selection_current_buffer(),
            Cmd::Format => {
                if let PaneKind::Buffer(buffer_id, view_id) =
                    self.workspace.panes.get_current_pane()
                {
                    self.format_buffer(buffer_id, view_id);
                }
            }
            Cmd::OpenFile { path } => {
                self.open_file(path, false);
            }
            Cmd::Save { path } => {
                let PaneKind::Buffer(buffer_id, _) = self.workspace.panes.get_current_pane() else {
                    return;
                };

                self.save_buffer(buffer_id, path);
            }
            Cmd::SaveAll => {
                let mut buffers_to_save = Vec::new();
                for (buffer_id, buffer) in &self.workspace.buffers {
                    if buffer.file().is_some() {
                        buffers_to_save.push(buffer_id);
                    }
                }

                for buffer_id in buffers_to_save {
                    self.save_buffer(buffer_id, None);
                }
            }
            Cmd::Language { language } => {
                let PaneKind::Buffer(buffer_id, _) = self.workspace.panes.get_current_pane() else {
                    return;
                };
                match language {
                    Some(language) => {
                        if let Err(err) = self.workspace.buffers[buffer_id]
                            .set_langauge(&language, self.proxy.dup())
                        {
                            self.palette.set_error(err);
                        }
                    }
                    None => self
                        .palette
                        .set_msg(self.workspace.buffers[buffer_id].language_name()),
                }
            }
            Cmd::Encoding { encoding } => {
                let PaneKind::Buffer(buffer_id, _) = self.workspace.panes.get_current_pane() else {
                    return;
                };
                match encoding {
                    Some(encoding) => {
                        match get_encoding(&encoding) {
                            Some(encoding) => self.workspace.buffers[buffer_id].encoding = encoding,
                            None => self.palette.set_error("unknown encoding, these encodings are supported: https://docs.rs/encoding_rs/latest/encoding_rs"),
                        }
                    }
                    None => self
                    .palette
                    .set_msg(self.workspace.buffers[buffer_id].encoding.name()),
                }
            }
            Cmd::Indent { indent } => {
                let PaneKind::Buffer(buffer_id, _) = self.workspace.panes.get_current_pane() else {
                    return;
                };
                match indent {
                    Some(indent) => {
                        if let Ok(spaces) = indent.parse::<NonZeroUsize>() {
                            self.workspace.buffers[buffer_id].indent = Indentation::Spaces(spaces);
                        } else if indent == "tabs" {
                            self.workspace.buffers[buffer_id].indent =
                                Indentation::Tabs(NonZeroUsize::new(1).unwrap());
                        } else {
                            self.palette
                                .set_error("Indentation must be a number or `tabs`");
                        }
                    }
                    None => match self.workspace.buffers[buffer_id].indent {
                        Indentation::Tabs(_) => self.palette.set_msg("tabs"),
                        Indentation::Spaces(amount) => {
                            self.palette.set_msg(format!("{} space(s)", amount))
                        }
                    },
                }
            }
            Cmd::LineEnding { line_ending } => {
                let PaneKind::Buffer(buffer_id, _) = self.workspace.panes.get_current_pane() else {
                    return;
                };
                match line_ending {
                    Some(line_ending) => {
                        self.workspace.buffers[buffer_id].line_ending = line_ending
                    }
                    None => {
                        self.palette
                            .set_msg(match self.workspace.buffers[buffer_id].line_ending {
                                line_ending::LineEnding::Crlf => "crlf",
                                line_ending::LineEnding::LF => "lf",
                                _ => unreachable!(),
                            })
                    }
                }
            }
            Cmd::New { path } => {
                if let Some(path) = path {
                    match Buffer::builder().with_path(path).build() {
                        Ok(mut buffer) => {
                            let view_id = buffer.create_view();
                            self.insert_buffer(buffer, view_id, true);
                        }
                        Err(err) => self.palette.set_error(err),
                    }
                } else {
                    let mut buffer = Buffer::new();
                    let view_id = buffer.create_view();
                    self.insert_buffer(buffer, view_id, true);
                }
            }
            Cmd::Reload => {
                let PaneKind::Buffer(buffer_id, _) = self.workspace.panes.get_current_pane() else {
                    return;
                };
                if self.workspace.buffers[buffer_id].is_dirty() {
                    self.palette.set_prompt(
                        "The buffer is unsaved are you sure you want to reload?",
                        ('y', PalettePromptEvent::Reload),
                        ('n', PalettePromptEvent::Nop),
                    );
                } else if let Err(err) = self.workspace.buffers[buffer_id].reload() {
                    self.palette.set_error(err)
                };
            }
            Cmd::ReloadAll => {
                for buffer in self.workspace.buffers.values_mut() {
                    if buffer.file().is_some() && buffer.is_dirty() {
                        self.palette
                            .set_error(format!("`{}` is dirty cannot reload", buffer.name()));
                        continue;
                    }

                    if buffer.file().is_some()
                        && let Err(err) = buffer.reload()
                    {
                        self.palette.set_error(err);
                    }
                }
            }
            Cmd::Goto { line } => {
                let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane()
                else {
                    return;
                };
                self.workspace.buffers[buffer_id].goto(view_id, line);
            }
            Cmd::Case { case } => {
                let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane()
                else {
                    return;
                };
                self.workspace.buffers[buffer_id].transform_case(view_id, case);
            }
            Cmd::ForceQuit => *control_flow = EventLoopControlFlow::Exit,
            Cmd::Logger => {
                self.logger_state.lines_scrolled_up = 0.0;
                self.workspace.panes.replace_current(PaneKind::Logger);
            }
            Cmd::Theme { theme } => match theme {
                Some(theme) =>
                {
                    #[allow(clippy::map_entry)]
                    if self.themes.contains_key(&theme) {
                        Arc::make_mut(&mut self.config.editor).theme = theme;
                    } else {
                        self.palette.set_error("Theme not found");
                    }
                }
                None => {
                    self.palette.set_msg(&self.config.editor.theme);
                }
            },
            Cmd::BufferPickerOpen => self.open_buffer_picker(),
            Cmd::FilePickerOpen => {
                if self.config.editor.picker.file_picker_auto_reload {
                    self.file_scanner = Some(FileScanner::new(
                        env::current_dir().unwrap_or(PathBuf::from(".")),
                        &self.config.editor,
                    ));
                }
                self.open_file_picker();
            }
            Cmd::OpenConfig => self.open_config(),
            Cmd::DefaultConfig => self.open_default_config(),
            Cmd::OpenLanguages => self.open_languages(),
            Cmd::DefaultLanguages => self.open_default_languages(),
            Cmd::OpenKeymap => self.open_keymap(),
            Cmd::DefaultKeymap => self.open_default_keymap(),
            Cmd::OpenWorkspaceConfig => self.open_workspace_config(),
            Cmd::ForceClose => self.force_close_current_buffer(),
            Cmd::Close => self.close_current_buffer(),
            Cmd::ClosePane => self.close_pane(),
            Cmd::RevertBuffer => {
                let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane()
                else {
                    return;
                };
                let _ = self.workspace.buffers[buffer_id].handle_input(view_id, Cmd::RevertBuffer);
            }
            Cmd::GitReload => self.branch_watcher.force_reload(),
            Cmd::SwitchPane { direction } => {
                self.workspace
                    .panes
                    .switch_pane_direction(direction, self.buffer_area);
            }
            Cmd::ZoomIn => {
                self.scale += 0.1;
                self.palette
                    .set_msg(format!("Zoom: {}%", (self.scale * 100.0).round() as u64));
            }
            Cmd::ZoomOut => {
                self.scale -= 0.1;
                self.palette
                    .set_msg(format!("Zoom: {}%", (self.scale * 100.0).round() as u64));
            }
            Cmd::ResetZoom => {
                self.scale = 1.0;
                self.palette
                    .set_msg(format!("Zoom: {}%", (self.scale * 100.0).round() as u64));
            }
            Cmd::KillJob => {
                if let Some((current_buffer_id, _)) = self.get_current_buffer_id() {
                    for (buffer_id, job) in &mut self.shell_jobs {
                        if let Some(buffer_id) = buffer_id
                            && *buffer_id == current_buffer_id
                        {
                            job.kill();
                        }
                    }
                }
            }
            Cmd::RunAction { name } => match self
                .workspace
                .config
                .actions
                .get(&name)
                .or_else(|| self.config.editor.actions.get(&name))
            {
                Some(args) => {
                    self.run_shell_command(args.join(" "), None, true, false);
                }
                None => {
                    self.palette.set_error(format!("Action '{name}' not found"));
                }
            },
            Cmd::Duplicate => {
                if let Some((buffer, _)) = self.get_current_buffer() {
                    let mut new_buffer = buffer.clone();
                    new_buffer.views.clear();
                    let _ = new_buffer.set_file(None::<&str>); // NOTE cannot fail
                    let view_id = new_buffer.create_view();
                    let (buffer_id, _) = self.insert_buffer(new_buffer, view_id, true);
                    self.load_view_data(buffer_id, view_id);
                }
            }
            Cmd::OpenRename => {
                let path = match self.workspace.panes.get_current_pane() {
                    PaneKind::Buffer(buffer_id, _) => {
                        let Some(path) = self.workspace.buffers[buffer_id].file() else {
                            self.palette.set_error("Cannot rename buffer without path");
                            return;
                        };
                        path.to_path_buf()
                    }
                    PaneKind::FileExplorer(file_explorer_id) => {
                        match self.workspace.file_explorers[file_explorer_id].current() {
                            Some(entry) => entry.path.to_path_buf(),
                            None => return,
                        }
                    }
                    _ => {
                        self.palette.set_error("Only buffers are renameable");
                        return;
                    }
                };
                self.palette.focus(
                    "rename: ",
                    PaletteMode::Rename { path: path.clone() },
                    CompleterContext::new(
                        self.themes.keys().cloned().collect(),
                        self.get_action_names(),
                        false,
                        None,
                    ),
                );
                self.palette.set_line(path.to_string_lossy());
            }
            input => {
                if self.palette.has_focus() {
                    let _ = self.palette.handle_input(input);
                } else if let Some(picker) = &mut self.file_picker {
                    let _ = picker.handle_input(input);
                    if let Some(path) = picker.get_choice() {
                        self.hide_pickers();
                        self.open_file(path, false);
                    }
                } else if let Some(picker) = &mut self.buffer_picker {
                    let _ = picker.handle_input(input);
                    if let Some(choice) = picker.get_choice() {
                        self.workspace.buffers[choice.id].update_interact(None);
                        self.buffer_picker = None;

                        let buffer = &mut self.workspace.buffers[choice.id];
                        let view_id = buffer.create_view();
                        self.load_view_data(choice.id, view_id);

                        let old = self
                            .workspace
                            .panes
                            .replace_current(PaneKind::Buffer(choice.id, view_id));
                        if let PaneKind::Buffer(id, view_id) = old {
                            let buffer = &mut self.workspace.buffers[id];
                            buffer.remove_view(view_id);
                            if buffer.is_disposable() {
                                self.workspace.buffers.remove(id);
                            }
                        }
                    }
                } else if let Some(picker) = &mut self.global_search_picker {
                    let _ = picker.handle_input(input);
                    if let Some(choice) = picker.get_choice() {
                        self.global_search_picker = None;
                        let guard = choice.buffer.lock().unwrap();
                        if let Some(file) = guard.file()
                            && self.open_file(file, false)
                        {
                            let view_id = guard.get_first_view().unwrap();
                            let cursor_line = guard.cursor_line_idx(view_id, 0);
                            let cursor_col = guard.cursor_grapheme_column(view_id, 0);
                            let anchor_line = guard.anchor_line_idx(view_id, 0);
                            let anchor_col = guard.anchor_grapheme_column(view_id, 0);
                            if let Some((buffer, view_id)) = self.get_current_buffer_mut() {
                                buffer.select_area(
                                    view_id,
                                    Point::new(cursor_col, cursor_line),
                                    Point::new(anchor_col, anchor_line),
                                );
                                // A buffers default amount of lines when newly opened is too large
                                // and the view will not jump to it.
                                buffer.set_view_lines(view_id, 10);
                                buffer.center_on_main_cursor(view_id);
                            }
                        }
                    }
                } else {
                    match self.workspace.panes.get_current_pane() {
                        PaneKind::Buffer(buffer_id, view_id) => {
                            if let Err(err) =
                                self.workspace.buffers[buffer_id].handle_input(view_id, input)
                            {
                                self.palette.set_error(err);
                            }
                        }
                        PaneKind::FileExplorer(file_explorer_id) => {
                            let cmd =
                                self.workspace.file_explorers[file_explorer_id].handle_input(input);
                            if cmd == Cmd::Nop {
                                return;
                            }
                            self.handle_single_input_command(cmd, control_flow);
                        }
                        PaneKind::Logger => self.logger_state.handle_input(input),
                    }
                }
            }
        }
    }

    pub fn handle_search(&mut self, text: String) {
        match self.workspace.panes.get_current_pane() {
            PaneKind::Buffer(buffer_id, view_id) => {
                self.workspace.buffers[buffer_id].start_search(
                    view_id,
                    self.proxy.dup(),
                    text,
                    self.config.editor.case_insensitive_search,
                );
            }
            PaneKind::FileExplorer(file_explorer_id) => {
                self.workspace.file_explorers[file_explorer_id].handle_search(text);
            }
            PaneKind::Logger => (),
        }
    }

    pub fn handle_app_event(&mut self, event: UserEvent, control_flow: &mut EventLoopControlFlow) {
        match event {
            UserEvent::Wake => (),
            #[allow(clippy::single_match)]
            UserEvent::PalettePreview { mode, content } => match mode {
                PaletteMode::Search => self.handle_search(content),
                _ => (),
            },
            UserEvent::PaletteFinished { mode, content } => match mode {
                PaletteMode::Command => match cmd_parser::parse_cmd(&content) {
                    Ok(cmd) => {
                        self.palette.reset();
                        self.handle_single_input_command(cmd, control_flow);
                    }
                    Err(err) => self.palette.set_error(err),
                },
                PaletteMode::Goto => {
                    self.palette.reset();
                    if let Ok(line) = content.trim().parse::<i64>() {
                        let PaneKind::Buffer(buffer_id, view_id) =
                            self.workspace.panes.get_current_pane()
                        else {
                            return;
                        };
                        self.workspace.buffers[buffer_id].goto(view_id, line);
                    }
                }
                PaletteMode::Search => {
                    self.handle_search(content);
                    self.palette.unfocus();
                }
                PaletteMode::Replace => {
                    self.palette.unfocus();
                    let PaneKind::Buffer(buffer_id, view_id) =
                        self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    let buffer = &mut self.workspace.buffers[buffer_id];
                    buffer.views[view_id].replacement = Some(content);
                }
                PaletteMode::GlobalSearch => {
                    self.palette.unfocus();
                    let global_search_provider = GlobalSearchProvider::new(
                        content,
                        self.config.editor.picker,
                        self.config.editor.case_insensitive_search,
                    );
                    self.global_search_picker = Some(Picker::new(
                        global_search_provider,
                        Some(Box::new(GlobalSearchPreviewer)),
                        self.proxy.dup(),
                        None,
                    ));
                }
                PaletteMode::Shell => {
                    self.palette.reset();
                    self.run_shell_command(
                        content,
                        None,
                        self.config.editor.pipe_shell_palette,
                        false,
                    );
                }
                PaletteMode::Rename { path } => match self.rename_file(path, content) {
                    Ok(_) => self.palette.reset(),
                    Err(err) => self.palette.set_error(err),
                },
            },
            UserEvent::PromptEvent(event) => match event {
                PalettePromptEvent::Nop => (),
                PalettePromptEvent::Reload => {
                    let PaneKind::Buffer(buffer_id, _) = self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    if let Err(err) = self.workspace.buffers[buffer_id].reload() {
                        self.palette.set_error(err);
                    }
                }
                PalettePromptEvent::Quit => *control_flow = EventLoopControlFlow::Exit,
                PalettePromptEvent::CloseCurrent => self.force_close_current_buffer(),
            },
        }
    }

    pub fn format_selection_current_buffer(&mut self) {
        let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane() else {
            return;
        };
        let buffer_lang = self.workspace.buffers[buffer_id].language_name();
        let config = self.config.languages.from_name(buffer_lang);
        let Some(config) = config else {
            self.palette
                .set_error(format!("No language config found for `{buffer_lang}`"));

            return;
        };

        let Some(fmt) = &config.format_selection else {
            self.palette
                .set_error(format!("No selection formatter found for `{buffer_lang}`"));
            return;
        };

        if let Err(err) = self.workspace.buffers[buffer_id].format_selection(view_id, fmt) {
            self.palette.set_error(err);
        }
    }

    pub fn format_buffer(&mut self, buffer_id: BufferId, view_id: ViewId) {
        let buffer_lang = self.workspace.buffers[buffer_id].language_name();
        let config = self.config.languages.from_name(buffer_lang);
        let Some(config) = config else {
            self.palette
                .set_error(format!("No language config found for `{buffer_lang}`"));
            return;
        };

        let Some(fmt) = &config.format else {
            self.palette
                .set_error(format!("No formatter found for `{buffer_lang}`"));
            return;
        };

        if let Err(err) = self.workspace.buffers[buffer_id].format(Some(view_id), fmt) {
            self.palette.set_error(err);
        }
    }

    fn cd(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        if let Err(err) = self.workspace.save_workspace() {
            self.palette.set_error(err);
        }
        match env::set_current_dir(path) {
            Ok(_) => {
                self.hide_pickers();

                self.file_scanner = Some(FileScanner::new(
                    env::current_dir().unwrap_or(PathBuf::from(".")),
                    &self.config.editor,
                ));

                match BranchWatcher::new(self.proxy.dup()) {
                    Ok(branch_watcher) => self.branch_watcher = branch_watcher,
                    Err(err) => {
                        let msg = format!("Error creating branch watcher: {err}");
                        tracing::error!(msg);
                        self.palette.set_error(msg);
                    }
                }

                self.workspace = match Workspace::load_workspace(true, self.proxy.dup()) {
                    Ok(workspace) => workspace,
                    Err(err) => {
                        let msg = format!("Error loading workspace: {err}");
                        tracing::error!(msg);
                        self.palette.set_error(msg);
                        Workspace::default()
                    }
                };

                self.palette
                    .set_msg(format!("Set working dir to: {}", path.to_string_lossy()));
            }
            Err(err) => self.palette.set_error(format!("{err}")),
        }
    }

    fn open_url(&mut self, url: impl AsRef<Path>, open_with_os: bool, create_file: bool) {
        let url = url.as_ref();
        let url_str = url.to_string_lossy();
        let (scheme, body) = url::parse_scheme(&url_str);
        tracing::info!("Opening url: {}://{}", scheme, body);

        match scheme {
            "man" => self.open_manpage(body),
            _ => {
                if scheme == "file" && (!open_with_os || is_text_file(body).unwrap_or(false)) {
                    self.open_file(body, create_file);
                    return;
                }
                if let Err(err) = opener::open(url) {
                    self.palette.set_error(err);
                }
            }
        }
    }

    pub fn open_selected_url(&mut self) {
        if let Some((buffer_id, view_id)) = self.get_current_buffer_id() {
            for i in 0..self.workspace.buffers[buffer_id].views[view_id]
                .cursors
                .len()
            {
                let selection = self.workspace.buffers[buffer_id].get_selection(view_id, i);
                let mut finder = LinkFinder::new();
                finder.kinds(&[LinkKind::Url]);
                let spans: Vec<_> = finder.spans(&selection).collect();
                if spans.is_empty() {
                    self.open_url(&selection, true, false);
                } else {
                    for span in spans {
                        self.open_url(span.as_str(), true, false);
                    }
                }
            }
        }
    }

    pub fn open_file(&mut self, path: impl AsRef<Path>, create_file: bool) -> bool {
        let real_path = match dunce::canonicalize(&path) {
            Ok(path) => path,
            Err(err) => match err.kind() {
                // NOTE: it might be broken that we do not canonicalize this file path as some code
                // assumes that all paths are absolute
                io::ErrorKind::NotFound if create_file => path.as_ref().to_path_buf(),
                _ => {
                    self.palette.set_error(err);
                    return false;
                }
            },
        };

        match self.workspace.buffers.iter_mut().find(|(_, buffer)| {
            buffer
                .file()
                .and_then(|path| dunce::canonicalize(path).ok())
                .as_deref()
                == Some(&real_path)
        }) {
            Some((id, buffer)) => {
                buffer.update_interact(None);
                let view_id = buffer.create_view();
                self.load_view_data(id, view_id);
                let replaced = self
                    .workspace
                    .panes
                    .replace_current(PaneKind::Buffer(id, view_id));
                if let PaneKind::Buffer(buffer_id, view_id) = replaced {
                    self.workspace.buffers[buffer_id].remove_view(view_id);
                }
                true
            }
            None => match Buffer::builder().from_file(&real_path).build() {
                Ok(mut buffer) => {
                    let view_id = buffer.create_view();
                    let (buffer_id, _) = self.insert_buffer(buffer, view_id, true);
                    self.load_view_data(buffer_id, view_id);
                    true
                }
                Err(err) => match err.kind() {
                    io::ErrorKind::NotFound if create_file => {
                        match Buffer::builder().with_path(path.as_ref()).build() {
                            Ok(mut buffer) => {
                                let view_id = buffer.create_view();
                                let (buffer_id, _) = self.insert_buffer(buffer, view_id, true);
                                self.load_view_data(buffer_id, view_id);
                                true
                            }
                            Err(err) => {
                                self.palette.set_error(err);
                                false
                            }
                        }
                    }
                    _ => {
                        self.palette.set_error(err);
                        false
                    }
                },
            },
        }
    }

    fn open_manpage(&mut self, page: &str) {
        // TODO: Make width variable
        self.run_shell_command(
            format!("MANWIDTH=80 man {page}"),
            Some(format!("man://{page}")),
            true,
            true,
        );
    }

    pub fn quit(&mut self, control_flow: &mut EventLoopControlFlow) {
        let unsaved: Vec<_> = self
            .workspace
            .buffers
            .iter()
            .filter_map(|(_, buffer)| {
                if buffer.is_dirty() {
                    Some(buffer.name().to_string())
                } else {
                    None
                }
            })
            .collect();

        if !unsaved.is_empty() {
            self.palette.set_prompt(
                format!(
                    "You have {} unsaved buffer(s): {:?}, Are you sure you want to exit?",
                    unsaved.len(),
                    unsaved
                ),
                ('y', PalettePromptEvent::Quit),
                ('n', PalettePromptEvent::Nop),
            );
        } else if self.config.editor.always_prompt_on_exit {
            self.palette.set_prompt(
                "Are you sure you want to exit?",
                ('y', PalettePromptEvent::Quit),
                ('n', PalettePromptEvent::Nop),
            );
        } else {
            *control_flow = EventLoopControlFlow::Exit;
        }
    }

    pub fn open_buffer_picker(&mut self) {
        self.hide_pickers();
        self.palette.reset();
        let mut buffers: Vec<_> = self
            .workspace
            .buffers
            .iter()
            .map(|(id, buffer)| BufferItem {
                id,
                dirty: buffer.is_dirty(),
                name: {
                    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::new());
                    let current_dir = current_dir.to_string_lossy();
                    buffer
                        .file()
                        .map(|path| trim_path(&current_dir, path))
                        .unwrap_or_else(|| buffer.name().to_string())
                },
                order: buffer.get_last_interact(),
            })
            .collect();

        buffers.sort_by(|a, b| b.order.cmp(&a.order));
        let buffers: boxcar::Vec<_> = buffers.into_iter().collect();

        self.buffer_picker = Some(Picker::new(
            BufferFindProvider(Arc::new(buffers)),
            Some(Box::new(self.workspace.buffers.clone())),
            self.proxy.dup(),
            self.try_get_current_buffer_path(),
        ));
    }

    pub fn open_file_picker(&mut self) {
        self.palette.reset();
        self.buffer_picker = None;
        if self.file_scanner.is_none() || self.config.editor.picker.file_picker_auto_reload {
            self.file_scanner = Some(FileScanner::new(
                env::current_dir().unwrap_or(PathBuf::from(".")),
                &self.config.editor,
            ));
        }
        self.file_picker = Some(Picker::new(
            FileFindProvider(self.file_scanner.as_ref().unwrap().subscribe()),
            Some(Box::new(FilePreviewer::new(self.proxy.dup()))),
            self.proxy.dup(),
            self.try_get_current_buffer_path(),
        ));
    }

    pub fn open_config(&mut self) {
        match &self.config.editor_path {
            Some(path) => {
                self.open_file(path.clone(), false);
            }
            None => self.palette.set_error("Could not locate the config file"),
        }
    }

    pub fn open_default_config(&mut self) {
        let mut buffer = Buffer::builder()
            .with_text(Editor::DEFAULT)
            .with_name("default_config.toml")
            .build()
            .unwrap();
        let view_id = buffer.create_view();
        self.insert_buffer(buffer, view_id, true);
    }

    pub fn open_languages(&mut self) {
        match &self.config.languages_path {
            Some(path) => {
                self.open_file(path.clone(), false);
            }
            None => self
                .palette
                .set_error("Could not locate the languages file"),
        }
    }

    pub fn open_default_languages(&mut self) {
        let mut buffer = Buffer::builder()
            .with_text(Languages::DEFAULT)
            .with_name("default_languages.toml")
            .build()
            .unwrap();
        let view_id = buffer.create_view();
        self.insert_buffer(buffer, view_id, true);
    }

    pub fn open_keymap(&mut self) {
        let keymap = self.config.keymap.to_map();
        let data = toml::to_string_pretty(&keymap).unwrap();
        let mut buffer = Buffer::builder()
            .with_text(&format!(
                "# This are the current loaded keybinds. Editing this file does nothing.\n\n{}",
                data
            ))
            .with_name("keymap.toml")
            .read_only(false)
            .build()
            .unwrap();
        let view_id = buffer.create_view();
        self.insert_buffer(buffer, view_id, true);
    }

    pub fn open_default_keymap(&mut self) {
        let keymap = Keymap::default().to_map();
        let content = toml::to_string_pretty(&keymap).unwrap();
        let mut buffer = Buffer::builder()
            .with_text(&content)
            .with_name("default_keymap.toml")
            .build()
            .unwrap();
        let view_id = buffer.create_view();
        self.insert_buffer(buffer, view_id, true);
    }

    pub fn open_workspace_config(&mut self) {
        self.open_file(crate::workspace::get_config_path("."), false);
    }

    pub fn open_file_explorer(&mut self, path: Option<PathBuf>) {
        let file_explorer_id =
            self.workspace
                .file_explorers
                .insert(FileExplorer::new(path.unwrap_or_else(|| {
                    self.get_current_buffer()
                        .and_then(|(buffer, _)| {
                            buffer
                                .file()
                                .and_then(|path| path.parent().map(|path| path.to_owned()))
                        })
                        .unwrap_or_else(|| std::env::current_dir().unwrap())
                })));
        let old = self
            .workspace
            .panes
            .replace_current(PaneKind::FileExplorer(file_explorer_id));
        match old {
            PaneKind::Buffer(buffer_id, view_id) => {
                self.workspace.buffers[buffer_id].remove_view(view_id);
            }
            PaneKind::FileExplorer(file_explorer_id) => {
                self.workspace.file_explorers.remove(file_explorer_id);
            }
            PaneKind::Logger => (),
        }
    }

    pub fn close_current_buffer(&mut self) {
        let Some((buffer, _)) = self.get_current_buffer() else {
            self.force_close_current_buffer();
            return;
        };

        if !buffer.is_dirty() {
            self.force_close_current_buffer();
            return;
        }

        self.palette.set_prompt(
            "Current buffer has unsaved changes are you sure you want to close it?",
            ('y', PalettePromptEvent::CloseCurrent),
            ('n', PalettePromptEvent::Nop),
        );
    }

    fn load_view_data(&mut self, buffer_id: BufferId, view_id: ViewId) {
        if let Some(real_path) = self.workspace.buffers[buffer_id].file()
            && let Some(buffer_data) = self
                .workspace
                .buffer_extra_data
                .iter()
                .find(|b| b.path == real_path)
        {
            let buffer = &mut self.workspace.buffers[buffer_id];
            buffer.load_view_data(view_id, buffer_data);
            buffer.load_buffer_data(buffer_data);
        }

        let _ = self.workspace.buffers[buffer_id].handle_input(view_id, Cmd::Nop);
    }

    /// Gets a buffer that can be used to replace the current pane with
    fn get_next_buffer(&mut self) -> (BufferId, ViewId) {
        let mut next_buffer = None;
        let mut buffers: Vec<_> = self.workspace.buffers.iter_mut().collect();
        buffers.sort_by(|a, b| b.1.get_last_interact().cmp(&a.1.get_last_interact()));
        for (buffer_id, buffer) in &mut buffers {
            if !self.workspace.panes.contains_buffer(*buffer_id) {
                let view_id = buffer.create_view();
                next_buffer = Some((*buffer_id, view_id));
                break;
            }
        }

        if next_buffer.is_none()
            && let Some((buffer_id, buffer)) = buffers.first_mut()
        {
            next_buffer = Some((*buffer_id, buffer.create_view()));
        }

        if let Some((buffer_id, view_id)) = next_buffer {
            self.load_view_data(buffer_id, view_id);
        }

        next_buffer.unwrap_or_else(|| {
            let mut buffer = Buffer::new();
            let view_id = buffer.create_view();
            (self.workspace.buffers.insert(buffer), view_id)
        })
    }

    pub fn close_pane(&mut self) {
        if self.workspace.panes.num_panes() > 1 {
            match self.workspace.panes.get_current_pane() {
                PaneKind::Buffer(buffer_id, view_id) => {
                    self.workspace.buffers[buffer_id].remove_view(view_id);
                    self.workspace
                        .panes
                        .remove_pane(PaneKind::Buffer(buffer_id, view_id));
                    if self.workspace.buffers[buffer_id].is_disposable() {
                        self.workspace.buffers.remove(buffer_id);
                    }
                }
                PaneKind::FileExplorer(file_explorer_id) => {
                    self.workspace.file_explorers.remove(file_explorer_id);
                    self.workspace
                        .panes
                        .remove_pane(PaneKind::FileExplorer(file_explorer_id));
                }
                PaneKind::Logger => {
                    self.workspace.panes.remove_pane(PaneKind::Logger);
                }
            }
        }
    }

    pub fn force_close_current_buffer(&mut self) {
        if let Some((buffer_id, _)) = self.get_current_buffer_id() {
            if let Some(path) = self.workspace.buffers[buffer_id].file() {
                self.insert_removed_buffer(path.to_path_buf());
            }
            let buffer = self.workspace.buffers.remove(buffer_id).unwrap();

            let (new_buffer_id, new_view_id) = self.get_next_buffer();
            self.workspace
                .panes
                .replace_current(PaneKind::Buffer(new_buffer_id, new_view_id));

            for (view_id, _) in buffer.views {
                self.workspace.panes.replace(
                    PaneKind::Buffer(buffer_id, view_id),
                    PaneKind::Buffer(
                        new_buffer_id,
                        self.workspace.buffers[new_buffer_id].create_view(),
                    ),
                );
            }
        } else {
            let (buffer_id, view_id) = self.get_next_buffer();
            self.workspace
                .panes
                .replace_current(PaneKind::Buffer(buffer_id, view_id));
        }
    }

    pub fn reopen_last_closed_buffer(&mut self) {
        while let Some(path) = self.closed_buffers.pop() {
            if let Some((buffer, _)) = self.get_current_buffer()
                && buffer.file() == Some(&path)
            {
                continue;
            }
            self.open_file(path, false);
            break;
        }
    }

    fn insert_removed_buffer(&mut self, new: PathBuf) {
        self.closed_buffers.retain(|path| &new != path);
        self.closed_buffers.push(new);
    }

    pub fn get_search_prompt(&self, global: bool) -> String {
        let mut prompt = if global {
            String::from("global-search")
        } else {
            String::from("search")
        };
        if self.config.editor.case_insensitive_search {
            prompt += " (i): ";
        } else {
            prompt += ": ";
        }
        prompt
    }

    pub fn get_current_buffer_id(&self) -> Option<(BufferId, ViewId)> {
        match self.workspace.panes.get_current_pane() {
            PaneKind::Buffer(buffer_id, view_id) => Some((buffer_id, view_id)),
            _ => None,
        }
    }

    pub fn get_current_buffer(&self) -> Option<(&Buffer, ViewId)> {
        let PaneKind::Buffer(buffer, view_id) = self.workspace.panes.get_current_pane() else {
            return None;
        };

        Some((self.workspace.buffers.get(buffer)?, view_id))
    }

    pub fn get_current_buffer_mut(&mut self) -> Option<(&mut Buffer, ViewId)> {
        let PaneKind::Buffer(buffer, view_id) = self.workspace.panes.get_current_pane() else {
            return None;
        };

        Some((self.workspace.buffers.get_mut(buffer)?, view_id))
    }

    pub fn insert_buffer(
        &mut self,
        buffer: Buffer,
        view_id: ViewId,
        make_current: bool,
    ) -> (BufferId, &mut Buffer) {
        let buffer_id = self.workspace.buffers.insert(buffer);
        if make_current {
            if let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane() {
                self.workspace.buffers[buffer_id].remove_view(view_id);
            }
            let old = self
                .workspace
                .panes
                .replace_current(PaneKind::Buffer(buffer_id, view_id));

            if let PaneKind::Buffer(id, view_id) = old {
                let buffer = &mut self.workspace.buffers[id];
                buffer.remove_view(view_id);
                if buffer.is_disposable() {
                    self.workspace.buffers.remove(id);
                }
            }
        }
        (buffer_id, &mut self.workspace.buffers[buffer_id])
    }

    pub fn save_buffer(&mut self, buffer_id: BufferId, path: Option<PathBuf>) {
        let buffer = &mut self.workspace.buffers[buffer_id];

        if let Some(path) = path
            && let Err(err) = buffer.set_file(Some(path))
        {
            self.palette.set_msg(err);
            return;
        }

        let Some(path) = buffer.file().map(|p| p.to_owned()) else {
            self.palette.set_msg(buffer::error::BufferError::NoPathSet);
            return;
        };

        let config = self.config.languages.from_name(buffer.language_name());
        let fmt = config.and_then(|config| config.format.clone());
        let auto_trim = config
            .and_then(|language| language.auto_trim_whitespace)
            .unwrap_or(self.config.editor.auto_trim_whitespace);
        let auto_format = config
            .and_then(|language| language.auto_format)
            .unwrap_or(self.config.editor.auto_format);

        if auto_trim {
            buffer.trim_trailing_whitespace();
        }

        if auto_format && let Some(fmt) = fmt {
            let _ = buffer.format(None, &fmt);
        }

        if self
            .save_jobs
            .iter()
            .any(|(job_buffer_id, _)| *job_buffer_id == buffer_id)
        {
            tracing::warn!("Buffer already being saved");
            return;
        }

        let job = self.job_manager.spawn_foreground_job(
            move |_, _, (buffer_id, encoding, line_ending, rope, path, last_edit)| {
                let written = buffer::write::write(encoding, line_ending, rope.clone(), &path)?;
                Ok(SaveBufferJob {
                    buffer_id,
                    path,
                    last_edit,
                    written,
                })
            },
            (
                buffer_id,
                buffer.encoding,
                buffer.line_ending,
                buffer.rope().clone(),
                path.to_path_buf(),
                buffer.get_last_edit(),
            ),
        );

        self.save_jobs.push((buffer_id, job));
    }

    pub fn get_current_keymappings(&self) -> &[Keymapping] {
        if let Some(name) = &self.chord {
            self.config
                .keymap
                .input_modes
                .get(name)
                .unwrap_or(&self.config.keymap.normal)
        } else {
            &self.config.keymap.normal
        }
    }

    pub fn run_shell_command(
        &mut self,
        cmd: String,
        name: Option<String>,
        pipe: bool,
        read_only: bool,
    ) {
        let buffer_id = if pipe {
            let mut buffer = Buffer::new();
            let view_id = buffer.create_view();
            match name {
                Some(name) => buffer.set_name(name),
                None => buffer.set_name(cmd.clone()),
            }
            buffer.read_only = read_only;
            Some(self.insert_buffer(buffer, view_id, true).0)
        } else {
            None
        };

        let job = self.job_manager.spawn_foreground_job(
            move |killed, progressor, ()| -> Result<_, anyhow::Error> {
                let mut command = get_exec(&cmd);
                command.stdout(Stdio::piped());
                command.stderr(Stdio::piped());
                command.stdin(Stdio::null());
                #[cfg(target_os = "linux")]
                {
                    std::os::unix::process::CommandExt::process_group(&mut command, 0);
                    unsafe {
                        std::os::unix::process::CommandExt::pre_exec(&mut command, || {
                            Ok(rustix::stdio::dup2_stderr(rustix::stdio::stdout())?)
                        })
                    };
                }
                let mut child = command.spawn()?;
                let mut stdout = child.stdout.take().unwrap();
                // This is a lazy life time extension.
                // It is safe because we always join the thread later in the function
                let progressor: &'static mut Progressor<_> =
                    unsafe { std::mem::transmute::<_, _>(progressor) };
                let reader_thread = std::thread::spawn(move || {
                    let mut rope = Rope::new();
                    let mut buffer = Vec::new();
                    let mut bytes = [0u8; 4096];
                    let mut dirty = false;
                    loop {
                        if let Ok(read_bytes) = stdout.read(&mut bytes) {
                            if read_bytes == 0 {
                                break;
                            }

                            buffer.extend_from_slice(&bytes[..read_bytes]);
                            let mut slice = &buffer[..];
                            let mut total = 0;
                            while let Some(idx) = memchr::memchr(b'\n', slice) {
                                let len = idx + 1;
                                let line = String::from_utf8_lossy(&slice[..len]);
                                let rope_line = Rope::from_str(&line);
                                rope.append(rope_line);
                                slice = &slice[len..];
                                total += len;
                                dirty = true;
                            }
                            buffer.drain(..total);
                        }
                        if let (Some(buffer_id), true) = (buffer_id, dirty) {
                            progressor.make_progress((buffer_id, rope.clone()));
                            dirty = false;
                        }
                    }
                    rope
                });
                let status = loop {
                    match child.try_wait() {
                        Ok(None) => {
                            if killed.load(Ordering::Relaxed) {
                                #[cfg(not(target_os = "linux"))]
                                if let Err(err) = child.kill() {
                                    tracing::error!("Error killing child: {err}");
                                }
                                #[cfg(target_os = "linux")]
                                {
                                    if let Err(err) = rustix::process::kill_process_group(
                                        rustix::process::Pid::from_raw(child.id() as i32).unwrap(),
                                        rustix::process::Signal::Term,
                                    ) {
                                        tracing::error!("Error killing child: {err}");
                                    }
                                }
                            }
                            std::thread::sleep(Duration::from_millis(20));
                            continue;
                        }
                        Ok(Some(s)) => {
                            break s;
                        }
                        Err(err) => tracing::error!("error: {err}"),
                    }
                };

                let rope = reader_thread.join().unwrap();

                if !status.success() && !pipe {
                    return Err(anyhow::Error::msg(rope.to_string()));
                }

                Ok((buffer_id, rope))
            },
            (),
        );
        self.shell_jobs.push((buffer_id, job));
    }

    pub fn search(&mut self) {
        match self.workspace.panes.get_current_pane() {
            PaneKind::Buffer(buffer_id, view_id) => {
                let buffer = &mut self.workspace.buffers[buffer_id];
                let selection = buffer.get_selection(view_id, 0);
                let current_query = buffer.views[view_id]
                    .searcher
                    .as_ref()
                    .map(|searcher| searcher.get_last_query());
                self.palette.focus(
                    self.get_search_prompt(false),
                    PaletteMode::Search,
                    CompleterContext::new(
                        self.themes.keys().cloned().collect(),
                        self.get_action_names(),
                        false,
                        None,
                    ),
                );
                if !selection.is_empty() {
                    self.palette.set_line(selection);
                } else if let Some(current_query) = current_query {
                    self.palette.set_line(current_query);
                }
                self.hide_pickers();
            }
            PaneKind::FileExplorer(_) => {
                self.palette.focus(
                    self.get_search_prompt(false),
                    PaletteMode::Search,
                    CompleterContext::new(
                        self.themes.keys().cloned().collect(),
                        self.get_action_names(),
                        false,
                        None,
                    ),
                );
                self.hide_pickers();
            }
            PaneKind::Logger => (),
        }
    }

    pub fn global_search(&mut self) {
        self.hide_pickers();
        let selection = self
            .get_current_buffer()
            .map(|(buffer, view_id)| buffer.get_selection(view_id, 0))
            .unwrap_or_default();
        self.palette.focus(
            self.get_search_prompt(true),
            PaletteMode::GlobalSearch,
            CompleterContext::new(
                self.themes.keys().cloned().collect(),
                self.get_action_names(),
                false,
                None,
            ),
        );
        if !selection.is_empty()
            && self.palette.mode() == Some(&PaletteMode::GlobalSearch)
            && self.palette.get_line().is_some()
        {
            self.palette.set_line(selection);
        }
    }

    pub fn start_replace(&mut self) {
        let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane() else {
            return;
        };
        let buffer = &mut self.workspace.buffers[buffer_id];
        if buffer.views[view_id].searcher.is_some() {
            self.palette.focus(
                "replace: ",
                PaletteMode::Replace,
                CompleterContext::new(
                    self.themes.keys().cloned().collect(),
                    self.get_action_names(),
                    false,
                    None,
                ),
            );
        }
    }

    fn try_get_current_buffer_path(&self) -> Option<PathBuf> {
        self.get_current_buffer()?.0.file().map(|p| p.to_owned())
    }

    pub fn rename_file(&mut self, current: impl AsRef<Path>, new: impl AsRef<Path>) -> Result<()> {
        let current = current.as_ref();
        let new = new.as_ref();
        for buffer in self.workspace.buffers.values_mut() {
            if buffer.file() == Some(current) {
                buffer.set_file(Some(new))?;
            }
        }
        std::fs::rename(current, new)?;
        for file_explorer in self.workspace.file_explorers.values_mut() {
            file_explorer.reload();
        }
        Ok(())
    }

    pub fn hide_pickers(&mut self) {
        self.file_picker = None;
        self.buffer_picker = None;
        self.global_search_picker = None;
    }

    pub fn get_input_ctx(&self) -> InputContext {
        if self.palette.has_focus()
            || self.file_picker.is_some()
            || self.buffer_picker.is_some()
            || self.global_search_picker.is_some()
        {
            return InputContext::Edit;
        }
        match self.workspace.panes.get_current_pane() {
            PaneKind::Buffer(..) => InputContext::Edit,
            PaneKind::Logger => InputContext::Edit,
            PaneKind::FileExplorer(_) => InputContext::FileExplorer,
        }
    }

    pub fn get_action_names(&self) -> Vec<String> {
        let mut actions: Vec<_> = self.workspace.config.actions.keys().cloned().collect();
        for action in self.config.editor.actions.keys().cloned() {
            if !actions.contains(&action) {
                actions.push(action);
            }
        }
        actions
    }
}

fn get_exec(cmd: &str) -> Command {
    #[cfg(unix)]
    pub const SHELL: [&str; 2] = ["sh", "-c"];

    #[cfg(windows)]
    pub const SHELL: [&str; 2] = ["cmd.exe", "/c"];
    if cfg!(unix) {
        match std::env::var("SHELL") {
            Ok(shell) => {
                let mut command = Command::new(shell);
                command.arg("-c").arg(cmd);
                command
            }
            Err(err) => {
                tracing::error!("{err}");
                let mut command = Command::new(SHELL[0]);
                command.arg(SHELL[1]).arg(cmd);
                command
            }
        }
    } else {
        let mut command = Command::new(SHELL[0]);
        command.arg(SHELL[1]).arg(cmd);
        command
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Err(e) = self.workspace.save_workspace() {
            tracing::error!("Error saving workspace: {e}");
        };
        for job in &mut self.shell_jobs {
            job.1.kill();
        }
        clipboard::uninit();
    }
}
