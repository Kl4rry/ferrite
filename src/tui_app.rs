use std::{
    collections::HashMap,
    fs,
    io::{self, Stdout},
    path::{Path, PathBuf},
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind, MouseButton, MouseEventKind},
    execute, terminal,
};
use slab::Slab;
use tui::layout::{Margin, Rect};
use utility::{line_ending, point::Point};

use self::{
    event_loop::{TuiAppEvent, TuiEvent, TuiEventLoop, TuiEventLoopControlFlow, TuiEventLoopProxy},
    input::{get_default_mappings, Exclusiveness, Mapping},
    widgets::{
        background_widget::BackgroundWidget,
        editor_widget::{lines_to_left_offset, EditorWidget},
        palette_widget::CmdPaletteWidget,
        search_widget::SearchWidget,
        splash::SplashWidget,
    },
};
use crate::{
    core::{
        buffer::{encoding::get_encoding, Buffer},
        config::{Config, ConfigWatcher},
        git::branch::BranchWatcher,
        indent::Indentation,
        language,
        palette::{cmd, cmd_parser, CommandPalette, PalettePromptEvent},
        search_buffer::{
            buffer_find::{BufferFindProvider, BufferItem},
            file_find::FileFindProvider,
            SearchBuffer,
        },
        theme::EditorTheme,
    },
    tui_app::input::InputCommand,
    Args,
};

pub mod event_loop;
pub mod input;
mod widgets;

pub struct TuiApp {
    terminal: tui::Terminal<tui::backend::CrosstermBackend<Stdout>>,
    buffers: Slab<Buffer>,
    current_buffer_id: usize,
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
}

impl TuiApp {
    pub fn new(args: &Args, proxy: TuiEventLoopProxy) -> Result<Self> {
        let mut palette = CommandPalette::new(proxy.clone());
        let config_path = Config::get_default_location().ok();
        let mut config = match Config::load_or_create_default() {
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

        let themes = EditorTheme::load_themes();
        if !themes.contains_key(&config.theme) {
            config.theme = "default".into();
        }
        // The theme needs to be set before the buffer is created
        language::update_theme(&themes[&config.theme]);

        let mut file_finder = None;
        let mut buffer = match &args.file {
            Some(file) if file.is_dir() => {
                std::env::set_current_dir(file)?;
                file_finder = Some(SearchBuffer::new(
                    FileFindProvider(std::env::current_dir().unwrap_or(PathBuf::from("/"))),
                    proxy.clone(),
                ));
                Buffer::new()
            }
            Some(file) => match Buffer::from_file(file, proxy.clone()) {
                Ok(buffer) => buffer,
                Err(err) => match err.kind() {
                    io::ErrorKind::NotFound => Buffer::with_path(file),
                    _ => Buffer::new(),
                },
            },
            None => Buffer::new(),
        };

        let (width, height) = crossterm::terminal::size()?;
        buffer.set_view_lines(height.saturating_sub(2).into());
        buffer.set_view_columns(width.into());
        buffer.goto(args.line as i64);
        if let Some(language) = &args.language {
            buffer.set_langauge(language, proxy.clone())?;
        }

        let mut buffers = Slab::new();
        let current_buffer_id = buffers.insert(buffer);

        Ok(Self {
            terminal: tui::Terminal::new(tui::backend::CrosstermBackend::new(std::io::stdout()))?,
            buffers,
            current_buffer_id,
            themes,
            config,
            config_path,
            config_watcher,
            palette,
            file_finder,
            buffer_finder: None,
            key_mappings: get_default_mappings(),
            branch_watcher: BranchWatcher::new(proxy.clone())?,
            proxy,
            drag_start: None,
        })
    }

    pub fn new_buffer_with_text(&mut self, text: &str) -> &mut Buffer {
        let mut buffer = Buffer::new();
        buffer.set_text(text);
        let id = self.buffers.insert(buffer);
        self.current_buffer_id = id;
        &mut self.buffers[self.current_buffer_id]
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

        event_loop.run(|proxy, event, control_flow| self.handle_event(proxy, event, control_flow));

        terminal::disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            terminal::LeaveAlternateScreen,
            event::DisableMouseCapture,
            event::DisableBracketedPaste,
        )?;
        self.terminal.show_cursor()?;

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
                f.render_stateful_widget(
                    EditorWidget::new(
                        theme,
                        &self.config,
                        !self.palette.has_focus() && self.file_finder.is_none(),
                        self.branch_watcher.current_branch(),
                    ),
                    editor_size,
                    &mut self.buffers[self.current_buffer_id],
                );

                {
                    let buffer = &mut self.buffers[self.current_buffer_id];
                    if buffer.rope().len_bytes() == 0
                        && !buffer.is_dirty()
                        && buffer.file().is_none()
                        && self.buffers.len() == 1
                    {
                        f.render_widget(SplashWidget::new(theme), editor_size);
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

                let palette_size = Rect::new(size.x, size.height - 1, size.width, 1);
                f.render_stateful_widget(
                    CmdPaletteWidget::new(theme, self.palette.has_focus()),
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
            let input = match event {
                Event::Key(event) => {
                    if event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat {
                        log::debug!("{:?}", event);
                        input::get_command_from_input(
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
                    MouseEventKind::Down(MouseButton::Left) => {
                        self.drag_start =
                            Some(Point::new(event.column as usize, event.row as usize));

                        let buffer = &self.buffers[self.current_buffer_id];
                        if (event.row as usize) < buffer.get_view_lines()
                            && (event.column as usize) < buffer.get_view_columns()
                        {
                            let (_, left_offset) = lines_to_left_offset(buffer.len_lines());
                            let column = (event.column as usize).saturating_sub(left_offset)
                                + buffer.col_pos();
                            let line = event.row as usize + buffer.line_pos();
                            Some(InputCommand::SetCursorPos(column, line))
                        } else {
                            // TODO handle other clicks then in current buffer
                            None
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        self.drag_start = None;
                        None
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        // TODO maybe scroll more of the buffer into view when going outside its bounds
                        if let Some(Point { line, column }) = self.drag_start {
                            let buffer = &mut self.buffers[self.current_buffer_id];
                            let (_, left_offset) = lines_to_left_offset(buffer.len_lines());

                            let anchor = {
                                let column = column.saturating_sub(left_offset) + buffer.col_pos();
                                let line = line + buffer.line_pos();
                                Point::new(column, line)
                            };

                            let cursor = {
                                let column = (event.column as usize).saturating_sub(left_offset)
                                    + buffer.col_pos();
                                let line = event.row as usize + buffer.line_pos();
                                Point::new(column, line)
                            };

                            Some(InputCommand::SelectArea { cursor, anchor })
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
                Event::Paste(text) => Some(InputCommand::Insert(text)),
                _ => None,
            };

            if let Some(input) = input {
                match input {
                    InputCommand::Quit => {
                        self.quit(control_flow);
                    }
                    InputCommand::Escape if self.palette.has_focus() => {
                        self.palette.reset();
                    }
                    InputCommand::FocusPalette if !self.palette.has_focus() => {
                        self.file_finder = None;
                        self.buffer_finder = None;
                        self.palette.focus("> ", "command");
                    }
                    InputCommand::PromptGoto => {
                        self.file_finder = None;
                        self.buffer_finder = None;
                        self.palette.focus("goto: ", "goto");
                    }
                    InputCommand::FileSearch => {
                        self.file_finder = None;
                        self.buffer_finder = None;
                        self.palette.focus("search: ", "search");
                    }
                    InputCommand::Escape
                        if self.file_finder.is_some() | self.buffer_finder.is_some() =>
                    {
                        self.file_finder = None;
                        self.buffer_finder = None;
                    }
                    InputCommand::OpenFileBrowser => self.browse_workspace(),
                    InputCommand::OpenBufferBrowser => self.browse_buffers(),
                    input => {
                        if self.palette.has_focus() {
                            let _ = self.palette.handle_input(input);
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
                                self.current_buffer_id = choice.id;
                            }
                        } else if let Err(err) =
                            self.buffers[self.current_buffer_id].handle_input(input)
                        {
                            self.palette.set_error(err);
                        }
                    }
                }
            }
        }
    }

    pub fn handle_app_event(
        &mut self,
        _proxy: &TuiEventLoopProxy,
        event: TuiAppEvent,
        control_flow: &mut TuiEventLoopControlFlow,
    ) {
        match event {
            event_loop::TuiAppEvent::PaletteEvent { mode, content } => match mode.as_str() {
                "command" => {
                    use cmd::Command;
                    self.palette.reset();
                    match cmd_parser::parse_cmd(&content) {
                        Ok(cmd) => match cmd {
                            Command::OpenFile(path) => self.open_file(path),
                            Command::SaveFile(path) => {
                                if let Err(err) = self.buffers[self.current_buffer_id].save(path) {
                                    self.palette.set_msg(err.to_string())
                                }
                            }
                            Command::Language(language) => match language {
                                Some(language) => {
                                    if let Err(err) = self.buffers[self.current_buffer_id].set_langauge(&language, self.proxy.clone()) {
                                        self.palette.set_error(err);
                                    }
                                }
                                None => self
                                .palette
                                .set_msg(self.buffers[self.current_buffer_id].language_name()),
                            },
                            Command::Encoding(encoding) => match encoding {
                                Some(encoding) => {
                                    match get_encoding(&encoding) {
                                        Some(encoding) => self.buffers[self.current_buffer_id].encoding = encoding,
                                        None => self.palette.set_error("unknown encoding, these encodings are supported: https://docs.rs/encoding_rs/latest/encoding_rs"),
                                    }
                                }
                                None => self
                                .palette
                                .set_msg(self.buffers[self.current_buffer_id].encoding.name()),
                            },
                            Command::Indent => match self.buffers[self.current_buffer_id].indent {
                                Indentation::Tabs(_) => self.palette.set_msg("tabs"),
                                Indentation::Spaces(amount) => {
                                    self.palette.set_msg(format!("{} space(s)", amount))
                                }
                            },
                            Command::LineEnding(line_ending) => {
                                match line_ending {
                                    Some(line_ending) => self.buffers[self.current_buffer_id].line_ending = line_ending,
                                    None => self.palette.set_msg(match self.buffers[self.current_buffer_id].line_ending {
                                        line_ending::LineEnding::Crlf => "crlf",
                                        line_ending::LineEnding::LF => "lf",
                                        _ => unreachable!(),
                                    }),
                                }
                            }
                            Command::New => self.current_buffer_id = self.buffers.insert(Buffer::new()),
                            Command::Reload => {
                                self.palette.set_prompt(
                                    "The buffer is unsaved are you sure you want to reload?",
                                    ('y', PalettePromptEvent::Reload),
                                    ('n', PalettePromptEvent::Nop),
                                );
                                if let Err(err) = self.buffers[self.current_buffer_id].reload() {
                                    self.palette.set_error(err)
                                };
                            }
                            Command::Goto(line) => self.buffers[self.current_buffer_id].goto(line),
                            Command::Quit => self.quit(control_flow),
                            Command::ForceQuit => *control_flow = TuiEventLoopControlFlow::Exit,
                            Command::Logger => todo!(),
                            Command::Theme(name) => match name {
                                Some(name) => {
                                    if self.themes.contains_key(&name) {
                                        self.config.theme = name;
                                        language::update_theme(&self.themes[&self.config.theme]);
                                    } else {
                                        self.palette.set_error("Theme not found");
                                    }
                                }
                                None => {
                                    self.palette.set_msg(&self.config.theme);
                                }
                            },
                            Command::BrowseBuffers => self.browse_buffers(),
                            Command::BrowseWorkspace => self.browse_workspace(),
                            Command::OpenConfig => self.open_config(),
                            Command::ForceClose => self.force_close_current_buffer(),
                            Command::Close => self.close_current_buffer(),
                        },
                        Err(err) => self.palette.set_error(err),
                    }
                }
                "goto" => {
                    self.palette.reset();
                    if let Ok(line) = content.trim().parse::<i64>() {
                        self.buffers[self.current_buffer_id].goto(line);
                    }
                }
                "search" => {
                    self.buffers[self.current_buffer_id].start_search(self.proxy.clone(), content);
                    self.palette.unfocus();
                }
                _ => (),
            },
            TuiAppEvent::PromptEvent(event) => match event {
                PalettePromptEvent::Nop => (),
                PalettePromptEvent::Reload => {
                    if let Err(err) = self.buffers[self.current_buffer_id].reload() {
                        self.palette.set_error(err);
                    }
                }
                PalettePromptEvent::Quit => *control_flow = TuiEventLoopControlFlow::Exit,
                PalettePromptEvent::CloseCurrent => self.force_close_current_buffer(),
            },
        }
    }

    pub fn open_file(&mut self, path: impl AsRef<Path>) {
        let real_path = match fs::canonicalize(&path) {
            Ok(path) => path,
            Err(err) => {
                self.palette.set_error(err);
                return;
            }
        };

        match self.buffers.iter().find(|(_, buffer)| {
            buffer
                .file()
                .map(|path| fs::canonicalize(path).unwrap())
                .as_deref()
                == Some(&real_path)
        }) {
            Some((id, _)) => self.current_buffer_id = id,
            None => match Buffer::from_file(path, self.proxy.clone()) {
                Ok(buffer) => {
                    let current_buf = self.buffers.get_mut(self.current_buffer_id).unwrap();
                    if !current_buf.is_dirty() && current_buf.rope().len_bytes() == 0 {
                        *current_buf = buffer;
                    } else {
                        self.current_buffer_id = self.buffers.insert(buffer);
                    }
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

        if unsaved.is_empty() {
            *control_flow = TuiEventLoopControlFlow::Exit;
        } else {
            self.palette.set_prompt(
                format!(
                    "You have {} unsaved buffer(s): {:?}, Are you sure you want to exit?",
                    unsaved.len(),
                    unsaved
                ),
                ('y', PalettePromptEvent::Quit),
                ('n', PalettePromptEvent::Nop),
            );
        }
    }

    pub fn browse_buffers(&mut self) {
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
            BufferFindProvider(buffers),
            self.proxy.clone(),
        ));
    }

    pub fn browse_workspace(&mut self) {
        self.palette.reset();
        self.buffer_finder = None;
        self.file_finder = Some(SearchBuffer::new(
            FileFindProvider(std::env::current_dir().unwrap_or(PathBuf::from("/"))),
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
        if self.buffers[self.current_buffer_id].is_dirty() {
            self.palette.set_prompt(
                "Current buffer has unsaved changes are you sure you want to close it?",
                ('y', PalettePromptEvent::CloseCurrent),
                ('n', PalettePromptEvent::Nop),
            );
        }
    }

    pub fn force_close_current_buffer(&mut self) {
        self.buffers.remove(self.current_buffer_id);
        self.current_buffer_id = match self.buffers.iter().next() {
            Some((id, _)) => id,
            None => self.buffers.insert(Buffer::new()),
        }
    }
}
