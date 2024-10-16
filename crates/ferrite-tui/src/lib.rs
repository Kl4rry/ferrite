use std::{
    io::{self, IsTerminal, Read, Stdout},
    sync::mpsc,
    time::Instant,
};

use anyhow::{bail, Result};
use crossterm::{
    event::{
        self, Event, KeyEventKind, KeyboardEnhancementFlags, MouseButton, MouseEventKind,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute, terminal,
};
use ferrite_cli::Args;
use ferrite_core::{
    buffer::Buffer,
    clipboard,
    cmd::Cmd,
    engine::Engine,
    event_loop_proxy::EventLoopControlFlow,
    keymap::{self},
    layout::panes::PaneKind,
    logger::{self, LogMessage},
    picker::{buffer_picker::BufferItem, global_search_picker::GlobalSearchMatch},
};
use ferrite_utility::point::Point;
use glue::{ferrite_to_tui_rect, tui_to_ferrite_rect};
use tui::layout::{Margin, Position, Rect};
use widgets::{choord_widget::ChoordWidget, logger_widget::LoggerWidget};

use self::{
    event_loop::{TuiEvent, TuiEventLoop, TuiEventLoopProxy},
    widgets::{
        background_widget::BackgroundWidget,
        editor_widget::{lines_to_left_offset, EditorWidget},
        palette_widget::CmdPaletteWidget,
        picker_widget::PickerWidget,
        splash::SplashWidget,
    },
};
use crate::glue::{convert_keycode, convert_modifier};

pub mod event_loop;
#[rustfmt::skip]
pub mod glue;
pub mod rect_ext;
mod widgets;

pub fn run(args: &Args, recv: mpsc::Receiver<LogMessage>) -> Result<()> {
    let event_loop = TuiEventLoop::new();
    let mut tui_app = TuiApp::new(args, event_loop.create_proxy(), recv)?;
    if !io::stdin().is_terminal() {
        let mut stdin = io::stdin().lock();
        let mut bytes = Vec::new();
        stdin.read_to_end(&mut bytes)?;
        let mut buffer = Buffer::from_bytes(&bytes)?;
        let view_id = buffer.create_view();
        buffer.goto(view_id, args.line as i64);
        tui_app.engine.insert_buffer(buffer, view_id, true);
    }

    if !io::stdout().is_terminal() {
        bail!("stdout must is not a tty");
    }

    tui_app.run(event_loop);
    Ok(())
}

pub struct TuiApp {
    terminal: tui::Terminal<tui::backend::CrosstermBackend<Stdout>>,
    buffer_area: Rect,
    drag_start: Option<Point<usize>>,
    engine: Engine,
    keyboard_enhancement: bool,
}

impl TuiApp {
    pub fn new(
        args: &Args,
        proxy: TuiEventLoopProxy,
        recv: mpsc::Receiver<LogMessage>,
    ) -> Result<Self> {
        let engine = Engine::new(args, Box::new(proxy), recv)?;

        logger::set_proxy(engine.proxy.dup());

        let (width, height) = crossterm::terminal::size()?;

        if !io::stdout().is_terminal() {
            bail!("Stdout must be a terminal");
        }

        Ok(Self {
            terminal: tui::Terminal::new(tui::backend::CrosstermBackend::new(std::io::stdout()))?,
            buffer_area: Rect {
                x: 0,
                y: 0,
                width,
                height: height.saturating_sub(2),
            },
            drag_start: None,
            engine,
            keyboard_enhancement: false,
        })
    }

    pub fn run(mut self, event_loop: TuiEventLoop) {
        tracing::info!("Starting tui app");
        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode().unwrap();
        execute!(
            stdout,
            event::EnableBracketedPaste,
            terminal::EnterAlternateScreen,
            terminal::Clear(terminal::ClearType::Purge),
            event::EnableMouseCapture,
        )
        .unwrap();

        if terminal::supports_keyboard_enhancement().unwrap() {
            execute!(
                stdout,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            )
            .unwrap();
        }

        // Reset terminal to non raw mode on panic
        {
            std::panic::set_hook(Box::new(move |info| {
                let _ = execute!(
                    io::stdout(),
                    event::DisableMouseCapture,
                    event::DisableBracketedPaste,
                    terminal::LeaveAlternateScreen,
                );
                _ = terminal::disable_raw_mode();
                println!();
                let backtrace = std::backtrace::Backtrace::force_capture();
                let panic_info = format!("{backtrace}\n{info}");
                let _ = std::fs::write("panic.txt", &panic_info);
                println!("{}", panic_info);
            }));
        }

        event_loop.run(|proxy, event, control_flow| self.handle_event(proxy, event, control_flow));
    }

    pub fn handle_event(
        &mut self,
        proxy: &TuiEventLoopProxy,
        event: TuiEvent,
        control_flow: &mut EventLoopControlFlow,
    ) {
        match event {
            event_loop::TuiEvent::StartOfEvents => self.engine.start_of_events = Instant::now(),
            event_loop::TuiEvent::Crossterm(event) => {
                self.handle_crossterm_event(proxy, event, control_flow)
            }
            event_loop::TuiEvent::AppEvent(event) => {
                self.engine.handle_app_event(event, control_flow)
            }
            event_loop::TuiEvent::Render => {
                self.engine.do_polling(control_flow);
                self.render();
                self.engine.last_render_time =
                    Instant::now().duration_since(self.engine.start_of_events);
            }
        }
    }

    pub fn render(&mut self) {
        if self.engine.force_redraw {
            self.engine.force_redraw = false;
            let _ = self.terminal.clear();
        }
        self.terminal
            .draw(|f| {
                let theme = &self.engine.themes[&self.engine.config.editor.theme];
                f.render_widget(BackgroundWidget::new(theme), f.area());
                let size = f.area();
                let editor_size = Rect::new(
                    size.x,
                    size.y,
                    size.width,
                    size.height
                        .saturating_sub(self.engine.palette.height() as u16),
                );

                self.buffer_area = editor_size;
                let current_pane = self.engine.workspace.panes.get_current_pane();
                for (pane, pane_rect) in self
                    .engine
                    .workspace
                    .panes
                    .get_pane_bounds(tui_to_ferrite_rect(editor_size))
                {
                    match pane {
                        PaneKind::Buffer(buffer_id, view_id) => {
                            f.render_stateful_widget(
                                EditorWidget::new(
                                    theme,
                                    &self.engine.config.editor,
                                    view_id,
                                    !self.engine.palette.has_focus()
                                        && self.engine.file_picker.is_none()
                                        && current_pane == pane,
                                    self.engine.branch_watcher.current_branch(),
                                    self.engine.spinner.current(),
                                ),
                                ferrite_to_tui_rect(pane_rect),
                                &mut self.engine.workspace.buffers[buffer_id],
                            );

                            if self.engine.config.editor.show_splash
                                && self.engine.workspace.panes.num_panes() == 1
                            {
                                let buffer = &mut self.engine.workspace.buffers[buffer_id];
                                if buffer.len_bytes() == 0
                                    && !buffer.is_dirty()
                                    && buffer.file().is_none()
                                    && self.engine.workspace.buffers.len() == 1
                                {
                                    f.render_widget(
                                        SplashWidget::new(theme),
                                        ferrite_to_tui_rect(pane_rect),
                                    );
                                }
                            }
                        }
                        PaneKind::Logger => {
                            let has_focus = !self.engine.palette.has_focus()
                                && self.engine.file_picker.is_none()
                                && current_pane == pane;
                            f.render_stateful_widget(
                                LoggerWidget::new(theme, self.engine.last_render_time, has_focus),
                                ferrite_to_tui_rect(pane_rect),
                                &mut self.engine.logger_state,
                            );
                        }
                    }
                }

                if let Some(file_picker) = &mut self.engine.file_picker {
                    let size = size.inner(Margin {
                        horizontal: 5,
                        vertical: 2,
                    });
                    f.render_stateful_widget(
                        PickerWidget::new(theme, &self.engine.config.editor, "Open file"),
                        size,
                        file_picker,
                    );
                }

                if let Some(buffer_picker) = &mut self.engine.buffer_picker {
                    let size = size.inner(Margin {
                        horizontal: 5,
                        vertical: 2,
                    });
                    f.render_stateful_widget(
                        PickerWidget::<BufferItem>::new(
                            theme,
                            &self.engine.config.editor,
                            "Open buffer",
                        ),
                        size,
                        buffer_picker,
                    );
                }

                if let Some(global_search_picker) = &mut self.engine.global_search_picker {
                    let size = size.inner(Margin {
                        horizontal: 5,
                        vertical: 2,
                    });
                    f.render_stateful_widget(
                        PickerWidget::<GlobalSearchMatch>::new(
                            theme,
                            &self.engine.config.editor,
                            "Matches",
                        ),
                        size,
                        global_search_picker,
                    );
                }

                let palette_size = Rect::new(
                    size.left(),
                    size.bottom()
                        .saturating_sub(self.engine.palette.height() as u16),
                    size.width,
                    (self.engine.palette.height() as u16).min(size.height),
                );
                f.render_stateful_widget(
                    CmdPaletteWidget::new(theme, self.engine.palette.has_focus(), size),
                    palette_size,
                    &mut self.engine.palette,
                );

                if self.engine.choord.is_some() {
                    let choord_widget =
                        ChoordWidget::new(theme, self.engine.get_current_keymappings());
                    f.render_widget(choord_widget, size);
                }
            })
            .unwrap();
    }

    pub fn handle_crossterm_event(
        &mut self,
        _proxy: &TuiEventLoopProxy,
        event: event::Event,
        control_flow: &mut EventLoopControlFlow,
    ) {
        {
            let input = 'block: {
                match event {
                    Event::Key(event) => {
                        if event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat {
                            tracing::trace!("{:?}", event);
                            keymap::get_command_from_input(
                                convert_keycode(event.code),
                                convert_modifier(event.modifiers),
                                self.engine.get_current_keymappings(),
                            )
                        } else {
                            None
                        }
                    }
                    Event::Mouse(event) => match event.kind {
                        // TODO allow scoll when using cmd palette
                        MouseEventKind::ScrollUp => Some(Cmd::VerticalScroll(-3)),
                        MouseEventKind::ScrollDown => Some(Cmd::VerticalScroll(3)),
                        MouseEventKind::Down(MouseButton::Middle) => {
                            for (pane_kind, pane_rect) in self
                                .engine
                                .workspace
                                .panes
                                .get_pane_bounds(tui_to_ferrite_rect(self.buffer_area))
                            {
                                if ferrite_to_tui_rect(pane_rect)
                                    .contains(Position::new(event.column, event.row))
                                {
                                    self.engine.workspace.panes.make_current(pane_kind);
                                    if let PaneKind::Buffer(buffer_id, view_id) = pane_kind {
                                        let buffer = &self.engine.workspace.buffers[buffer_id];
                                        let (_, left_offset) =
                                            lines_to_left_offset(buffer.len_lines());
                                        let column = ((event.column as usize)
                                            + buffer.col_pos(view_id))
                                        .saturating_sub(pane_rect.x)
                                        .saturating_sub(left_offset);
                                        let line = (event.row as usize + buffer.line_pos(view_id))
                                            .saturating_sub(pane_rect.y);
                                        break 'block Some(Cmd::PastePrimary(column, line));
                                    }
                                }
                            }

                            None
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            for (pane_kind, pane_rect) in self
                                .engine
                                .workspace
                                .panes
                                .get_pane_bounds(tui_to_ferrite_rect(self.buffer_area))
                            {
                                if ferrite_to_tui_rect(pane_rect)
                                    .contains(Position::new(event.column, event.row))
                                {
                                    self.engine.workspace.panes.make_current(pane_kind);
                                    if let PaneKind::Buffer(buffer_id, view_id) = pane_kind {
                                        self.drag_start = Some(Point::new(
                                            event.column as usize,
                                            event.row as usize,
                                        ));

                                        let buffer = &self.engine.workspace.buffers[buffer_id];
                                        let (_, left_offset) =
                                            lines_to_left_offset(buffer.len_lines());
                                        let column = ((event.column as usize)
                                            + buffer.col_pos(view_id))
                                        .saturating_sub(pane_rect.x)
                                        .saturating_sub(left_offset);
                                        let line = (event.row as usize + buffer.line_pos(view_id))
                                            .saturating_sub(pane_rect.y);
                                        break 'block Some(Cmd::ClickCell(column, line));
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
                            for (pane_kind, pane_rect) in self
                                .engine
                                .workspace
                                .panes
                                .get_pane_bounds(tui_to_ferrite_rect(self.buffer_area))
                            {
                                if ferrite_to_tui_rect(pane_rect)
                                    .contains(Position::new(event.column, event.row))
                                {
                                    self.engine.workspace.panes.make_current(pane_kind);
                                    if let PaneKind::Buffer(buffer_id, view_id) = pane_kind {
                                        // TODO maybe scroll more of the buffer into view when going outside its bounds
                                        if let Some(Point { line, column }) = self.drag_start {
                                            let buffer =
                                                &mut self.engine.workspace.buffers[buffer_id];
                                            let (_, left_offset) =
                                                lines_to_left_offset(buffer.len_lines());

                                            let anchor = {
                                                let column = (column + buffer.col_pos(view_id))
                                                    .saturating_sub(left_offset)
                                                    .saturating_sub(pane_rect.x);
                                                let line = (line + buffer.line_pos(view_id))
                                                    .saturating_sub(pane_rect.y);
                                                Point::new(column, line)
                                            };

                                            let cursor = {
                                                let column = ((event.column as usize)
                                                    + buffer.col_pos(view_id))
                                                .saturating_sub(left_offset)
                                                .saturating_sub(pane_rect.x);
                                                let line = (event.row as usize
                                                    + buffer.line_pos(view_id))
                                                .saturating_sub(pane_rect.y);
                                                Point::new(column, line)
                                            };

                                            break 'block Some(Cmd::SelectArea { cursor, anchor });
                                        }
                                    }
                                }
                            }

                            None
                        }
                        _ => None,
                    },
                    Event::Paste(text) => Some(Cmd::Insert(text)),
                    _ => None,
                }
            };

            self.engine.buffer_area = tui_to_ferrite_rect(self.buffer_area);
            if let Some(input) = input {
                self.engine.handle_input_command(input, control_flow);
            }
        }
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        if self.keyboard_enhancement {
            let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags,);
        }
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
