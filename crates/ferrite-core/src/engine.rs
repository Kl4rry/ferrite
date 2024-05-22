use std::{
    collections::HashMap,
    env, io,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

use anyhow::Result;
use ferrite_cli::Args;
use ferrite_utility::line_ending;
use slab::Slab;
use subprocess::{Exec, Redirection};

use crate::{
    buffer::{self, encoding::get_encoding, Buffer},
    byte_size::format_byte_size,
    clipboard,
    config::{Config, ConfigWatcher, DEFAULT_CONFIG},
    event_loop_proxy::{EventLoopControlFlow, EventLoopProxy, UserEvent},
    git::branch::BranchWatcher,
    indent::Indentation,
    job_manager::{JobHandle, JobManager},
    jobs::SaveBufferJob,
    keymap::{get_default_choords, get_default_mappings, Exclusiveness, InputCommand, Mapping},
    logger::{LogMessage, LoggerState},
    palette::{cmd, cmd_parser, completer::CompleterContext, CommandPalette, PalettePromptEvent},
    panes::{PaneKind, Panes, Rect},
    search_buffer::{
        buffer_find::{BufferFindProvider, BufferItem},
        file_daemon::FileDaemon,
        file_find::FileFindProvider,
        SearchBuffer,
    },
    spinner::Spinner,
    theme::EditorTheme,
    workspace::Workspace,
};

pub struct Engine {
    pub workspace: Workspace,
    pub themes: HashMap<String, EditorTheme>,
    pub config: Config,
    pub config_path: Option<PathBuf>,
    pub config_watcher: Option<ConfigWatcher>,
    pub palette: CommandPalette,
    pub file_finder: Option<SearchBuffer<String>>,
    pub buffer_finder: Option<SearchBuffer<BufferItem>>,
    pub key_mappings: HashMap<String, Vec<(Mapping, InputCommand, Exclusiveness)>>,
    pub branch_watcher: BranchWatcher,
    pub proxy: Box<dyn EventLoopProxy>,
    pub file_daemon: FileDaemon,
    pub job_manager: JobManager,
    pub save_jobs: Vec<JobHandle<Result<SaveBufferJob>>>,
    pub shell_jobs: Vec<JobHandle<Result<(bool, Buffer), anyhow::Error>>>,
    pub spinner: Spinner,
    pub logger_state: LoggerState,
    pub choord: bool,
    pub last_render_time: Duration,
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
            config_watcher = Some(ConfigWatcher::watch(config_path, proxy.dup())?);
        }

        if config.local_clipboard {
            clipboard::set_local_clipboard(true);
        }

        let themes = EditorTheme::load_themes();
        if !themes.contains_key(&config.theme) {
            config.theme = "default".into();
        }

        let mut buffers = Slab::new();
        let mut current_buffer_id = 0;

        for (i, file) in args.files.iter().enumerate() {
            if i == 0 && file.is_dir() {
                continue;
            }

            let buffer = match Buffer::from_file(file) {
                Ok(buffer) => buffer,
                Err(err) => match err.kind() {
                    io::ErrorKind::NotFound => Buffer::with_path(file)?,
                    _ => Err(err)?,
                },
            };
            current_buffer_id = buffers.insert(buffer);
        }

        for (_, buffer) in &mut buffers {
            buffer.goto(args.line as i64);
            if let Some(language) = &args.language {
                buffer.set_langauge(language, proxy.dup())?;
            }
        }

        let mut file_daemon = None;
        let mut file_finder = None;

        if let Some(path) = args.files.first() {
            if path.is_dir() {
                std::env::set_current_dir(path)?;
                let daemon = FileDaemon::new(std::env::current_dir()?, &config)?;
                file_finder = Some(SearchBuffer::new(
                    FileFindProvider(daemon.subscribe()),
                    proxy.dup(),
                ));
                file_daemon = Some(daemon);
            }
        }

        let file_daemon = if let Some(daemon) = file_daemon {
            daemon
        } else {
            FileDaemon::new(std::env::current_dir()?, &config)?
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

        let branch_watcher = BranchWatcher::new(proxy.dup(), file_daemon.change_detector())?;

        let mut key_mappings = HashMap::new();
        key_mappings.insert(String::from("normal"), get_default_mappings());
        key_mappings.insert(String::from("choord"), get_default_choords());

        Ok(Self {
            workspace,
            themes,
            config,
            config_path,
            config_watcher,
            palette,
            file_finder,
            buffer_finder: None,
            key_mappings,
            branch_watcher,
            proxy,
            file_daemon,
            job_manager,
            save_jobs: Default::default(),
            shell_jobs: Default::default(),
            spinner: Default::default(),
            choord: false,
            logger_state: LoggerState::new(recv),
            last_render_time: Duration::ZERO,
        })
    }

    pub fn do_polling(&mut self, control_flow: &mut EventLoopControlFlow) {
        self.logger_state.update();

        if let Some(config_watcher) = &self.config_watcher {
            if config_watcher.has_changed() {
                if let Some(path) = &self.config_path {
                    match Config::load(path) {
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
        if input != InputCommand::Choord {
            self.choord = false;
        }
        match input {
            InputCommand::OpenUrl => {
                self.open_selected_url();
            }
            InputCommand::Split { direction } => {
                let buffer_id = self.insert_buffer(Buffer::new(), false).0;
                self.workspace
                    .panes
                    .split(PaneKind::Buffer(buffer_id), direction);
                self.open_file_picker();
            }
            InputCommand::Shell => {
                self.file_finder = None;
                self.buffer_finder = None;
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
            InputCommand::Quit => {
                self.quit(control_flow);
            }
            InputCommand::Escape if self.palette.has_focus() => {
                self.palette.reset();
            }
            InputCommand::FocusPalette if !self.palette.has_focus() => {
                self.file_finder = None;
                self.buffer_finder = None;
                self.palette
                    .focus("> ", "command", CompleterContext::new(&self.themes));
            }
            InputCommand::PromptGoto => {
                self.file_finder = None;
                self.buffer_finder = None;
                self.palette
                    .focus("goto: ", "goto", CompleterContext::new(&self.themes));
            }
            InputCommand::FileSearch => {
                if let Some(buffer) = self.get_current_buffer() {
                    let selection = buffer.get_selection();
                    self.file_finder = None;
                    self.buffer_finder = None;
                    self.palette.focus(
                        self.get_search_prompt(),
                        "search",
                        CompleterContext::new(&self.themes),
                    );
                    self.palette.set_line(selection);
                }
            }
            InputCommand::CaseInsensitive => {
                self.config.case_insensitive_search = !self.config.case_insensitive_search;
                if let Some("search") = self.palette.mode() {
                    self.palette.update_prompt(self.get_search_prompt());
                }
            }
            InputCommand::Escape if self.file_finder.is_some() | self.buffer_finder.is_some() => {
                self.file_finder = None;
                self.buffer_finder = None;
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
                } else if let Some(finder) = &mut self.file_finder {
                    let _ = finder.handle_input(input);
                    if let Some(path) = finder.get_choice() {
                        self.file_finder = None;
                        self.open_file(path);
                    }
                } else if let Some(finder) = &mut self.buffer_finder {
                    let _ = finder.handle_input(input);
                    if let Some(choice) = finder.get_choice() {
                        self.buffer_finder = None;
                        self.workspace
                            .panes
                            .replace_current(PaneKind::Buffer(choice.id));
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

    pub fn handle_app_event(&mut self, event: UserEvent, control_flow: &mut EventLoopControlFlow) {
        match event {
            UserEvent::Wake => (),
            UserEvent::PaletteEvent { mode, content } => match mode.as_str() {
                "command" => {
                    use cmd::Command;
                    self.palette.reset();
                    match cmd_parser::parse_cmd(&content) {
                        Ok(cmd) => match cmd {
                            Command::About => {
                                self.palette.set_msg(format!(
                                    "ferrite\nVersion: {}\nCommit: {}",
                                    env!("CARGO_PKG_VERSION"),
                                    env!("GIT_HASH"),
                                ));
                            }
                            Command::Url => self.open_selected_url(),
                            Command::Pwd => match env::current_dir() {
                                Ok(path) => self.palette.set_msg(path.to_string_lossy()),
                                Err(err) => self.palette.set_error(err),
                            },
                            Command::Cd(path) => match env::set_current_dir(&path) {
                                Ok(_) => {
                                    self.buffer_finder = None;
                                    self.file_finder = None;

                                    match FileDaemon::new(
                                        env::current_dir().unwrap_or(PathBuf::from(".")),
                                        &self.config,
                                    ) {
                                        Ok(file_daemon) => self.file_daemon = file_daemon,
                                        Err(err) => {
                                            let msg = format!("Error creating file daemon: {err}");
                                            tracing::error!(msg);
                                            self.palette.set_error(msg);
                                        }
                                    }

                                    match BranchWatcher::new(
                                        self.proxy.dup(),
                                        self.file_daemon.change_detector(),
                                    ) {
                                        Ok(branch_watcher) => self.branch_watcher = branch_watcher,
                                        Err(err) => {
                                            let msg =
                                                format!("Error creating branch watcher: {err}");
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
                            },
                            Command::Split(direction) => {
                                let buffer_id = self.insert_buffer(Buffer::new(), false).0;
                                self.workspace
                                    .panes
                                    .split(PaneKind::Buffer(buffer_id), direction);
                                self.open_file_picker();
                            }
                            Command::Shell { args, pipe } => {
                                self.run_shell_command(args, pipe);
                            }
                            Command::Delete => {
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
                                else {
                                    return;
                                };

                                match self.workspace.buffers[buffer_id].move_to_trash() {
                                    Ok(true) => {
                                        let path =
                                            self.workspace.buffers[buffer_id].file().unwrap();
                                        self.palette.set_msg(format!(
                                            "`{}` moved to trash",
                                            path.to_string_lossy()
                                        ));
                                        self.close_current_buffer();
                                    }
                                    Ok(false) => {
                                        self.palette.set_error(
                                            "No path set for file, cannot move to trash",
                                        );
                                    }
                                    Err(e) => {
                                        self.palette.set_error(e);
                                        self.close_current_buffer();
                                    }
                                }
                            }
                            Command::FormatSelection => self.format_selection_current_buffer(),
                            Command::Format => self.format_current_buffer(),
                            Command::OpenFile(path) => self.open_file(path),
                            Command::SaveFile(path) => {
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
                                else {
                                    return;
                                };

                                self.save_buffer(buffer_id, path);
                            }
                            Command::Language(language) => {
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
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
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
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
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
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
                                            self.palette.set_error(
                                                "Indentation must be a number or `tabs`",
                                            );
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
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
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
                            Command::New => {
                                self.insert_buffer(Buffer::new(), true);
                            }
                            Command::Reload => {
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
                                else {
                                    return;
                                };
                                if self.workspace.buffers[buffer_id].is_dirty() {
                                    self.palette.set_prompt(
                                        "The buffer is unsaved are you sure you want to reload?",
                                        ('y', PalettePromptEvent::Reload),
                                        ('n', PalettePromptEvent::Nop),
                                    );
                                } else if let Err(err) = self.workspace.buffers[buffer_id].reload()
                                {
                                    self.palette.set_error(err)
                                };
                            }
                            Command::Goto(line) => {
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
                                else {
                                    return;
                                };
                                self.workspace.buffers[buffer_id].goto(line);
                            }
                            Command::Case(case) => {
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
                                else {
                                    return;
                                };
                                self.workspace.buffers[buffer_id].transform_case(case);
                            }
                            Command::Quit => self.quit(control_flow),
                            Command::ForceQuit => *control_flow = EventLoopControlFlow::Exit,
                            Command::Logger => {
                                self.logger_state.lines_scrolled_up = 0;
                                self.workspace.panes.replace_current(PaneKind::Logger)
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
                            Command::BrowseBuffers => self.open_buffer_picker(),
                            Command::BrowseWorkspace => self.open_file_picker(),
                            Command::OpenConfig => self.open_config(),
                            Command::DefaultConfig => self.open_default_config(),
                            Command::ForceClose => self.force_close_current_buffer(),
                            Command::Close => self.close_current_buffer(),
                            Command::Paste => {
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
                                else {
                                    return;
                                };
                                if let Err(err) = self.workspace.buffers[buffer_id]
                                    .handle_input(InputCommand::Paste)
                                {
                                    self.palette.set_error(err);
                                }
                            }
                            Command::Copy => {
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
                                else {
                                    return;
                                };
                                if let Err(err) = self.workspace.buffers[buffer_id]
                                    .handle_input(InputCommand::Copy)
                                {
                                    self.palette.set_error(err);
                                }
                            }
                            Command::RevertBuffer => {
                                let PaneKind::Buffer(buffer_id) =
                                    self.workspace.panes.get_current_pane()
                                else {
                                    return;
                                };
                                let _ = self.workspace.buffers[buffer_id]
                                    .handle_input(InputCommand::RevertBuffer);
                            }
                            Command::GitReload => self.branch_watcher.force_reload(),
                        },
                        Err(err) => self.palette.set_error(err),
                    }
                }
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
                "shell" => {
                    let args: Vec<_> = content.split_whitespace().map(PathBuf::from).collect();
                    self.run_shell_command(args, false);
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

    pub fn open_file(&mut self, path: impl AsRef<Path>) {
        let real_path = match dunce::canonicalize(&path) {
            Ok(path) => path,
            Err(err) => {
                self.palette.set_error(err);
                return;
            }
        };

        match self.workspace.buffers.iter().find(|(_, buffer)| {
            buffer
                .file()
                .and_then(|path| dunce::canonicalize(path).ok())
                .as_deref()
                == Some(&real_path)
        }) {
            Some((id, _)) => self.workspace.panes.replace_current(PaneKind::Buffer(id)),
            None => match Buffer::from_file(path) {
                Ok(buffer) => {
                    if let PaneKind::Buffer(buffer_id) = self.workspace.panes.get_current_pane() {
                        let current_buf = self.workspace.buffers.get_mut(buffer_id).unwrap();
                        if !current_buf.is_dirty() && current_buf.rope().len_bytes() == 0 {
                            *current_buf = buffer;
                            return;
                        }
                    }
                    self.insert_buffer(buffer, true);
                }
                Err(err) => self.palette.set_error(err),
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
        self.file_finder = None;
        let buffers: Vec<_> = self
            .workspace
            .buffers
            .iter()
            .map(|(id, buffer)| BufferItem {
                id,
                dirty: buffer.is_dirty(),
                name: buffer.name().to_string(),
            })
            .collect();

        self.buffer_finder = Some(SearchBuffer::new(
            BufferFindProvider(buffers.into()),
            self.proxy.dup(),
        ));
    }

    pub fn open_file_picker(&mut self) {
        self.palette.reset();
        self.buffer_finder = None;
        self.file_finder = Some(SearchBuffer::new(
            FileFindProvider(self.file_daemon.subscribe()),
            self.proxy.dup(),
        ));
    }

    pub fn open_config(&mut self) {
        match &self.config_path {
            Some(path) => self.open_file(path.clone()),
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

    pub fn force_close_current_buffer(&mut self) {
        if let Some(buffer_id) = self.get_current_buffer_id() {
            if self.workspace.panes.num_panes() > 1 {
                self.workspace
                    .panes
                    .remove_pane(PaneKind::Buffer(buffer_id));
                self.workspace.buffers.remove(buffer_id);
            } else if self.workspace.buffers.len() > 1 {
                self.workspace.buffers.remove(buffer_id);
                let (buffer_id, _) = self.workspace.buffers.iter().next().unwrap();
                self.workspace
                    .panes
                    .replace_current(PaneKind::Buffer(buffer_id));
            } else {
                self.workspace.buffers[buffer_id] = Buffer::new();
            }
        } else {
            self.workspace.panes.remove_pane(PaneKind::Logger);
        }
    }

    pub fn get_search_prompt(&self) -> String {
        let mut prompt = String::from("search");
        if self.config.case_insensitive_search {
            prompt += " (i): ";
        } else {
            prompt += ": ";
        }
        prompt
    }

    pub fn get_current_buffer_id(&self) -> Option<usize> {
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

    pub fn _get_current_buffer_mut(&mut self) -> Option<&mut Buffer> {
        let PaneKind::Buffer(buffer) = self.workspace.panes.get_current_pane() else {
            return None;
        };

        self.workspace.buffers.get_mut(buffer)
    }

    pub fn insert_buffer(&mut self, buffer: Buffer, make_current: bool) -> (usize, &mut Buffer) {
        let buffer_id = self.workspace.buffers.insert(buffer);
        if make_current {
            self.workspace
                .panes
                .replace_current(PaneKind::Buffer(buffer_id));
        }
        (buffer_id, &mut self.workspace.buffers[buffer_id])
    }

    pub fn save_buffer(&mut self, buffer_id: usize, path: Option<PathBuf>) {
        let buffer = &mut self.workspace.buffers[buffer_id];

        if let Some(path) = path {
            buffer.set_file(path);
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

    pub fn run_shell_command(&mut self, args: Vec<PathBuf>, pipe: bool) {
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

                let buffer = Buffer::from_bytes(&stdout.unwrap())?;

                Ok((pipe, buffer))
            },
            (),
        );
        self.shell_jobs.push(job);
    }

    pub fn open_selected_url(&mut self) {
        if let Some(buffer) = self.get_current_buffer() {
            let selection = buffer.get_selection();
            if let Err(err) = opener::open(selection) {
                self.palette.set_error(err);
            }
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Err(e) = self.workspace.save_workspace() {
            tracing::error!("Error saving workspace: {e}");
        };
    }
}
