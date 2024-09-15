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
use slotmap::{Key, SlotMap};
use subprocess::{Exec, Redirection};

use crate::{
    buffer::{self, encoding::get_encoding, Buffer},
    buffer_watcher::BufferWatcher,
    byte_size::format_byte_size,
    clipboard,
    config::{Config, DEFAULT_CONFIG},
    event_loop_proxy::{EventLoopControlFlow, EventLoopProxy, UserEvent},
    git::branch::BranchWatcher,
    indent::Indentation,
    job_manager::{JobHandle, JobManager},
    jobs::SaveBufferJob,
    keymap::{get_default_choords, get_default_mappings, Exclusiveness, InputCommand, Mapping},
    logger::{LogMessage, LoggerState},
    palette::{cmd, cmd_parser, completer::CompleterContext, CommandPalette, PalettePromptEvent},
    panes::{PaneKind, Panes, Rect},
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
    workspace::{BufferId, Workspace},
};

pub struct Engine {
    pub workspace: Workspace,
    pub themes: HashMap<String, EditorTheme>,
    pub config: Config,
    pub config_path: Option<PathBuf>,
    pub config_watcher: Option<FileWatcher<Config>>,
    pub palette: CommandPalette,
    pub file_picker: Option<Picker<String>>,
    pub buffer_picker: Option<Picker<BufferItem>>,
    pub global_search_picker: Option<Picker<GlobalSearchMatch>>,
    pub key_mappings: HashMap<String, Vec<(Mapping, InputCommand, Exclusiveness)>>,
    pub branch_watcher: BranchWatcher,
    pub proxy: Box<dyn EventLoopProxy>,
    pub file_scanner: FileScanner,
    pub job_manager: JobManager,
    pub save_jobs: Vec<JobHandle<Result<SaveBufferJob>>>,
    pub shell_jobs: Vec<JobHandle<Result<(bool, Buffer), anyhow::Error>>>,
    pub spinner: Spinner,
    pub logger_state: LoggerState,
    pub choord: bool,
    pub repeat: Option<String>,
    pub last_render_time: Duration,
    pub start_of_events: Instant,
    pub closed_buffers: Vec<PathBuf>,
    pub buffer_watcher: Option<BufferWatcher>,
}

impl Engine {
    pub fn new(
        args: &Args,
        proxy: Box<dyn EventLoopProxy>,
        recv: mpsc::Receiver<LogMessage>,
    ) -> Result<Self> {
        buffer::set_buffer_proxy(proxy.dup());
        let mut palette = CommandPalette::new(proxy.dup());
        let config_path = Config::get_default_location().ok();
        let mut config = match Config::load_from_default_location() {
            Ok(config) => config,
            Err(err) => {
                palette.set_error(err);
                Config::default()
            }
        };

        let mut config_watcher = None;
        if let Some(ref config_path) = config_path {
            match FileWatcher::new(config_path, proxy.dup()) {
                Ok(watcher) => config_watcher = Some(watcher),
                Err(err) => tracing::error!("Error starting config watcher: {err}"),
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
            buffer.goto(args.line as i64);
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

        let workspace = if buffers.is_empty() {
            match Workspace::load_workspace() {
                Ok(workspace) => workspace,
                Err(err) => {
                    tracing::error!("Error loading workspace: {err}");
                    Workspace::default()
                }
            }
        } else {
            Workspace {
                buffers,
                panes: Panes::new(current_buffer_id),
            }
        };

        let branch_watcher = BranchWatcher::new(proxy.dup())?;

        let mut key_mappings = HashMap::new();
        key_mappings.insert(String::from("normal"), get_default_mappings());
        key_mappings.insert(String::from("choord"), get_default_choords());

        let buffer_watcher = if config.watch_open_files {
            BufferWatcher::new(proxy.dup()).ok()
        } else {
            None
        };

        Ok(Self {
            workspace,
            themes,
            config,
            config_path,
            config_watcher,
            palette,
            file_picker: file_finder,
            buffer_picker: None,
            global_search_picker: None,
            key_mappings,
            branch_watcher,
            proxy,
            file_scanner: file_daemon,
            job_manager,
            save_jobs: Default::default(),
            shell_jobs: Default::default(),
            spinner: Default::default(),
            choord: false,
            repeat: None,
            logger_state: LoggerState::new(recv),
            last_render_time: Duration::ZERO,
            start_of_events: Instant::now(),
            closed_buffers: Vec::new(),
            buffer_watcher,
        })
    }

    pub fn do_polling(&mut self, control_flow: &mut EventLoopControlFlow) {
        self.logger_state.update();

        if !self.config.watch_open_files {
            self.buffer_watcher = None;
        } else if let Some(buffer_watcher) = &mut self.buffer_watcher {
            buffer_watcher.update(&mut self.workspace.buffers);
        } else {
            self.buffer_watcher = BufferWatcher::new(self.proxy.dup()).ok();
        }

        if let Some(config_watcher) = &mut self.config_watcher {
            if let Some(result) = config_watcher.poll_update() {
                match result {
                    Ok(config) => {
                        self.config = config;
                        if !self.themes.contains_key(&self.config.theme) {
                            self.config.theme = "default".into();
                        }
                        self.palette.set_msg("Reloaded config");
                    }
                    Err(err) => self.palette.set_error(err),
                }
            }
        }

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

        for buffer in new_buffers {
            self.insert_buffer(buffer, true);
        }

        self.job_manager.poll_jobs();

        let duration = self
            .spinner
            .update(!self.save_jobs.is_empty() || !self.shell_jobs.is_empty());
        *control_flow = EventLoopControlFlow::WaitMax(duration);
    }

    pub fn handle_input_command(
        &mut self,
        input: InputCommand,
        control_flow: &mut EventLoopControlFlow,
        buffer_area: Rect,
    ) {
        if let Some(repeat) = &mut self.repeat {
            match input {
                InputCommand::Char(ch) if ch.is_ascii_digit() => {
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
                            self.handle_single_input_command(
                                input.clone(),
                                control_flow,
                                buffer_area,
                            );
                        }
                    } else {
                        self.handle_single_input_command(input, control_flow, buffer_area);
                        self.repeat = None;
                    }
                }
            }
        } else {
            self.handle_single_input_command(input, control_flow, buffer_area);
        }

        if let Some(repeat) = &self.repeat {
            self.palette.set_msg(format!("Repeat: {repeat}"));
        }
    }

    pub fn handle_single_input_command(
        &mut self,
        input: InputCommand,
        control_flow: &mut EventLoopControlFlow,
        buffer_area: Rect,
    ) {
        if input != InputCommand::Choord {
            self.choord = false;
        }
        match input {
            InputCommand::RotateFile => {
                if let Some(buffer) = self.get_current_buffer() {
                    match buffer.get_next_file() {
                        Ok(file) => {
                            self.open_file(file);
                        }
                        Err(err) => self.palette.set_error(err),
                    };
                };
            }
            InputCommand::Repeat => {
                self.repeat = Some(String::new());
            }
            InputCommand::ReopenBuffer => self.reopen_last_closed_buffer(),
            InputCommand::OpenUrl => self.open_selected_url(),
            InputCommand::Split { direction } => {
                let buffer_id = self.insert_buffer(Buffer::new(), false).0;
                self.workspace
                    .panes
                    .split(PaneKind::Buffer(buffer_id), direction);
            }
            InputCommand::New => {
                self.insert_buffer(Buffer::new(), true);
            }
            InputCommand::Shell => {
                self.file_picker = None;
                self.buffer_picker = None;
                self.global_search_picker = None;
                self.palette
                    .focus("$ ", "shell", CompleterContext::new(&self.themes));
            }
            InputCommand::Format => {
                self.format_current_buffer();
            }
            InputCommand::Choord => {
                self.choord = !self.choord;
            }
            InputCommand::GrowPane => {
                self.workspace.panes.grow_current(buffer_area);
            }
            InputCommand::ShrinkPane => {
                self.workspace.panes.shrink_current(buffer_area);
            }
            InputCommand::Close => {
                self.close_current_buffer();
            }
            InputCommand::ClosePane => {
                self.close_pane();
            }
            InputCommand::Quit => {
                self.quit(control_flow);
            }
            InputCommand::Escape if self.repeat.is_some() => {
                self.repeat = None;
            }
            InputCommand::Escape if self.palette.has_focus() => {
                self.palette.reset();
            }
            InputCommand::FocusPalette if !self.palette.has_focus() => {
                self.file_picker = None;
                self.buffer_picker = None;
                self.global_search_picker = None;
                self.palette
                    .focus("> ", "command", CompleterContext::new(&self.themes));
            }
            InputCommand::PromptGoto => {
                self.file_picker = None;
                self.buffer_picker = None;
                self.global_search_picker = None;
                self.palette
                    .focus("goto: ", "goto", CompleterContext::new(&self.themes));
            }
            InputCommand::Search => self.search(),
            InputCommand::Replace => self.start_replace(),
            InputCommand::GlobalSearch => self.global_search(),
            InputCommand::CaseInsensitive => {
                self.config.case_insensitive_search = !self.config.case_insensitive_search;
                if let Some("search") = self.palette.mode() {
                    self.palette.update_prompt(self.get_search_prompt(false));
                }
                if let Some("global-search") = self.palette.mode() {
                    self.palette.update_prompt(self.get_search_prompt(true));
                }
            }
            InputCommand::Escape
                if self.file_picker.is_some()
                    | self.buffer_picker.is_some()
                    | self.global_search_picker.is_some() =>
            {
                self.file_picker = None;
                self.buffer_picker = None;
                self.global_search_picker = None;
            }
            InputCommand::Escape if self.choord => {
                self.choord = false;
            }
            InputCommand::OpenFileBrowser => self.open_file_picker(),
            InputCommand::OpenBufferBrowser => self.open_buffer_picker(),
            InputCommand::Save => {
                if let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane() {
                    self.save_buffer(buffer_id, None);
                }
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
                        let old = self
                            .workspace
                            .panes
                            .replace_current(PaneKind::Buffer(choice.id));
                        if let PaneKind::Buffer(id) = old {
                            let buffer = &self.workspace.buffers[id];
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
                                let cursor_line = guard.cursor_line_idx();
                                let cursor_col = guard.cursor_grapheme_column();
                                let anchor_line = guard.anchor_line_idx();
                                let anchor_col = guard.anchor_grapheme_column();
                                if let Some(buffer) = self.get_current_buffer_mut() {
                                    buffer.select_area(
                                        Point::new(cursor_col, cursor_line),
                                        Point::new(anchor_col, anchor_line),
                                        false,
                                    );
                                    buffer.center_on_cursor();
                                }
                            }
                        }
                    }
                } else {
                    match self.workspace.panes.get_current_pane() {
                        PaneKind::Buffer(buffer_id) => {
                            if let Err(err) = self.workspace.buffers[buffer_id].handle_input(input)
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

    pub fn handle_command(&mut self, content: String, control_flow: &mut EventLoopControlFlow) {
        use cmd::Command;
        self.palette.reset();
        match cmd_parser::parse_cmd(&content) {
            Ok(cmd) => match cmd {
                Command::FilePickerReload => {
                    self.file_scanner = FileScanner::new(
                        env::current_dir().unwrap_or(PathBuf::from(".")),
                        &self.config,
                    );
                }
                Command::ReplaceAll(replacement) => {
                    if let Some(buffer) = self.get_current_buffer_mut() {
                        buffer.replace_all(replacement);
                    }
                }
                Command::Replace => self.start_replace(),
                Command::Search => self.search(),
                Command::SortLines(asc) => {
                    if let Some(buffer) = self.get_current_buffer_mut() {
                        buffer.sort_lines(asc);
                    }
                }
                Command::Path => match self.try_get_current_buffer_path() {
                    Some(path) => self.palette.set_msg(path.to_string_lossy()),
                    None => self
                        .palette
                        .set_error("No path has been set for the current buffer"),
                },
                Command::About => {
                    self.palette.set_msg(format!(
                        "ferrite\nVersion: {}\nCommit: {}",
                        env!("CARGO_PKG_VERSION"),
                        env!("GIT_HASH"),
                    ));
                }
                Command::UrlOpen => self.open_selected_url(),
                Command::Pwd => match env::current_dir() {
                    Ok(path) => self.palette.set_msg(path.to_string_lossy()),
                    Err(err) => self.palette.set_error(err),
                },
                Command::Cd(path) => {
                    if let Err(err) = self.workspace.save_workspace() {
                        self.palette.set_error(err);
                    } else {
                        match env::set_current_dir(&path) {
                            Ok(_) => {
                                self.buffer_picker = None;
                                self.file_picker = None;

                                self.file_scanner = FileScanner::new(
                                    env::current_dir().unwrap_or(PathBuf::from(".")),
                                    &self.config,
                                );

                                match BranchWatcher::new(self.proxy.dup()) {
                                    Ok(branch_watcher) => self.branch_watcher = branch_watcher,
                                    Err(err) => {
                                        let msg = format!("Error creating branch watcher: {err}");
                                        tracing::error!(msg);
                                        self.palette.set_error(msg);
                                    }
                                }

                                self.workspace = match Workspace::load_workspace() {
                                    Ok(workspace) => workspace,
                                    Err(err) => {
                                        let msg = format!("Error loading workspace: {err}");
                                        tracing::error!(msg);
                                        self.palette.set_error(msg);
                                        Workspace::default()
                                    }
                                };

                                self.palette.set_msg(format!(
                                    "Set working dir to: {}",
                                    path.to_string_lossy()
                                ));
                            }
                            Err(err) => self.palette.set_error(format!("{err}")),
                        }
                    }
                }
                Command::Split(direction) => {
                    let (buffer_id, _) = self.insert_buffer(Buffer::new(), false);
                    self.workspace
                        .panes
                        .split(PaneKind::Buffer(buffer_id), direction);
                }
                Command::Shell { args, pipe } => {
                    self.run_shell_command(args, pipe, false);
                }
                Command::Trash => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
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
                            self.close_current_buffer();
                        }
                    }
                }
                Command::FormatSelection => self.format_selection_current_buffer(),
                Command::Format => self.format_current_buffer(),
                Command::OpenFile(path) => {
                    self.open_file(path);
                }
                Command::SaveFile(path) => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };

                    self.save_buffer(buffer_id, path);
                }
                Command::Language(language) => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
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
                Command::Encoding(encoding) => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
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
                Command::Indent(indent) => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    match indent {
                        Some(indent) => {
                            if let Ok(spaces) = indent.parse::<NonZeroUsize>() {
                                self.workspace.buffers[buffer_id].indent =
                                    Indentation::Spaces(spaces);
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
                Command::LineEnding(line_ending) => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    match line_ending {
                        Some(line_ending) => {
                            self.workspace.buffers[buffer_id].line_ending = line_ending
                        }
                        None => self.palette.set_msg(
                            match self.workspace.buffers[buffer_id].line_ending {
                                line_ending::LineEnding::Crlf => "crlf",
                                line_ending::LineEnding::LF => "lf",
                                _ => unreachable!(),
                            },
                        ),
                    }
                }
                Command::New(path) => {
                    if let Some(path) = path {
                        match Buffer::with_path(path) {
                            Ok(buffer) => drop(self.insert_buffer(buffer, true)),
                            Err(err) => self.palette.set_error(err),
                        }
                    } else {
                        self.insert_buffer(Buffer::new(), true);
                    }
                }
                Command::Reload => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
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
                Command::ReloadAll => {
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
                Command::Goto(line) => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    self.workspace.buffers[buffer_id].goto(line);
                }
                Command::Case(case) => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    self.workspace.buffers[buffer_id].transform_case(case);
                }
                Command::Quit => self.quit(control_flow),
                Command::ForceQuit => *control_flow = EventLoopControlFlow::Exit,
                Command::Logger => {
                    self.logger_state.lines_scrolled_up = 0;
                    self.workspace.panes.replace_current(PaneKind::Logger);
                }
                Command::Theme(name) => match name {
                    Some(name) => {
                        if self.themes.contains_key(&name) {
                            self.config.theme = name;
                        } else {
                            self.palette.set_error("Theme not found");
                        }
                    }
                    None => {
                        self.palette.set_msg(&self.config.theme);
                    }
                },
                Command::BufferPickerOpen => self.open_buffer_picker(),
                Command::FilePickerOpen => {
                    if self.config.picker.file_picker_auto_reload {
                        self.file_scanner = FileScanner::new(
                            env::current_dir().unwrap_or(PathBuf::from(".")),
                            &self.config,
                        );
                    }
                    self.open_file_picker();
                }
                Command::OpenConfig => self.open_config(),
                Command::DefaultConfig => self.open_default_config(),
                Command::ForceClose => self.force_close_current_buffer(),
                Command::Close => self.close_current_buffer(),
                Command::ClosePane => self.close_pane(),
                Command::Paste => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    if let Err(err) =
                        self.workspace.buffers[buffer_id].handle_input(InputCommand::Paste)
                    {
                        self.palette.set_error(err);
                    }
                }
                Command::Copy => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    if let Err(err) =
                        self.workspace.buffers[buffer_id].handle_input(InputCommand::Copy)
                    {
                        self.palette.set_error(err);
                    }
                }
                Command::RevertBuffer => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    let _ =
                        self.workspace.buffers[buffer_id].handle_input(InputCommand::RevertBuffer);
                }
                Command::GitReload => self.branch_watcher.force_reload(),
                Command::GitDiff => {
                    self.run_shell_command(vec!["git".into(), "diff".into()], true, true);
                }
            },
            Err(err) => self.palette.set_error(err),
        }
    }

    pub fn handle_app_event(&mut self, event: UserEvent, control_flow: &mut EventLoopControlFlow) {
        match event {
            UserEvent::Wake => (),
            UserEvent::PaletteEvent { mode, content } => match mode.as_str() {
                "command" => self.handle_command(content, control_flow),
                "goto" => {
                    self.palette.reset();
                    if let Ok(line) = content.trim().parse::<i64>() {
                        let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                        else {
                            return;
                        };
                        self.workspace.buffers[buffer_id].goto(line);
                    }
                }
                "search" => {
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    self.workspace.buffers[buffer_id].start_search(
                        self.proxy.dup(),
                        content,
                        self.config.case_insensitive_search,
                    );
                    self.palette.unfocus();
                }
                "replace" => {
                    self.palette.unfocus();
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
                    else {
                        return;
                    };
                    let buffer = &mut self.workspace.buffers[buffer_id];
                    buffer.replacement = Some(content);
                }
                "global-search" => {
                    self.palette.unfocus();
                    let global_search_provider = GlobalSearchProvider::new(
                        content,
                        self.config.picker,
                        self.config.case_insensitive_search,
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
                    let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane()
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
        let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane() else {
            return;
        };
        let buffer_lang = self.workspace.buffers[buffer_id].language_name();
        let config = self
            .config
            .language
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

        if let Err(err) = self.workspace.buffers[buffer_id].format_selection(fmt) {
            // FIXME make error able to display more then one line
            self.palette.set_error(err);
        }
    }

    pub fn format_current_buffer(&mut self) {
        if let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane() {
            let buffer_lang = self.workspace.buffers[buffer_id].language_name();
            let config = self
                .config
                .language
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

            if let Err(err) = self.workspace.buffers[buffer_id].format(fmt) {
                // FIXME make error able to display more then one line
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
                self.workspace.panes.replace_current(PaneKind::Buffer(id));
                true
            }
            None => match Buffer::from_file(path) {
                Ok(buffer) => {
                    if let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane() {
                        let current_buf = self.workspace.buffers.get_mut(buffer_id).unwrap();
                        if current_buf.is_disposable() {
                            *current_buf = buffer;
                            return true;
                        }
                    }
                    self.insert_buffer(buffer, true);
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
        } else if self.config.always_prompt_on_exit {
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
            &self.config,
        );
        self.file_picker = Some(Picker::new(
            FileFindProvider(self.file_scanner.subscribe()),
            Some(Box::new(FilePreviewer::new(self.proxy.dup()))),
            self.proxy.dup(),
            self.try_get_current_buffer_path(),
        ));
    }

    pub fn open_config(&mut self) {
        match &self.config_path {
            Some(path) => {
                self.open_file(path.clone());
            }
            None => self.palette.set_error("Could not locate the config file"),
        }
    }

    pub fn open_default_config(&mut self) {
        let mut buffer = Buffer::with_name("default_config.toml");
        buffer.set_text(DEFAULT_CONFIG);
        self.insert_buffer(buffer, true);
    }

    pub fn close_current_buffer(&mut self) {
        if let Some(buffer) = self.get_current_buffer() {
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
    fn get_next_buffer(&mut self) -> BufferId {
        let mut next_buffer = None;
        let mut buffers: Vec<_> = self.workspace.buffers.iter().collect();
        buffers.sort_by(|a, b| b.1.get_last_interact().cmp(&a.1.get_last_interact()));
        for (buffer_id, _) in buffers {
            if !self.workspace.panes.contains(PaneKind::Buffer(buffer_id)) {
                next_buffer = Some(buffer_id);
                break;
            }
        }

        next_buffer.unwrap_or_else(|| self.workspace.buffers.insert(Buffer::new()))
    }

    pub fn close_pane(&mut self) {
        if self.workspace.panes.num_panes() > 1 {
            if let Some(buffer_id) = self.get_current_buffer_id() {
                self.workspace
                    .panes
                    .remove_pane(PaneKind::Buffer(buffer_id));
            } else {
                let buffer_id = self.get_next_buffer();
                self.workspace
                    .panes
                    .replace_current(PaneKind::Buffer(buffer_id));
            }
        }
    }

    pub fn force_close_current_buffer(&mut self) {
        if let Some(buffer_id) = self.get_current_buffer_id() {
            if self.workspace.panes.num_panes() > 1 || self.workspace.buffers.len() > 1 {
                if let Some(path) = self.workspace.buffers.remove(buffer_id).unwrap().file() {
                    self.insert_removed_buffer(path.to_path_buf());
                }
                let buffer_id = self.get_next_buffer();
                self.workspace
                    .panes
                    .replace_current(PaneKind::Buffer(buffer_id));
            } else {
                if let Some(path) = self.workspace.buffers[buffer_id].file() {
                    self.insert_removed_buffer(path.to_path_buf());
                }
                self.workspace.buffers[buffer_id] = Buffer::new();
            }
        } else {
            let buffer_id = self.get_next_buffer();
            self.workspace
                .panes
                .replace_current(PaneKind::Buffer(buffer_id));
        }
    }

    pub fn reopen_last_closed_buffer(&mut self) {
        while let Some(path) = self.closed_buffers.pop() {
            if let Some(buffer) = self.get_current_buffer() {
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
        if self.config.case_insensitive_search {
            prompt += " (i): ";
        } else {
            prompt += ": ";
        }
        prompt
    }

    pub fn get_current_buffer_id(&self) -> Option<BufferId> {
        match self.workspace.panes.get_current_pane() {
            PaneKind::Buffer(id) => Some(id),
            _ => None,
        }
    }

    pub fn get_current_buffer(&self) -> Option<&Buffer> {
        let PaneKind::Buffer(buffer) = self.workspace.panes.get_current_pane() else {
            return None;
        };

        self.workspace.buffers.get(buffer)
    }

    pub fn get_current_buffer_mut(&mut self) -> Option<&mut Buffer> {
        let PaneKind::Buffer(buffer) = self.workspace.panes.get_current_pane() else {
            return None;
        };

        self.workspace.buffers.get_mut(buffer)
    }

    pub fn insert_buffer(&mut self, buffer: Buffer, make_current: bool) -> (BufferId, &mut Buffer) {
        let buffer_id = self.workspace.buffers.insert(buffer);
        if make_current {
            self.workspace
                .panes
                .replace_current(PaneKind::Buffer(buffer_id));
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

    pub fn get_current_keymappings(&self) -> &[(Mapping, InputCommand, Exclusiveness)] {
        if self.choord {
            self.key_mappings.get("choord").unwrap()
        } else {
            self.key_mappings.get("normal").unwrap()
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
        if let Some(buffer) = self.get_current_buffer() {
            let selection = buffer.get_selection();
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
        if let Some(buffer) = self.get_current_buffer() {
            let selection = buffer.get_selection();
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
            .map(|buffer| buffer.get_selection())
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
        let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane() else {
            return;
        };
        let buffer = &mut self.workspace.buffers[buffer_id];
        if buffer.get_searcher().is_some() {
            self.palette
                .focus("replace: ", "replace", CompleterContext::new(&self.themes));
        }
    }

    fn try_get_current_buffer_path(&self) -> Option<PathBuf> {
        self.get_current_buffer()?.file().map(|p| p.to_owned())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Err(e) = self.workspace.save_workspace() {
            tracing::error!("Error saving workspace: {e}");
        };
    }
}
