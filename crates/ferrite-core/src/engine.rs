use std::{
    collections::HashMap,
    env, io,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

use anyhow::Result;
use ferrite_cli::Args;
use ferrite_utility::{line_ending, point::Point, trim::trim_path};
use linkify::{LinkFinder, LinkKind};
use slotmap::{Key as _, SlotMap};
use subprocess::{Exec, Redirection};

use crate::{
    buffer::{self, encoding::get_encoding, Buffer, ViewId},
    buffer_watcher::BufferWatcher,
    byte_size::format_byte_size,
    clipboard,
    cmd::Cmd,
    config::{
        editor::Editor,
        keymap::{Keymap, Keymapping},
        languages::Languages,
        Config,
    },
    event_loop_proxy::{EventLoopControlFlow, EventLoopProxy, UserEvent},
    git::branch::BranchWatcher,
    indent::Indentation,
    job_manager::{JobHandle, JobManager},
    jobs::SaveBufferJob,
    layout::panes::{PaneKind, Panes, Rect},
    logger::{LogMessage, LoggerState},
    palette::{cmd_parser, completer::CompleterContext, CommandPalette, PalettePromptEvent},
    picker::{
        buffer_picker::{BufferFindProvider, BufferItem},
        file_picker::FileFindProvider,
        file_previewer::FilePreviewer,
        file_scanner::FileScanner,
        global_search_picker::{GlobalSearchMatch, GlobalSearchPreviewer, GlobalSearchProvider},
        Picker,
    },
    spinner::Spinner,
    theme::EditorTheme,
    watcher::FileWatcher,
    workspace::{BufferData, BufferId, Workspace},
};

pub struct Engine {
    pub workspace: Workspace,
    pub themes: HashMap<String, EditorTheme>,
    pub config: Config,
    pub palette: CommandPalette,
    pub file_picker: Option<Picker<String>>,
    pub buffer_picker: Option<Picker<BufferItem>>,
    pub global_search_picker: Option<Picker<GlobalSearchMatch>>,
    pub branch_watcher: BranchWatcher,
    pub proxy: Box<dyn EventLoopProxy>,
    pub file_scanner: FileScanner,
    pub job_manager: JobManager,
    pub save_jobs: Vec<JobHandle<Result<SaveBufferJob>>>,
    pub shell_jobs: Vec<JobHandle<Result<(bool, Buffer), anyhow::Error>>>,
    pub spinner: Spinner,
    pub logger_state: LoggerState,
    pub choord: Option<String>,
    pub repeat: Option<String>,
    pub last_render_time: Duration,
    pub start_of_events: Instant,
    pub closed_buffers: Vec<PathBuf>,
    pub buffer_watcher: Option<BufferWatcher>,
    pub buffer_area: Rect,
    pub force_redraw: bool,
}

impl Engine {
    pub fn new(
        args: &Args,
        proxy: Box<dyn EventLoopProxy>,
        recv: mpsc::Receiver<LogMessage>,
    ) -> Result<Self> {
        buffer::set_buffer_proxy(proxy.dup());
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

        let keymap_path = Keymap::get_default_location().ok();
        let keymap = match Keymap::load_from_default_location() {
            Ok(languages) => languages,
            Err(err) => {
                palette.set_error(err);
                Keymap::default()
            }
        };

        let mut keymap_watcher = None;
        if let Some(ref keymap_path) = keymap_path {
            match FileWatcher::new(keymap_path, proxy.dup()) {
                Ok(watcher) => keymap_watcher = Some(watcher),
                Err(err) => tracing::error!("Error starting keymap config watcher: {err}"),
            }
        }

        if config.local_clipboard {
            clipboard::set_local_clipboard(true);
        }

        let themes = EditorTheme::load_themes();
        if !themes.contains_key(&config.theme) {
            config.theme = "default".into();
        }

        let mut buffers: SlotMap<BufferId, _> = SlotMap::with_key();
        let mut current_buffer_id = BufferId::null();

        for (i, file) in args.files.iter().enumerate() {
            if i == 0 && file.is_dir() {
                continue;
            }

            let buffer = match Buffer::from_file(file) {
                Ok(buffer) => buffer,
                Err(err) => match err.kind() {
                    io::ErrorKind::NotFound => match Buffer::with_path(file) {
                        Ok(buffer) => buffer,
                        Err(err) => {
                            palette.set_error(err);
                            continue;
                        }
                    },
                    _ => {
                        palette.set_error(err);
                        continue;
                    }
                },
            };
            current_buffer_id = buffers.insert(buffer);
        }

        for (_, buffer) in &mut buffers {
            if let Some(language) = &args.language {
                if let Err(err) = buffer.set_langauge(language, proxy.dup()) {
                    palette.set_error(err);
                }
            }
        }

        let mut file_daemon = None;
        let mut file_finder = None;

        if let Some(path) = args.files.first() {
            if path.is_dir() {
                std::env::set_current_dir(path)?;
                let daemon = FileScanner::new(std::env::current_dir()?, &config);
                file_finder = Some(Picker::new(
                    FileFindProvider(daemon.subscribe()),
                    Some(Box::new(FilePreviewer::new(proxy.dup()))),
                    proxy.dup(),
                    None,
                ));
                file_daemon = Some(daemon);
            }
        }

        let file_daemon = if let Some(daemon) = file_daemon {
            daemon
        } else {
            FileScanner::new(std::env::current_dir()?, &config)
        };

        let job_manager = JobManager::new(proxy.dup());

        let mut workspace = match Workspace::load_workspace(buffers.is_empty()) {
            Ok(workspace) => workspace,
            Err(err) => {
                tracing::error!("Error loading workspace: {err}");
                Workspace::default()
            }
        };

        if !buffers.is_empty() {
            workspace.buffers = buffers;
            let buffer = &mut workspace.buffers[current_buffer_id];
            let view_id = buffer.create_view();
            buffer.goto(view_id, args.line as i64);
            workspace.panes = Panes::new(current_buffer_id, view_id);
        }

        let branch_watcher = BranchWatcher::new(proxy.dup())?;

        let buffer_watcher = if config.watch_open_files {
            BufferWatcher::new(proxy.dup()).ok()
        } else {
            None
        };

        let config = Config {
            editor: config,
            editor_path: config_path,
            editor_watcher: config_watcher,
            languages,
            languages_path,
            languages_watcher,
            keymap,
            keymap_path,
            keymap_watcher,
        };

        Ok(Self {
            workspace,
            themes,
            config,
            palette,
            file_picker: file_finder,
            buffer_picker: None,
            global_search_picker: None,
            branch_watcher,
            proxy,
            file_scanner: file_daemon,
            job_manager,
            save_jobs: Default::default(),
            shell_jobs: Default::default(),
            spinner: Default::default(),
            choord: None,
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
        })
    }

    pub fn do_polling(&mut self, control_flow: &mut EventLoopControlFlow) {
        self.logger_state.update();

        if !self.config.editor.watch_open_files {
            self.buffer_watcher = None;
        } else if let Some(buffer_watcher) = &mut self.buffer_watcher {
            buffer_watcher.update(&mut self.workspace.buffers);
        } else {
            self.buffer_watcher = BufferWatcher::new(self.proxy.dup()).ok();
        }

        if let Some(config_watcher) = &mut self.config.editor_watcher {
            if let Some(result) = config_watcher.poll_update() {
                match result {
                    Ok(editor) => {
                        self.config.editor = editor;
                        if !self.themes.contains_key(&self.config.editor.theme) {
                            self.config.editor.theme = "default".into();
                        }
                        self.palette.set_msg("Reloaded editor config");
                    }
                    Err(err) => self.palette.set_error(err),
                }
            }
        }

        if let Some(config_watcher) = &mut self.config.languages_watcher {
            if let Some(result) = config_watcher.poll_update() {
                match result {
                    Ok(languages) => {
                        self.config.languages = languages;
                        self.palette.set_msg("Reloaded languages");
                    }
                    Err(err) => self.palette.set_error(err),
                }
            }
        }

        if let Some(config_watcher) = &mut self.config.keymap_watcher {
            if let Some(result) = config_watcher.poll_update() {
                match result {
                    Ok(keymap) => {
                        self.config.keymap = keymap;
                        self.palette.set_msg("Reloaded keymap");
                    }
                    Err(err) => self.palette.set_error(err),
                }
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
                        if let Some(view_id) = buffer.get_first_view() {
                            buffer_data.cursor = buffer.cursor(view_id);
                            buffer_data.line_pos = buffer.line_pos(view_id);
                            buffer_data.col_pos = buffer.col_pos(view_id);
                            buffer_data.indent = buffer.indent;
                            if buffer.language_name() != buffer_data.language {
                                buffer_data.language = buffer.language_name().into();
                            }
                        }
                    }
                    None => {
                        if let Some(view_id) = buffer.get_first_view() {
                            new_buffers.push(BufferData {
                                path: path.to_path_buf(),
                                cursor: buffer.cursor(view_id),
                                line_pos: buffer.line_pos(view_id),
                                col_pos: buffer.col_pos(view_id),
                                indent: buffer.indent,
                                language: buffer.language_name().into(),
                            });
                        }
                    }
                }
            }
        }
        self.workspace
            .buffer_extra_data
            .extend_from_slice(&new_buffers);

        for job in &mut self.save_jobs {
            if let Ok(result) = job.recv_try() {
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
        self.save_jobs.retain(|job| !job.is_finished());

        let mut new_buffers = Vec::new();
        {
            for job in &mut self.shell_jobs {
                if let Ok(result) = job.recv_try() {
                    match result {
                        Ok((pipe, buffer)) => {
                            if pipe {
                                new_buffers.push(buffer);
                            } else {
                                self.palette.set_msg(buffer);
                            }
                        }
                        Err(e) => self.palette.set_error(e),
                    }
                }
            }
        }
        self.shell_jobs.retain(|job| !job.is_finished());

        for mut buffer in new_buffers {
            let view_id = buffer.create_view();
            self.insert_buffer(buffer, view_id, true);
        }

        self.job_manager.poll_jobs();

        let duration = self
            .spinner
            .update(!self.save_jobs.is_empty() || !self.shell_jobs.is_empty());
        *control_flow = EventLoopControlFlow::WaitMax(duration);
    }

    pub fn handle_input_command(&mut self, input: Cmd, control_flow: &mut EventLoopControlFlow) {
        if let Some(repeat) = &mut self.repeat {
            match input {
                Cmd::Char(ch) if ch.is_ascii_digit() => {
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
            self.choord = None;
        }
        match input {
            Cmd::ForceRedraw => self.force_redraw = true,
            Cmd::RotateFile => {
                if let Some((buffer, _)) = self.get_current_buffer() {
                    match buffer.get_next_file() {
                        Ok(file) => {
                            self.open_file(file);
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
                self.file_picker = None;
                self.buffer_picker = None;
                self.global_search_picker = None;
                self.palette
                    .focus("$ ", "shell", CompleterContext::new(&self.themes));
            }
            Cmd::InputMode { name } => {
                if name == "normal" {
                    self.choord = None;
                } else {
                    self.choord = Some(name);
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
            Cmd::Escape if self.repeat.is_some() => {
                self.repeat = None;
            }
            Cmd::Escape if self.palette.has_focus() => {
                self.palette.reset();
            }
            Cmd::FocusPalette if !self.palette.has_focus() => {
                self.file_picker = None;
                self.buffer_picker = None;
                self.global_search_picker = None;
                self.palette
                    .focus("> ", "command", CompleterContext::new(&self.themes));
            }
            Cmd::PromptGoto => {
                self.file_picker = None;
                self.buffer_picker = None;
                self.global_search_picker = None;
                self.palette
                    .focus("goto: ", "goto", CompleterContext::new(&self.themes));
            }
            Cmd::Search => self.search(),
            Cmd::Replace => self.start_replace(),
            Cmd::GlobalSearch => self.global_search(),
            Cmd::CaseInsensitive => {
                self.config.editor.case_insensitive_search =
                    !self.config.editor.case_insensitive_search;
                if let Some("search") = self.palette.mode() {
                    self.palette.update_prompt(self.get_search_prompt(false));
                }
                if let Some("global-search") = self.palette.mode() {
                    self.palette.update_prompt(self.get_search_prompt(true));
                }
            }
            Cmd::Escape => {
                if self.file_picker.is_some()
                    || self.buffer_picker.is_some()
                    || self.global_search_picker.is_some()
                {
                    self.file_picker = None;
                    self.buffer_picker = None;
                    self.global_search_picker = None;
                }
                if self.choord.is_some() {
                    self.choord = None;
                }
            }
            Cmd::OpenFileBrowser => self.open_file_picker(),
            Cmd::OpenBufferBrowser => self.open_buffer_picker(),
            Cmd::Save => {
                if let PaneKind::Buffer(buffer_id, _) = self.workspace.panes.get_current_pane() {
                    self.save_buffer(buffer_id, None);
                }
            }
            Cmd::FilePickerReload => {
                self.file_scanner = FileScanner::new(
                    env::current_dir().unwrap_or(PathBuf::from(".")),
                    &self.config.editor,
                );
            }
            Cmd::ReplaceAll(replacement) => {
                if let Some((buffer, view_id)) = self.get_current_buffer_mut() {
                    buffer.replace_all(view_id, replacement);
                }
            }
            Cmd::SortLines(asc) => {
                if let Some((buffer, view_id)) = self.get_current_buffer_mut() {
                    buffer.sort_lines(view_id, asc);
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
                    env!("CARGO_PKG_VERSION"),
                    env!("GIT_HASH"),
                ));
            }
            Cmd::Pwd => match env::current_dir() {
                Ok(path) => self.palette.set_msg(path.to_string_lossy()),
                Err(err) => self.palette.set_error(err),
            },
            Cmd::Cd(path) => {
                if let Err(err) = self.workspace.save_workspace() {
                    self.palette.set_error(err);
                } else {
                    match env::set_current_dir(&path) {
                        Ok(_) => {
                            self.buffer_picker = None;
                            self.file_picker = None;

                            self.file_scanner = FileScanner::new(
                                env::current_dir().unwrap_or(PathBuf::from(".")),
                                &self.config.editor,
                            );

                            match BranchWatcher::new(self.proxy.dup()) {
                                Ok(branch_watcher) => self.branch_watcher = branch_watcher,
                                Err(err) => {
                                    let msg = format!("Error creating branch watcher: {err}");
                                    tracing::error!(msg);
                                    self.palette.set_error(msg);
                                }
                            }

                            self.workspace = match Workspace::load_workspace(true) {
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
            }
            Cmd::Split(direction) => {
                let mut buffer = Buffer::new();
                let view_id = buffer.create_view();
                let (buffer_id, _) = self.insert_buffer(buffer, view_id, false);
                self.workspace
                    .panes
                    .split(PaneKind::Buffer(buffer_id, view_id), direction);
            }
            Cmd::RunShellCmd { args, pipe } => {
                self.run_shell_command(args, pipe, false);
            }
            Cmd::Trash => {
                let PaneKind::Buffer(buffer_id, _) = self.workspace.panes.get_current_pane() else {
                    return;
                };

                match self.workspace.buffers[buffer_id].move_to_trash() {
                    Ok(true) => {
                        let path = self.workspace.buffers[buffer_id].file().unwrap();
                        match trash::delete(path) {
                            Ok(_) => {
                                self.palette.set_msg(format!(
                                    "`{}` moved to trash",
                                    path.to_string_lossy()
                                ));
                            }
                            Err(err) => self.palette.set_error(err),
                        }
                    }
                    Ok(false) => {
                        self.palette
                            .set_error("No path set for file, cannot move to trash");
                    }
                    Err(e) => {
                        self.palette.set_error(e);
                    }
                }
            }
            Cmd::FormatSelection => self.format_selection_current_buffer(),
            Cmd::Format => self.format_current_buffer(),
            Cmd::OpenFile(path) => {
                self.open_file(path);
            }
            Cmd::SaveFile(path) => {
                let PaneKind::Buffer(buffer_id, _) = self.workspace.panes.get_current_pane() else {
                    return;
                };

                self.save_buffer(buffer_id, path);
            }
            Cmd::Language(language) => {
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
            Cmd::Encoding(encoding) => {
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
            Cmd::Indent(indent) => {
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
            Cmd::LineEnding(line_ending) => {
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
            Cmd::New(path) => {
                if let Some(path) = path {
                    match Buffer::with_path(path) {
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

                    if buffer.file().is_some() {
                        if let Err(err) = buffer.reload() {
                            self.palette.set_error(err);
                        }
                    }
                }
            }
            Cmd::Goto(line) => {
                let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane()
                else {
                    return;
                };
                self.workspace.buffers[buffer_id].goto(view_id, line);
            }
            Cmd::Case(case) => {
                let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane()
                else {
                    return;
                };
                self.workspace.buffers[buffer_id].transform_case(view_id, case);
            }
            Cmd::ForceQuit => *control_flow = EventLoopControlFlow::Exit,
            Cmd::Logger => {
                self.logger_state.lines_scrolled_up = 0;
                self.workspace.panes.replace_current(PaneKind::Logger);
            }
            Cmd::Theme(name) => match name {
                Some(name) => {
                    if self.themes.contains_key(&name) {
                        self.config.editor.theme = name;
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
                    self.file_scanner = FileScanner::new(
                        env::current_dir().unwrap_or(PathBuf::from(".")),
                        &self.config.editor,
                    );
                }
                self.open_file_picker();
            }
            Cmd::OpenConfig => self.open_config(),
            Cmd::DefaultConfig => self.open_default_config(),
            Cmd::OpenLanguages => self.open_languages(),
            Cmd::DefaultLanguages => self.open_default_languages(),
            Cmd::OpenKeymap => self.open_keymap(),
            Cmd::DefaultKeymap => self.open_default_keymap(),
            Cmd::ForceClose => self.force_close_current_buffer(),
            Cmd::Close => self.close_current_buffer(),
            Cmd::ClosePane => self.close_pane(),
            Cmd::Paste => {
                let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane()
                else {
                    return;
                };
                if let Err(err) =
                    self.workspace.buffers[buffer_id].handle_input(view_id, Cmd::Paste)
                {
                    self.palette.set_error(err);
                }
            }
            Cmd::Copy => {
                let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane()
                else {
                    return;
                };
                if let Err(err) = self.workspace.buffers[buffer_id].handle_input(view_id, Cmd::Copy)
                {
                    self.palette.set_error(err);
                }
            }
            Cmd::RevertBuffer => {
                let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane()
                else {
                    return;
                };
                let _ = self.workspace.buffers[buffer_id].handle_input(view_id, Cmd::RevertBuffer);
            }
            Cmd::GitReload => self.branch_watcher.force_reload(),
            Cmd::GitDiff => {
                self.run_shell_command(vec!["git".into(), "diff".into()], true, true);
            }
            input => {
                if self.palette.has_focus() {
                    let _ = self
                        .palette
                        .handle_input(input, CompleterContext::new(&self.themes));
                } else if let Some(picker) = &mut self.file_picker {
                    let _ = picker.handle_input(input);
                    if let Some(path) = picker.get_choice() {
                        self.file_picker = None;
                        self.open_file(path);
                    }
                } else if let Some(picker) = &mut self.buffer_picker {
                    let _ = picker.handle_input(input);
                    if let Some(choice) = picker.get_choice() {
                        self.workspace.buffers[choice.id].update_interact();
                        self.buffer_picker = None;
                        let old = self.workspace.panes.replace_current(PaneKind::Buffer(
                            choice.id,
                            self.workspace.buffers[choice.id].create_view(),
                        ));
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
                        if let Some(file) = guard.file() {
                            if self.open_file(file) {
                                let view_id = guard.get_first_view().unwrap();
                                let cursor_line = guard.cursor_line_idx(view_id);
                                let cursor_col = guard.cursor_grapheme_column(view_id);
                                let anchor_line = guard.anchor_line_idx(view_id);
                                let anchor_col = guard.anchor_grapheme_column(view_id);
                                if let Some((buffer, view_id)) = self.get_current_buffer_mut() {
                                    buffer.select_area(
                                        view_id,
                                        Point::new(cursor_col, cursor_line),
                                        Point::new(anchor_col, anchor_line),
                                        false,
                                    );
                                    // A buffers default amount of lines when newly opened is too large
                                    // and the view will not jump to it.
                                    buffer.set_view_lines(view_id, 10);
                                    buffer.center_on_cursor(view_id);
                                }
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
                        PaneKind::Logger => self.logger_state.handle_input(input),
                    }
                }
            }
        }
    }

    pub fn handle_app_event(&mut self, event: UserEvent, control_flow: &mut EventLoopControlFlow) {
        match event {
            UserEvent::Wake => (),
            UserEvent::PaletteEvent { mode, content } => match mode.as_str() {
                "command" => match cmd_parser::parse_cmd(&content) {
                    Ok(cmd) => {
                        self.palette.reset();
                        self.handle_single_input_command(cmd, control_flow);
                    }
                    Err(err) => self.palette.set_error(err),
                },
                "goto" => {
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
                "search" => {
                    let PaneKind::Buffer(buffer_id, view_id) =
                        self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    self.workspace.buffers[buffer_id].start_search(
                        view_id,
                        self.proxy.dup(),
                        content,
                        self.config.editor.case_insensitive_search,
                    );
                    self.palette.unfocus();
                }
                "replace" => {
                    self.palette.unfocus();
                    let PaneKind::Buffer(buffer_id, view_id) =
                        self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    let buffer = &mut self.workspace.buffers[buffer_id];
                    buffer.views[view_id].replacement = Some(content);
                }
                "global-search" => {
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
                "shell" => {
                    let args: Vec<_> = content.split_whitespace().map(PathBuf::from).collect();
                    self.run_shell_command(args, false, false);
                }
                _ => (),
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
        let config = self
            .config
            .languages
            .languages
            .iter()
            .find(|lang| lang.name == buffer_lang);
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

    pub fn format_current_buffer(&mut self) {
        if let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane() {
            let buffer_lang = self.workspace.buffers[buffer_id].language_name();
            let config = self
                .config
                .languages
                .languages
                .iter()
                .find(|lang| lang.name == buffer_lang);
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

            if let Err(err) = self.workspace.buffers[buffer_id].format(view_id, fmt) {
                self.palette.set_error(err);
            }
        }
    }

    pub fn open_file(&mut self, path: impl AsRef<Path>) -> bool {
        let real_path = match dunce::canonicalize(&path) {
            Ok(path) => path,
            Err(err) => {
                self.palette.set_error(err);
                return false;
            }
        };

        match self.workspace.buffers.iter_mut().find(|(_, buffer)| {
            buffer
                .file()
                .and_then(|path| dunce::canonicalize(path).ok())
                .as_deref()
                == Some(&real_path)
        }) {
            Some((id, buffer)) => {
                buffer.update_interact();
                let view_id = buffer.create_view();
                let replaced = self
                    .workspace
                    .panes
                    .replace_current(PaneKind::Buffer(id, view_id));
                if let PaneKind::Buffer(buffer_id, view_id) = replaced {
                    self.workspace.buffers[buffer_id].remove_view(view_id);
                }
                true
            }
            None => match Buffer::from_file(&real_path) {
                Ok(mut buffer) => {
                    let view_id = buffer.create_view();
                    if let Some(buffer_data) = self
                        .workspace
                        .buffer_extra_data
                        .iter()
                        .find(|b| b.path == real_path)
                    {
                        buffer.load_view_data(view_id, buffer_data);
                        buffer.load_buffer_data(buffer_data);
                    }

                    if let PaneKind::Buffer(buffer_id, _) = self.workspace.panes.get_current_pane()
                    {
                        let current_buf = self.workspace.buffers.get_mut(buffer_id).unwrap();
                        if current_buf.is_disposable() {
                            *current_buf = buffer;
                            return true;
                        }
                    }
                    self.insert_buffer(buffer, view_id, true);
                    true
                }
                Err(err) => {
                    self.palette.set_error(err);
                    false
                }
            },
        }
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
        self.palette.reset();
        self.file_picker = None;
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
        self.file_scanner = FileScanner::new(
            env::current_dir().unwrap_or(PathBuf::from(".")),
            &self.config.editor,
        );
        self.file_picker = Some(Picker::new(
            FileFindProvider(self.file_scanner.subscribe()),
            Some(Box::new(FilePreviewer::new(self.proxy.dup()))),
            self.proxy.dup(),
            self.try_get_current_buffer_path(),
        ));
    }

    pub fn open_config(&mut self) {
        match &self.config.editor_path {
            Some(path) => {
                self.open_file(path.clone());
            }
            None => self.palette.set_error("Could not locate the config file"),
        }
    }

    pub fn open_default_config(&mut self) {
        let mut buffer = Buffer::with_name("default_config.toml");
        buffer.set_text(Editor::DEFAULT);
        let view_id = buffer.create_view();
        self.insert_buffer(buffer, view_id, true);
    }

    pub fn open_languages(&mut self) {
        match &self.config.languages_path {
            Some(path) => {
                self.open_file(path.clone());
            }
            None => self
                .palette
                .set_error("Could not locate the languages file"),
        }
    }

    pub fn open_default_languages(&mut self) {
        let mut buffer = Buffer::with_name("default_languages.toml");
        buffer.set_text(Languages::DEFAULT);
        let view_id = buffer.create_view();
        self.insert_buffer(buffer, view_id, true);
    }

    pub fn open_keymap(&mut self) {
        match &self.config.keymap_path {
            Some(path) => {
                self.open_file(path.clone());
            }
            None => self.palette.set_error("Could not locate the keymap file"),
        }
    }

    pub fn open_default_keymap(&mut self) {
        let mut buffer = Buffer::with_name("default_keymap.json");
        buffer.set_text(&serde_json::to_string_pretty(&Keymap::default()).unwrap());
        let view_id = buffer.create_view();
        self.insert_buffer(buffer, view_id, true);
    }

    pub fn close_current_buffer(&mut self) {
        if let Some((buffer, _)) = self.get_current_buffer() {
            if buffer.is_dirty() {
                self.palette.set_prompt(
                    "Current buffer has unsaved changes are you sure you want to close it?",
                    ('y', PalettePromptEvent::CloseCurrent),
                    ('n', PalettePromptEvent::Nop),
                );
            } else {
                self.force_close_current_buffer();
            }
        } else {
            self.force_close_current_buffer();
        }
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

        next_buffer.unwrap_or_else(|| {
            let mut buffer = Buffer::new();
            let view_id = buffer.create_view();
            (self.workspace.buffers.insert(buffer), view_id)
        })
    }

    pub fn close_pane(&mut self) {
        if self.workspace.panes.num_panes() > 1 {
            if let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane() {
                self.workspace.buffers[buffer_id].remove_view(view_id);
            }

            if let Some((buffer_id, view_id)) = self.get_current_buffer_id() {
                self.workspace
                    .panes
                    .remove_pane(PaneKind::Buffer(buffer_id, view_id));
            } else {
                let (buffer_id, view_id) = self.get_next_buffer();
                self.workspace
                    .panes
                    .replace_current(PaneKind::Buffer(buffer_id, view_id));
            }
        }
    }

    pub fn force_close_current_buffer(&mut self) {
        if let Some((buffer_id, _)) = self.get_current_buffer_id() {
            if let Some(path) = self.workspace.buffers[buffer_id].file() {
                self.insert_removed_buffer(path.to_path_buf());
            }
            let buffer = self.workspace.buffers.remove(buffer_id).unwrap();

            {
                let (new_buffer_id, new_view_id) = self.get_next_buffer();
                self.workspace
                    .panes
                    .replace_current(PaneKind::Buffer(new_buffer_id, new_view_id));
            }

            for (view_id, _) in buffer.views {
                let (new_buffer_id, new_view_id) = self.get_next_buffer();
                self.workspace.panes.replace(
                    PaneKind::Buffer(buffer_id, view_id),
                    PaneKind::Buffer(new_buffer_id, new_view_id),
                );
            }
        } else {
            tracing::warn!("REEE");
            let (buffer_id, view_id) = self.get_next_buffer();
            self.workspace
                .panes
                .replace_current(PaneKind::Buffer(buffer_id, view_id));
        }
    }

    pub fn reopen_last_closed_buffer(&mut self) {
        while let Some(path) = self.closed_buffers.pop() {
            if let Some((buffer, _)) = self.get_current_buffer() {
                if buffer.file() == Some(&path) {
                    continue;
                }
            }
            self.open_file(path);
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
            self.workspace
                .panes
                .replace_current(PaneKind::Buffer(buffer_id, view_id));
        }
        (buffer_id, &mut self.workspace.buffers[buffer_id])
    }

    pub fn save_buffer(&mut self, buffer_id: BufferId, path: Option<PathBuf>) {
        let buffer = &mut self.workspace.buffers[buffer_id];

        if let Some(path) = path {
            if let Err(err) = buffer.set_file(path) {
                self.palette.set_msg(err);
                return;
            }
        }

        let Some(path) = buffer.file() else {
            self.palette.set_msg(buffer::error::BufferError::NoPathSet);
            return;
        };

        let job = self.job_manager.spawn_foreground_job(
            move |(buffer_id, encoding, line_ending, rope, path, last_edit)| {
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

        self.save_jobs.push(job);
    }

    pub fn get_current_keymappings(&self) -> &[Keymapping] {
        if let Some(name) = &self.choord {
            self.config
                .keymap
                .input_modes
                .get(name)
                .unwrap_or(&self.config.keymap.normal)
        } else {
            &self.config.keymap.normal
        }
    }

    pub fn run_shell_command(&mut self, args: Vec<PathBuf>, pipe: bool, read_only: bool) {
        let job = self.job_manager.spawn_foreground_job(
            move |()| -> Result<_, anyhow::Error> {
                let mut cmd = String::new();
                for arg in args
                    .into_iter()
                    .map(|path| path.to_string_lossy().to_string())
                {
                    cmd.push_str(&arg);
                    cmd.push(' ');
                }

                let exec = Exec::shell(cmd)
                    .stdout(Redirection::Pipe)
                    .stderr(Redirection::Pipe);

                let mut popen = exec.popen()?;

                let (stdout, stderr) = popen.communicate_bytes(None)?;
                let status = popen.wait()?;

                if !status.success() {
                    return Err(anyhow::Error::msg(
                        String::from_utf8_lossy(&stderr.unwrap()).to_string(),
                    ));
                }

                let mut buffer = Buffer::from_bytes(&stdout.unwrap())?;
                let first_line = buffer.rope().line(0);
                let name = if first_line.len_chars() > 15 {
                    format!("{}...", first_line.slice(..15))
                } else {
                    first_line.to_string()
                };
                buffer.set_name(name);
                buffer.read_only = read_only;

                Ok((pipe, buffer))
            },
            (),
        );
        self.shell_jobs.push(job);
    }

    pub fn open_selected_url(&mut self) {
        if let Some((buffer, view_id)) = self.get_current_buffer() {
            let selection = buffer.get_selection(view_id);
            let mut finder = LinkFinder::new();
            finder.kinds(&[LinkKind::Url]);
            let spans: Vec<_> = finder.spans(&selection).collect();
            if spans.is_empty() {
                if let Err(err) = opener::open(selection) {
                    self.palette.set_error(err);
                }
            } else {
                for span in spans {
                    if let Err(err) = opener::open(span.as_str()) {
                        self.palette.set_error(err);
                    }
                }
            }
        }
    }

    pub fn search(&mut self) {
        if let Some((buffer, view_id)) = self.get_current_buffer() {
            let selection = buffer.get_selection(view_id);
            self.file_picker = None;
            self.buffer_picker = None;
            self.palette.focus(
                self.get_search_prompt(false),
                "search",
                CompleterContext::new(&self.themes),
            );
            if !selection.is_empty() {
                self.palette.set_line(selection);
            }
        }
    }

    pub fn global_search(&mut self) {
        let selection = self
            .get_current_buffer()
            .map(|(buffer, view_id)| buffer.get_selection(view_id))
            .unwrap_or_default();
        self.file_picker = None;
        self.buffer_picker = None;
        self.palette.focus(
            self.get_search_prompt(true),
            "global-search",
            CompleterContext::new(&self.themes),
        );
        if !selection.is_empty() {
            self.palette.set_line(selection);
        }
    }

    pub fn start_replace(&mut self) {
        let PaneKind::Buffer(buffer_id, view_id) = self.workspace.panes.get_current_pane() else {
            return;
        };
        let buffer = &mut self.workspace.buffers[buffer_id];
        if buffer.get_searcher(view_id).is_some() {
            self.palette
                .focus("replace: ", "replace", CompleterContext::new(&self.themes));
        }
    }

    fn try_get_current_buffer_path(&self) -> Option<PathBuf> {
        self.get_current_buffer()?.0.file().map(|p| p.to_owned())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Err(e) = self.workspace.save_workspace() {
            tracing::error!("Error saving workspace: {e}");
        };
    }
}
