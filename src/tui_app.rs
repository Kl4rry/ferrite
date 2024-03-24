use std::{
    collections::HashMap,
    io::{self, Stdout},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    thread,
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind, MouseButton, MouseEventKind},
    execute, terminal,
};
use slab::Slab;
use subprocess::{Exec, Redirection};
use tui::layout::{Margin, Position, Rect};
use utility::{line_ending, point::Point};

use self::{
    event_loop::{TuiAppEvent, TuiEvent, TuiEventLoop, TuiEventLoopControlFlow, TuiEventLoopProxy},
    keymap::{get_default_mappings, Exclusiveness, Mapping},
    panes::{PaneKind, Panes},
    widgets::{
        background_widget::BackgroundWidget,
        editor_widget::{lines_to_left_offset, EditorWidget},
        palette_widget::CmdPaletteWidget,
        search_widget::SearchWidget,
        splash::SplashWidget,
    },
};
use crate::{
    clipboard,
    ferrite_core::{
        buffer::{encoding::get_encoding, Buffer},
        byte_size::format_byte_size,
        config::{Config, ConfigWatcher},
        git::branch::BranchWatcher,
        indent::Indentation,
        palette::{
            cmd, cmd_parser, completer::CompleterContext, CommandPalette, PalettePromptEvent,
        },
        search_buffer::{
            buffer_find::{BufferFindProvider, BufferItem},
            file_daemon::FileDaemon,
            file_find::FileFindProvider,
            SearchBuffer,
        },
        theme::EditorTheme,
    },
    tui_app::keymap::InputCommand,
    Args,
};

pub mod event_loop;
pub mod keymap;
pub mod panes;
pub mod rect_ext;
mod widgets;

pub struct TuiApp {
    terminal: tui::Terminal<tui::backend::CrosstermBackend<Stdout>>,
    buffers: Slab<Buffer>,
    buffer_area: Rect,
    panes: Panes,
    themes: HashMap<String, EditorTheme>,
    config: Config,
    config_path: Option<PathBuf>,
    config_watcher: Option<ConfigWatcher>,
    palette: CommandPalette,
    file_finder: Option<SearchBuffer<String>>,
    buffer_finder: Option<SearchBuffer<BufferItem>>,
    key_mappings: Vec<(Mapping, InputCommand, Exclusiveness)>,
    branch_watcher: BranchWatcher,
    proxy: TuiEventLoopProxy,
    drag_start: Option<Point<usize>>,
    file_daemon: FileDaemon,
}

impl TuiApp {
    pub fn new(args: &Args, proxy: TuiEventLoopProxy) -> Result<Self> {
        let mut palette = CommandPalette::new(proxy.clone());
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
            config_watcher = Some(ConfigWatcher::watch(config_path, proxy.clone())?);
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

            let buffer = match Buffer::from_file(file, proxy.clone()) {
                Ok(buffer) => buffer,
                Err(err) => match err.kind() {
                    io::ErrorKind::NotFound => Buffer::with_path(file, proxy.clone()),
                    _ => Err(err)?,
                },
            };
            current_buffer_id = buffers.insert(buffer);
        }

        if buffers.is_empty() {
            current_buffer_id = buffers.insert(Buffer::new());
        }

        let (width, height) = crossterm::terminal::size()?;
        for (_, buffer) in &mut buffers {
            buffer.set_view_lines(height.saturating_sub(2).into());
            buffer.set_view_columns(width.into());
            buffer.goto(args.line as i64);
            if let Some(language) = &args.language {
                buffer.set_langauge(language, proxy.clone())?;
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
                    proxy.clone(),
                ));
                file_daemon = Some(daemon);
            }
        }

        let file_daemon = if let Some(daemon) = file_daemon {
            daemon
        } else {
            FileDaemon::new(std::env::current_dir()?, &config)?
        };

        Ok(Self {
            terminal: tui::Terminal::new(tui::backend::CrosstermBackend::new(std::io::stdout()))?,
            buffers,
            buffer_area: Rect {
                x: 0,
                y: 0,
                width,
                height: height.saturating_sub(2),
            },
            panes: Panes::new(current_buffer_id),
            themes,
            palette,
            file_finder,
            buffer_finder: None,
            key_mappings: get_default_mappings(),
            branch_watcher: BranchWatcher::new(proxy.clone(), file_daemon.change_detector())?,
            proxy,
            drag_start: None,
            file_daemon,
            config,
            config_path,
            config_watcher,
        })
    }

    pub fn new_buffer_with_text(&mut self, text: &str) -> &mut Buffer {
        let mut buffer = Buffer::new();
        buffer.set_text(text);
        self.insert_buffer(buffer, true).1
    }

    pub fn run(mut self, event_loop: TuiEventLoop) -> Result<()> {
        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode()?;
        execute!(
            stdout,
            event::EnableBracketedPaste,
            terminal::EnterAlternateScreen,
            terminal::Clear(terminal::ClearType::Purge),
            event::EnableMouseCapture,
        )?;

        // Reset terminal to non raw mode on panic
        {
            let default_panic = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                _ = terminal::disable_raw_mode();
                println!();
                let _ = std::fs::write("./panic.txt", format!("{info:?}"));
                default_panic(info);
            }));
        }

        event_loop.run(|proxy, event, control_flow| self.handle_event(proxy, event, control_flow));

        Ok(())
    }

    pub fn handle_event(
        &mut self,
        proxy: &TuiEventLoopProxy,
        event: TuiEvent,
        control_flow: &mut TuiEventLoopControlFlow,
    ) {
        match event {
            event_loop::TuiEvent::Crossterm(event) => {
                self.handle_crossterm_event(proxy, event, control_flow)
            }
            event_loop::TuiEvent::AppEvent(event) => {
                self.handle_app_event(proxy, event, control_flow)
            }
            event_loop::TuiEvent::Render => {
                self.do_polling();
                self.render();
            }
        }
    }

    pub fn do_polling(&mut self) {
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
    }

    pub fn render(&mut self) {
        self.terminal
            .draw(|f| {
                let theme = &self.themes[&self.config.theme];
                f.render_widget(BackgroundWidget::new(theme), f.size());
                let size = f.size();
                let editor_size = Rect::new(size.x, size.y, size.width, size.height - 1);

                self.buffer_area = editor_size;
                let current_pane = self.panes.get_current_pane();
                for (pane, pane_rect) in self.panes.get_pane_bounds(editor_size) {
                    if let PaneKind::Buffer(buffer_id) = pane {
                        f.render_stateful_widget(
                            EditorWidget::new(
                                theme,
                                &self.config,
                                !self.palette.has_focus()
                                    && self.file_finder.is_none()
                                    && current_pane == pane,
                                self.branch_watcher.current_branch(),
                            ),
                            pane_rect,
                            &mut self.buffers[buffer_id],
                        );

                        if self.config.show_splash && self.panes.len() == 1 {
                            let buffer = &mut self.buffers[buffer_id];
                            if buffer.rope().len_bytes() == 0
                                && !buffer.is_dirty()
                                && buffer.file().is_none()
                                && self.buffers.len() == 1
                            {
                                f.render_widget(SplashWidget::new(theme), pane_rect);
                            }
                        }
                    }
                }

                if let Some(file_finder) = &mut self.file_finder {
                    let size = size.inner(&Margin {
                        horizontal: 5,
                        vertical: 2,
                    });
                    f.render_stateful_widget(
                        SearchWidget::new(theme, "Open file"),
                        size,
                        file_finder,
                    );
                }

                if let Some(buffer_finder) = &mut self.buffer_finder {
                    let size = size.inner(&Margin {
                        horizontal: 5,
                        vertical: 2,
                    });
                    f.render_stateful_widget(
                        SearchWidget::<BufferItem>::new(theme, "Open buffer"),
                        size,
                        buffer_finder,
                    );
                }

                let palette_size = Rect::new(size.left(), size.bottom() - 1, size.width, 1);
                f.render_stateful_widget(
                    CmdPaletteWidget::new(theme, self.palette.has_focus(), size),
                    palette_size,
                    &mut self.palette,
                );
            })
            .unwrap();
    }

    pub fn handle_crossterm_event(
        &mut self,
        _proxy: &TuiEventLoopProxy,
        event: event::Event,
        control_flow: &mut TuiEventLoopControlFlow,
    ) {
        {
            let input = 'block: {
                match event {
                    Event::Key(event) => {
                        if event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat {
                            tracing::trace!("{:?}", event);
                            keymap::get_command_from_input(
                                event.code,
                                event.modifiers,
                                &self.key_mappings,
                            )
                        } else {
                            None
                        }
                    }
                    Event::Mouse(event) => match event.kind {
                        // TODO allow scoll when using cmd palette
                        MouseEventKind::ScrollUp => Some(InputCommand::VerticalScroll(-3)),
                        MouseEventKind::ScrollDown => Some(InputCommand::VerticalScroll(3)),
                        MouseEventKind::Down(MouseButton::Middle) => {
                            for (pane_kind, pane_rect) in
                                self.panes.get_pane_bounds(self.buffer_area)
                            {
                                if pane_rect.contains(Position::new(event.column, event.row)) {
                                    self.panes.make_current(pane_kind);
                                    if let PaneKind::Buffer(buffer_id) = pane_kind {
                                        let buffer = &self.buffers[buffer_id];
                                        let (_, left_offset) =
                                            lines_to_left_offset(buffer.len_lines());
                                        let column = (event.column as usize)
                                            .saturating_sub(left_offset)
                                            + buffer.col_pos().saturating_sub(pane_rect.x.into());
                                        let line = (event.row as usize + buffer.line_pos())
                                            .saturating_sub(pane_rect.y.into());
                                        break 'block Some(InputCommand::PastePrimary(
                                            column, line,
                                        ));
                                    }
                                }
                            }

                            None
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            for (pane_kind, pane_rect) in
                                self.panes.get_pane_bounds(self.buffer_area)
                            {
                                if pane_rect.contains(Position::new(event.column, event.row)) {
                                    self.panes.make_current(pane_kind);
                                    if let PaneKind::Buffer(buffer_id) = pane_kind {
                                        self.drag_start = Some(Point::new(
                                            event.column as usize,
                                            event.row as usize,
                                        ));

                                        let buffer = &self.buffers[buffer_id];
                                        let (_, left_offset) =
                                            lines_to_left_offset(buffer.len_lines());
                                        let column = (event.column as usize)
                                            .saturating_sub(left_offset)
                                            + buffer.col_pos();
                                        let line = event.row as usize
                                            + buffer.line_pos().saturating_sub(pane_rect.y.into());
                                        break 'block Some(InputCommand::SetCursorPos(
                                            column, line,
                                        ));
                                    }
                                }
                            }

                            None
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            self.drag_start = None;
                            None
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            for (pane_kind, pane_rect) in
                                self.panes.get_pane_bounds(self.buffer_area)
                            {
                                if pane_rect.contains(Position::new(event.column, event.row)) {
                                    self.panes.make_current(pane_kind);
                                    if let PaneKind::Buffer(buffer_id) = pane_kind {
                                        // TODO maybe scroll more of the buffer into view when going outside its bounds
                                        if let Some(Point { line, column }) = self.drag_start {
                                            let buffer = &mut self.buffers[buffer_id];
                                            let (_, left_offset) =
                                                lines_to_left_offset(buffer.len_lines());

                                            let anchor = {
                                                let column = column.saturating_sub(left_offset)
                                                    + buffer.col_pos();
                                                let line = line
                                                    + buffer
                                                        .line_pos()
                                                        .saturating_sub(pane_rect.y.into());
                                                Point::new(column, line)
                                            };

                                            let cursor = {
                                                let column = (event.column as usize)
                                                    .saturating_sub(left_offset)
                                                    + buffer.col_pos();
                                                let line = event.row as usize
                                                    + buffer
                                                        .line_pos()
                                                        .saturating_sub(pane_rect.y.into());
                                                Point::new(column, line)
                                            };

                                            break 'block Some(InputCommand::SelectArea {
                                                cursor,
                                                anchor,
                                            });
                                        }
                                    }
                                }
                            }

                            None
                        }
                        _ => None,
                    },
                    Event::Paste(text) => Some(InputCommand::Insert(text)),
                    _ => None,
                }
            };

            if let Some(input) = input {
                match input {
                    InputCommand::GrowPane => {
                        self.panes.grow_current(self.buffer_area);
                    }
                    InputCommand::ShrinkPane => {
                        self.panes.shrink_current(self.buffer_area);
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
                        self.file_finder = None;
                        self.buffer_finder = None;
                        self.palette.focus(
                            self.get_search_prompt(),
                            "search",
                            CompleterContext::new(&self.themes),
                        );
                    }
                    InputCommand::CaseInsensitive => {
                        self.config.case_insensitive_search = !self.config.case_insensitive_search;
                        if let Some("search") = self.palette.mode() {
                            self.palette.update_prompt(self.get_search_prompt());
                        }
                    }
                    InputCommand::Escape
                        if self.file_finder.is_some() | self.buffer_finder.is_some() =>
                    {
                        self.file_finder = None;
                        self.buffer_finder = None;
                    }
                    InputCommand::OpenFileBrowser => self.open_file_picker(),
                    InputCommand::OpenBufferBrowser => self.open_buffer_picker(),
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
                                self.panes.replace_current(PaneKind::Buffer(choice.id));
                            }
                        } else {
                            use crate::ferrite_core::buffer::input::Response;
                            let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane() else {
                                return;
                            };
                            match self.buffers[buffer_id].handle_input(input) {
                                Ok(response) => match response {
                                    Response::Written(name, bytes_written) => {
                                        self.palette.set_msg(format!(
                                            "`{name}` written: {}",
                                            format_byte_size(bytes_written)
                                        ))
                                    }
                                    Response::None => (),
                                },
                                Err(err) => self.palette.set_error(err),
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn handle_app_event(
        &mut self,
        proxy: &TuiEventLoopProxy,
        event: TuiAppEvent,
        control_flow: &mut TuiEventLoopControlFlow,
    ) {
        match event {
            TuiAppEvent::ShellResult(result) => match result {
                Ok(buffer) => {
                    self.insert_buffer(buffer, true);
                }
                Err(e) => self.palette.set_error(e),
            },
            TuiAppEvent::PaletteEvent { mode, content } => match mode.as_str() {
                "command" => {
                    use cmd::Command;
                    self.palette.reset();
                    match cmd_parser::parse_cmd(&content) {
                        Ok(cmd) => match cmd {
                            Command::Split(direction) => {
                                let buffer_id = self.insert_buffer(Buffer::new(), false).0;
                                self.panes.split(PaneKind::Buffer(buffer_id), direction);
                                self.open_file_picker();
                            }
                            Command::Shell(args) => {
                                let thread_proxy = proxy.clone();
                                thread::spawn(move || {
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

                                    let mut popen = match exec.popen() {
                                        Ok(popen) => popen,
                                        Err(e) => {
                                            thread_proxy
                                                .send(TuiAppEvent::ShellResult(Err(e.into())));
                                            return;
                                        }
                                    };
                                    let (stdout, stderr) = match popen.communicate_bytes(None) {
                                        Ok(out) => out,
                                        Err(e) => {
                                            thread_proxy
                                                .send(TuiAppEvent::ShellResult(Err(e.into())));
                                            return;
                                        }
                                    };
                                    let status = match popen.wait() {
                                        Ok(status) => status,
                                        Err(e) => {
                                            thread_proxy
                                                .send(TuiAppEvent::ShellResult(Err(e.into())));
                                            return;
                                        }
                                    };
                                    if !status.success() {
                                        thread_proxy.send(TuiAppEvent::ShellResult(Err(
                                            anyhow::Error::msg(
                                                String::from_utf8_lossy(&stderr.unwrap())
                                                    .to_string(),
                                            ),
                                        )));
                                        return;
                                    }
                                    let buffer = match Buffer::from_bytes(
                                        &stdout.unwrap(),
                                        thread_proxy.clone(),
                                    ) {
                                        Ok(buffer) => buffer,
                                        Err(e) => {
                                            thread_proxy
                                                .send(TuiAppEvent::ShellResult(Err(e.into())));
                                            return;
                                        }
                                    };

                                    thread_proxy.send(TuiAppEvent::ShellResult(Ok(buffer)));
                                });
                            }
                            Command::Delete => {
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };

                                match self.buffers[buffer_id].move_to_trash() {
                                    Ok(true) => {
                                        let path = self.buffers[buffer_id].file().unwrap();
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
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };

                                match self.buffers[buffer_id].save(path) {
                                    Ok(bytes_written) => self.palette.set_msg(format!(
                                        "`{}` written: {}",
                                        self.buffers[buffer_id].name().unwrap_or_default(),
                                        format_byte_size(bytes_written)
                                    )),
                                    Err(err) => self.palette.set_msg(err.to_string()),
                                }
                            }
                            Command::Language(language) => {
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };
                                match language {
                                    Some(language) => {
                                        if let Err(err) = self.buffers[buffer_id]
                                            .set_langauge(&language, self.proxy.clone())
                                        {
                                            self.palette.set_error(err);
                                        }
                                    }
                                    None => self
                                        .palette
                                        .set_msg(self.buffers[buffer_id].language_name()),
                                }
                            }
                            Command::Encoding(encoding) => {
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };
                                match encoding {
                                Some(encoding) => {
                                    match get_encoding(&encoding) {
                                        Some(encoding) => self.buffers[buffer_id].encoding = encoding,
                                        None => self.palette.set_error("unknown encoding, these encodings are supported: https://docs.rs/encoding_rs/latest/encoding_rs"),
                                    }
                                }
                                None => self
                                .palette
                                .set_msg(self.buffers[buffer_id].encoding.name()),
                            }
                            }
                            Command::Indent(indent) => {
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };
                                match indent {
                                    Some(indent) => {
                                        if let Ok(spaces) = indent.parse::<NonZeroUsize>() {
                                            self.buffers[buffer_id].indent =
                                                Indentation::Spaces(spaces);
                                        } else if indent == "tabs" {
                                            self.buffers[buffer_id].indent =
                                                Indentation::Tabs(NonZeroUsize::new(1).unwrap());
                                        } else {
                                            self.palette.set_error(
                                                "Indentation must be a number or `tabs`",
                                            );
                                        }
                                    }
                                    None => match self.buffers[buffer_id].indent {
                                        Indentation::Tabs(_) => self.palette.set_msg("tabs"),
                                        Indentation::Spaces(amount) => {
                                            self.palette.set_msg(format!("{} space(s)", amount))
                                        }
                                    },
                                }
                            }
                            Command::LineEnding(line_ending) => {
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };
                                match line_ending {
                                    Some(line_ending) => {
                                        self.buffers[buffer_id].line_ending = line_ending
                                    }
                                    None => self.palette.set_msg(
                                        match self.buffers[buffer_id].line_ending {
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
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };
                                if self.buffers[buffer_id].is_dirty() {
                                    self.palette.set_prompt(
                                        "The buffer is unsaved are you sure you want to reload?",
                                        ('y', PalettePromptEvent::Reload),
                                        ('n', PalettePromptEvent::Nop),
                                    );
                                } else if let Err(err) = self.buffers[buffer_id].reload() {
                                    self.palette.set_error(err)
                                };
                            }
                            Command::Goto(line) => {
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };
                                self.buffers[buffer_id].goto(line);
                            }
                            Command::Case(case) => {
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };
                                self.buffers[buffer_id].transform_case(case);
                            }
                            Command::Quit => self.quit(control_flow),
                            Command::ForceQuit => *control_flow = TuiEventLoopControlFlow::Exit,
                            Command::Logger => todo!(),
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
                            Command::ForceClose => self.force_close_current_buffer(),
                            Command::Close => self.close_current_buffer(),
                            Command::Paste => {
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };
                                if let Err(err) =
                                    self.buffers[buffer_id].handle_input(InputCommand::Paste)
                                {
                                    self.palette.set_error(err);
                                }
                            }
                            Command::Copy => {
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };
                                if let Err(err) =
                                    self.buffers[buffer_id].handle_input(InputCommand::Copy)
                                {
                                    self.palette.set_error(err);
                                }
                            }
                            Command::RevertBuffer => {
                                let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane()
                                else {
                                    return;
                                };
                                let _ = self.buffers[buffer_id]
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
                        let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane() else {
                            return;
                        };
                        self.buffers[buffer_id].goto(line);
                    }
                }
                "search" => {
                    let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane() else {
                        return;
                    };
                    self.buffers[buffer_id].start_search(
                        self.proxy.clone(),
                        content,
                        self.config.case_insensitive_search,
                    );
                    self.palette.unfocus();
                }
                _ => (),
            },
            TuiAppEvent::PromptEvent(event) => match event {
                PalettePromptEvent::Nop => (),
                PalettePromptEvent::Reload => {
                    let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane() else {
                        return;
                    };
                    if let Err(err) = self.buffers[buffer_id].reload() {
                        self.palette.set_error(err);
                    }
                }
                PalettePromptEvent::Quit => *control_flow = TuiEventLoopControlFlow::Exit,
                PalettePromptEvent::CloseCurrent => self.force_close_current_buffer(),
            },
        }
    }

    pub fn format_selection_current_buffer(&mut self) {
        let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane() else {
            return;
        };
        let buffer_lang = self.buffers[buffer_id].language_name();
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

        if let Err(err) = self.buffers[buffer_id].format_selection(fmt) {
            // FIXME make error able to display more then one line
            self.palette.set_error(err);
        }
    }

    pub fn format_current_buffer(&mut self) {
        if let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane() {
            let buffer_lang = self.buffers[buffer_id].language_name();
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

            if let Err(err) = self.buffers[buffer_id].format(fmt) {
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

        match self.buffers.iter().find(|(_, buffer)| {
            buffer
                .file()
                .map(|path| dunce::canonicalize(path).unwrap())
                .as_deref()
                == Some(&real_path)
        }) {
            Some((id, _)) => self.panes.replace_current(PaneKind::Buffer(id)),
            None => match Buffer::from_file(path, self.proxy.clone()) {
                Ok(buffer) => {
                    if let PaneKind::Buffer(buffer_id) = self.panes.get_current_pane() {
                        let current_buf = self.buffers.get_mut(buffer_id).unwrap();
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

    pub fn quit(&mut self, control_flow: &mut TuiEventLoopControlFlow) {
        let unsaved: Vec<_> = self
            .buffers
            .iter()
            .filter_map(|(_, buffer)| {
                if buffer.is_dirty() {
                    Some(buffer.name().unwrap_or_else(|| "scratch".into()))
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
            *control_flow = TuiEventLoopControlFlow::Exit;
        }
    }

    pub fn open_buffer_picker(&mut self) {
        self.palette.reset();
        self.file_finder = None;
        let mut scratch_buffer_number = 1;
        let buffers: Vec<_> = self
            .buffers
            .iter()
            .map(|(id, buffer)| BufferItem {
                id,
                dirty: buffer.is_dirty(),
                name: buffer
                    .file()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_else(|| {
                        scratch_buffer_number += 1;
                        format!("[Scratch] {scratch_buffer_number}")
                    }),
            })
            .collect();

        self.buffer_finder = Some(SearchBuffer::new(
            BufferFindProvider(buffers.into()),
            self.proxy.clone(),
        ));
    }

    pub fn open_file_picker(&mut self) {
        self.palette.reset();
        self.buffer_finder = None;
        self.file_finder = Some(SearchBuffer::new(
            FileFindProvider(self.file_daemon.subscribe()),
            self.proxy.clone(),
        ));
    }

    pub fn open_config(&mut self) {
        match &self.config_path {
            Some(path) => self.open_file(path.clone()),
            None => self.palette.set_error("Could not locate the config file"),
        }
    }

    pub fn close_current_buffer(&mut self) {
        // TODO make this close any buffer
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
        }
    }

    pub fn force_close_current_buffer(&mut self) {
        // TODO make this close any buffer
        if let Some(buffer_id) = self.get_current_buffer_id() {
            if self.panes.len() > 1 {
                self.panes.remove_pane(PaneKind::Buffer(buffer_id));
                self.buffers.remove(buffer_id);
            } else if self.buffers.len() > 1 {
                self.buffers.remove(buffer_id);
                let (buffer_id, _) = self.buffers.iter().next().unwrap();
                self.panes.replace_current(PaneKind::Buffer(buffer_id));
            } else {
                self.buffers[buffer_id] = Buffer::new();
            }
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
        match self.panes.get_current_pane() {
            PaneKind::Buffer(id) => Some(id),
            _ => None,
        }
    }

    pub fn get_current_buffer(&self) -> Option<&Buffer> {
        let PaneKind::Buffer(buffer) = self.panes.get_current_pane() else {
            return None;
        };

        self.buffers.get(buffer)
    }

    pub fn _get_current_buffer_mut(&mut self) -> Option<&mut Buffer> {
        let PaneKind::Buffer(buffer) = self.panes.get_current_pane() else {
            return None;
        };

        self.buffers.get_mut(buffer)
    }

    pub fn insert_buffer(&mut self, buffer: Buffer, make_current: bool) -> (usize, &mut Buffer) {
        let buffer_id = self.buffers.insert(buffer);
        if make_current {
            self.panes.replace_current(PaneKind::Buffer(buffer_id));
        }
        (buffer_id, &mut self.buffers[buffer_id])
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            event::DisableMouseCapture,
            event::DisableBracketedPaste,
            terminal::LeaveAlternateScreen,
        );
        let _ = self.terminal.show_cursor();
        clipboard::uninit();
    }
}
