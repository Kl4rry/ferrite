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
use event_loop::{TuiEvent, TuiEventLoop, TuiEventLoopProxy};
use ferrite_cli::Args;
use ferrite_core::{
    buffer::Buffer, clipboard, cmd::Cmd, config::editor::CursorType,
    event_loop_proxy::EventLoopControlFlow, keymap, layout::panes::PaneKind, logger::LogMessage,
};
use ferrite_tui::{
    glue::{ferrite_to_tui_rect, tui_to_ferrite_rect},
    widgets::editor_widget::lines_to_left_offset,
    TuiApp,
};
use ferrite_utility::point::Point;
use glue::{convert_keycode, convert_modifier};
use tui::layout::Position;

mod event_loop;
mod glue;

pub fn run(args: &Args, recv: mpsc::Receiver<LogMessage>) -> Result<()> {
    let event_loop = TuiEventLoop::new();
    let backend = tui::backend::CrosstermBackend::new(std::io::stdout());
    let mut tui_app = TuiApp::new(args, event_loop.create_proxy(), backend, recv)?;
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

    let term_app = TermApp {
        tui_app,
        keyboard_enhancement: false,
    };
    term_app.run(event_loop);
    Ok(())
}

pub struct TermApp {
    tui_app: TuiApp<tui::backend::CrosstermBackend<Stdout>>,
    keyboard_enhancement: bool,
}

impl TermApp {
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
            event_loop::TuiEvent::StartOfEvents => {
                self.tui_app.start_of_events();
            }
            event_loop::TuiEvent::Crossterm(event) => {
                self.handle_crossterm_event(proxy, event, control_flow)
            }
            event_loop::TuiEvent::AppEvent(event) => {
                self.tui_app.engine.handle_app_event(event, control_flow)
            }
            event_loop::TuiEvent::Render => {
                self.tui_app.engine.do_polling(control_flow);
                self.tui_app.engine.config.editor.gui.cursor_type = CursorType::Block;
                self.tui_app.render();
                self.tui_app.engine.last_render_time =
                    Instant::now().duration_since(self.tui_app.engine.start_of_events);
            }
        }
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
                                self.tui_app.engine.get_current_keymappings(),
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
                                .tui_app
                                .engine
                                .workspace
                                .panes
                                .get_pane_bounds(tui_to_ferrite_rect(self.tui_app.buffer_area))
                            {
                                if ferrite_to_tui_rect(pane_rect)
                                    .contains(Position::new(event.column, event.row))
                                {
                                    self.tui_app.engine.workspace.panes.make_current(pane_kind);
                                    if let PaneKind::Buffer(buffer_id, view_id) = pane_kind {
                                        let buffer =
                                            &self.tui_app.engine.workspace.buffers[buffer_id];
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
                                .tui_app
                                .engine
                                .workspace
                                .panes
                                .get_pane_bounds(tui_to_ferrite_rect(self.tui_app.buffer_area))
                            {
                                if ferrite_to_tui_rect(pane_rect)
                                    .contains(Position::new(event.column, event.row))
                                {
                                    self.tui_app.engine.workspace.panes.make_current(pane_kind);
                                    if let PaneKind::Buffer(buffer_id, view_id) = pane_kind {
                                        let buffer =
                                            &self.tui_app.engine.workspace.buffers[buffer_id];
                                        self.tui_app.drag_start = Some(Point::new(
                                            event.column as usize + buffer.col_pos(view_id),
                                            event.row as usize + buffer.line_pos(view_id),
                                        ));

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
                            self.tui_app.drag_start = None;
                            None
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            for (pane_kind, pane_rect) in self
                                .tui_app
                                .engine
                                .workspace
                                .panes
                                .get_pane_bounds(tui_to_ferrite_rect(self.tui_app.buffer_area))
                            {
                                if ferrite_to_tui_rect(pane_rect)
                                    .contains(Position::new(event.column, event.row))
                                {
                                    self.tui_app.engine.workspace.panes.make_current(pane_kind);
                                    if let PaneKind::Buffer(buffer_id, view_id) = pane_kind {
                                        // TODO maybe scroll more of the buffer into view when going outside its bounds
                                        if let Some(Point { line, column }) =
                                            self.tui_app.drag_start
                                        {
                                            let buffer = &mut self.tui_app.engine.workspace.buffers
                                                [buffer_id];
                                            let (_, left_offset) =
                                                lines_to_left_offset(buffer.len_lines());

                                            let anchor = {
                                                let column = column
                                                    .saturating_sub(left_offset)
                                                    .saturating_sub(pane_rect.x);
                                                let line = line.saturating_sub(pane_rect.y);
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

            self.tui_app.engine.buffer_area = tui_to_ferrite_rect(self.tui_app.buffer_area);
            if let Some(input) = input {
                self.tui_app
                    .engine
                    .handle_input_command(input, control_flow);
            }
        }
    }
}

impl Drop for TermApp {
    fn drop(&mut self) {
        if self.keyboard_enhancement {
            let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
        }
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            self.tui_app.terminal.backend_mut(),
            event::DisableMouseCapture,
            event::DisableBracketedPaste,
            terminal::LeaveAlternateScreen,
        );
        let _ = self.tui_app.terminal.show_cursor();
        clipboard::uninit();
    }
}
