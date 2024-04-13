use std::io::{self, IsTerminal, Read, Stdout};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind, MouseButton, MouseEventKind},
    execute, terminal,
};
use ferrite_cli::Args;
use ferrite_core::{
    buffer::Buffer,
    clipboard,
    engine::Engine,
    event_loop_proxy::EventLoopControlFlow,
    keymap::{self, InputCommand},
    panes::PaneKind,
    search_buffer::buffer_find::BufferItem,
};
use ferrite_utility::point::Point;
use glue::{ferrite_to_tui_rect, tui_to_ferrite_rect};
use tui::layout::{Margin, Position, Rect};

use self::{
    event_loop::{TuiEvent, TuiEventLoop, TuiEventLoopProxy},
    widgets::{
        background_widget::BackgroundWidget,
        editor_widget::{lines_to_left_offset, EditorWidget},
        palette_widget::CmdPaletteWidget,
        search_widget::SearchWidget,
        splash::SplashWidget,
    },
};
use crate::glue::{convert_keycode, convert_modifier};

pub mod event_loop;
#[rustfmt::skip]
pub mod glue;
pub mod rect_ext;
mod widgets;

pub fn run(args: &Args) -> Result<()> {
    let event_loop = TuiEventLoop::new();
    let mut tui_app = TuiApp::new(args, event_loop.create_proxy())?;
    if !io::stdin().is_terminal() {
        let mut stdin = io::stdin().lock();
        let mut text = String::new();
        stdin.read_to_string(&mut text)?;
        let buffer = tui_app.new_buffer_with_text(&text);
        let (_, height) = crossterm::terminal::size()?;
        buffer.set_view_lines(height.saturating_sub(2).into());
        buffer.goto(args.line as i64);
    }

    if !io::stdout().is_terminal() {
        return Ok(());
    }

    tui_app.run(event_loop)?;
    Ok(())
}

pub struct TuiApp {
    terminal: tui::Terminal<tui::backend::CrosstermBackend<Stdout>>,
    buffer_area: Rect,
    drag_start: Option<Point<usize>>,
    engine: Engine,
}

impl TuiApp {
    pub fn new(args: &Args, proxy: TuiEventLoopProxy) -> Result<Self> {
        let engine = Engine::new(args, Box::new(proxy))?;

        let (width, height) = crossterm::terminal::size()?;

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
        })
    }

    pub fn new_buffer_with_text(&mut self, text: &str) -> &mut Buffer {
        let mut buffer = Buffer::new();
        buffer.set_text(text);
        self.engine.insert_buffer(buffer, true).1
    }

    pub fn run(mut self, event_loop: TuiEventLoop) -> Result<()> {
        tracing::info!("Starting tui app");
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
        control_flow: &mut EventLoopControlFlow,
    ) {
        match event {
            event_loop::TuiEvent::Crossterm(event) => {
                self.handle_crossterm_event(proxy, event, control_flow)
            }
            event_loop::TuiEvent::AppEvent(event) => {
                self.engine
                    .handle_app_event(Box::new(proxy.clone()), event, control_flow)
            }
            event_loop::TuiEvent::Render => {
                self.engine.do_polling(control_flow);
                self.render();
            }
        }
    }

    pub fn render(&mut self) {
        self.terminal
            .draw(|f| {
                let theme = &self.engine.themes[&self.engine.config.theme];
                f.render_widget(BackgroundWidget::new(theme), f.size());
                let size = f.size();
                let editor_size = Rect::new(size.x, size.y, size.width, size.height - 1);

                self.buffer_area = editor_size;
                let current_pane = self.engine.workspace.panes.get_current_pane();
                for (pane, pane_rect) in self
                    .engine
                    .workspace
                    .panes
                    .get_pane_bounds(tui_to_ferrite_rect(editor_size))
                {
                    if let PaneKind::Buffer(buffer_id) = pane {
                        f.render_stateful_widget(
                            EditorWidget::new(
                                theme,
                                &self.engine.config,
                                !self.engine.palette.has_focus()
                                    && self.engine.file_finder.is_none()
                                    && current_pane == pane,
                                self.engine.branch_watcher.current_branch(),
                                self.engine.spinner.current(),
                            ),
                            ferrite_to_tui_rect(pane_rect),
                            &mut self.engine.workspace.buffers[buffer_id],
                        );

                        if self.engine.config.show_splash
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
                }

                if let Some(file_finder) = &mut self.engine.file_finder {
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

                if let Some(buffer_finder) = &mut self.engine.buffer_finder {
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
                    CmdPaletteWidget::new(theme, self.engine.palette.has_focus(), size),
                    palette_size,
                    &mut self.engine.palette,
                );
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
                                &self.engine.key_mappings,
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
                                    if let PaneKind::Buffer(buffer_id) = pane_kind {
                                        let buffer = &self.engine.workspace.buffers[buffer_id];
                                        let (_, left_offset) =
                                            lines_to_left_offset(buffer.len_lines());
                                        let column = (event.column as usize)
                                            .saturating_sub(left_offset)
                                            + buffer.col_pos().saturating_sub(pane_rect.x);
                                        let line = (event.row as usize + buffer.line_pos())
                                            .saturating_sub(pane_rect.y);
                                        break 'block Some(InputCommand::PastePrimary(
                                            column, line,
                                        ));
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
                                    if let PaneKind::Buffer(buffer_id) = pane_kind {
                                        self.drag_start = Some(Point::new(
                                            event.column as usize,
                                            event.row as usize,
                                        ));

                                        let buffer = &self.engine.workspace.buffers[buffer_id];
                                        let (_, left_offset) =
                                            lines_to_left_offset(buffer.len_lines());
                                        let column = (event.column as usize)
                                            .saturating_sub(left_offset)
                                            + buffer.col_pos();
                                        let line = event.row as usize
                                            + buffer.line_pos().saturating_sub(pane_rect.y);
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
                                    if let PaneKind::Buffer(buffer_id) = pane_kind {
                                        // TODO maybe scroll more of the buffer into view when going outside its bounds
                                        if let Some(Point { line, column }) = self.drag_start {
                                            let buffer =
                                                &mut self.engine.workspace.buffers[buffer_id];
                                            let (_, left_offset) =
                                                lines_to_left_offset(buffer.len_lines());

                                            let anchor = {
                                                let column = column.saturating_sub(left_offset)
                                                    + buffer.col_pos();
                                                let line = line
                                                    + buffer.line_pos().saturating_sub(pane_rect.y);
                                                Point::new(column, line)
                                            };

                                            let cursor = {
                                                let column = (event.column as usize)
                                                    .saturating_sub(left_offset)
                                                    + buffer.col_pos();
                                                let line = event.row as usize
                                                    + buffer.line_pos().saturating_sub(pane_rect.y);
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
                self.engine.handle_input_command(
                    input,
                    control_flow,
                    tui_to_ferrite_rect(self.buffer_area),
                );
            }
        }
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
