use std::{io, io::Stdout, time::Instant};

use ferrite_runtime::{MouseInterctionKind, Runtime, event_loop_proxy::EventLoopProxy};

use crate::event_loop::TuiEventLoop;
pub mod event_loop;
mod glue;

use std::time::Duration;

use crossterm::{
    event,
    event::{
        Event, KeyEventKind, KeyboardEnhancementFlags, MouseEventKind, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute, terminal,
};
use ferrite_geom::rect::{Rect, Vec2};
use ferrite_runtime::{
    Bounds, Input, Layout, MouseButton, MouseInterction, MouseState, Painter, StartOfFrame, Update,
    View,
    any_view::AnyView,
    event_loop_proxy::EventLoopControlFlow,
    input::{event::InputEvent, keycode::KeyModifiers},
    painter::Rounding,
};
use tui::Terminal;

use crate::event_loop::{TuiEvent, TuiEventLoopProxy};

pub fn create_event_loop<UserEvent: Send + 'static>() -> (
    event_loop::TuiEventLoop<UserEvent>,
    Box<dyn EventLoopProxy<UserEvent>>,
) {
    let event_loop = TuiEventLoop::new();
    let proxy = event_loop.create_proxy();
    (event_loop, Box::new(proxy))
}
pub struct TermPlatform<S, UserEvent> {
    terminal: tui::Terminal<tui::backend::CrosstermBackend<Stdout>>,
    painter: Painter,
    runtime: Runtime<S>,
    update: Update<S>,
    input: Input<S, UserEvent>,
    layout: Layout<S>,
    start_of_frame: StartOfFrame<S>,
    view_tree: AnyView<S>,
    keyboard_enhancement: bool,
    columns: u16,
    lines: u16,
    modifiers: KeyModifiers,
    mouse_state: MouseState,
    dirty: bool,
}

impl<S, UserEvent> TermPlatform<S, UserEvent> {
    pub fn new(
        mut state: S,
        update: Update<S>,
        input: Input<S, UserEvent>,
        layout: Layout<S>,
        start_of_frame: StartOfFrame<S>,
    ) -> Result<Self, io::Error> {
        let backend = tui::backend::CrosstermBackend::new(std::io::stdout());
        let terminal = Terminal::new(backend)?;
        let (columns, lines) = crossterm::terminal::size()?;
        let view_tree = (layout)(&mut state);
        let painter = Painter::new(false);
        Ok(Self {
            terminal,
            painter,
            runtime: Runtime::new(state),
            update,
            input,
            layout,
            start_of_frame,
            view_tree,
            keyboard_enhancement: false,
            columns,
            lines,
            modifiers: KeyModifiers::default(),
            mouse_state: MouseState::default(),
            dirty: true,
        })
    }

    pub fn run(mut self, event_loop: event_loop::TuiEventLoop<UserEvent>) {
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
            self.keyboard_enhancement = true
        }

        // Reset terminal to non raw mode on panic
        std::panic::set_hook(Box::new(move |info| {
            if self.keyboard_enhancement {
                let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
            }
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
            println!("{panic_info}");
        }));

        self.runtime.drawing_backend = String::from("terminal");
        self.runtime.window_backend =
            std::env::var("TERM").unwrap_or_else(|_| "unknown".to_string());

        event_loop.run(|proxy, event, control_flow| self.handle_event(proxy, event, control_flow));
    }

    #[profiling::function]
    pub fn handle_event(
        &mut self,
        proxy: &TuiEventLoopProxy<UserEvent>,
        event: TuiEvent<UserEvent>,
        control_flow: &mut EventLoopControlFlow,
    ) {
        match event {
            event_loop::TuiEvent::StartOfEvents => {
                self.runtime.start_of_events = Instant::now();
                (self.start_of_frame)(&mut self.runtime);
            }
            event_loop::TuiEvent::Crossterm(event) => {
                self.handle_crossterm_event(proxy, event, control_flow)
            }
            event_loop::TuiEvent::UserEvent(event) => {
                self.dirty = true;
                (self.input)(
                    &mut self.runtime.state,
                    InputEvent::UserEvent(event),
                    control_flow,
                );
            }
            event_loop::TuiEvent::Render => {
                self.dirty = true;
                self.render(control_flow);
            }
        }
    }

    #[profiling::function]
    pub fn render(&mut self, control_flow: &mut EventLoopControlFlow) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        (self.update)(&mut self.runtime, control_flow);
        self.view_tree = (self.layout)(&mut self.runtime.state);

        if self.runtime.force_redraw {
            self.runtime.force_redraw = false;
            self.terminal.clear().unwrap();
        }

        let bounds = Bounds::new(
            Rect::new(0, 0, self.columns.into(), self.lines.into()),
            Vec2::new(1.0, 1.0),
            Rounding::Round,
        );
        {
            profiling::scope!("view tree render");
            self.view_tree
                .render(&mut self.runtime.state, bounds, &mut self.painter);
        }

        {
            profiling::scope!("terminal draw");
            self.terminal
                .draw(|f| {
                    for (_, _, layer) in self.painter.layers() {
                        let layer = layer.lock().unwrap();
                        overlay(f.buffer_mut(), &layer.buf);
                    }
                })
                .unwrap();
        }

        self.runtime.last_render_time = Instant::now().duration_since(self.runtime.start_of_events);
        self.painter.clean_up_frame();
    }

    #[profiling::function]
    pub fn handle_crossterm_event(
        &mut self,
        _proxy: &TuiEventLoopProxy<UserEvent>,
        event: event::Event,
        control_flow: &mut EventLoopControlFlow,
    ) {
        match event {
            Event::Resize(columns, lines) => {
                profiling::scope!("resize");
                self.dirty = true;
                self.columns = columns;
                self.lines = lines;
                self.terminal.clear().unwrap();
                self.render(control_flow);
            }
            Event::Key(event) => {
                profiling::scope!("key");
                tracing::debug!("{:?}", event);
                self.dirty = true;
                if event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat {
                    let keycode = glue::convert_keycode(event.code);
                    let modifiers = glue::convert_modifier(event.modifiers);
                    self.modifiers = modifiers;
                    (self.input)(
                        &mut self.runtime.state,
                        InputEvent::Key(keycode, modifiers),
                        control_flow,
                    );
                }
            }
            Event::Paste(string) => {
                profiling::scope!("paste");
                self.dirty = true;
                (self.input)(
                    &mut self.runtime.state,
                    InputEvent::Paste(string),
                    control_flow,
                );
            }
            Event::Mouse(event) => {
                profiling::scope!("mouse");
                self.modifiers = glue::convert_modifier(event.modifiers);
                match event.kind {
                    MouseEventKind::ScrollUp => {
                        self.dirty = true;
                        (self.input)(
                            &mut self.runtime.state,
                            InputEvent::Scroll(0.0, 1.0),
                            control_flow,
                        );
                    }
                    MouseEventKind::ScrollDown => {
                        self.dirty = true;
                        (self.input)(
                            &mut self.runtime.state,
                            InputEvent::Scroll(0.0, -1.0),
                            control_flow,
                        );
                    }
                    MouseEventKind::ScrollLeft => {
                        self.dirty = true;
                        (self.input)(
                            &mut self.runtime.state,
                            InputEvent::Scroll(1.0, 0.0),
                            control_flow,
                        );
                    }
                    MouseEventKind::ScrollRight => {
                        self.dirty = true;
                        (self.input)(
                            &mut self.runtime.state,
                            InputEvent::Scroll(-1.0, 0.0),
                            control_flow,
                        );
                    }
                    MouseEventKind::Down(button) => {
                        self.dirty = true;
                        self.mouse_state.position =
                            Vec2::new(event.column as f32, event.row as f32);
                        let button = match button {
                            crossterm::event::MouseButton::Left => MouseButton::Left,
                            crossterm::event::MouseButton::Right => MouseButton::Right,
                            crossterm::event::MouseButton::Middle => MouseButton::Middle,
                        };

                        let mouse_state = match button {
                            MouseButton::Left => &mut self.mouse_state.left,
                            MouseButton::Right => &mut self.mouse_state.right,
                            MouseButton::Middle => &mut self.mouse_state.middle,
                        };

                        mouse_state.pressed = true;
                        let now = Instant::now();
                        if now.duration_since(mouse_state.last_press) < Duration::from_millis(400) {
                            mouse_state.clicks += 1;
                            if mouse_state.clicks > 3 {
                                mouse_state.clicks = 1;
                            }
                        } else {
                            mouse_state.clicks = 1;
                        }
                        mouse_state.last_press = now;

                        let bounds = Bounds::new(
                            Rect::new(0, 0, self.columns.into(), self.lines.into()),
                            Vec2::new(1.0, 1.0),
                            Rounding::Round,
                        );

                        let mouse_interaction = MouseInterction {
                            button,
                            kind: MouseInterctionKind::Click(mouse_state.clicks),
                            cell_size: Vec2::new(1.0, 1.0),
                            position: self.mouse_state.position,
                            modifiers: self.modifiers,
                        };

                        self.view_tree.handle_mouse(
                            &mut self.runtime.state,
                            bounds,
                            mouse_interaction,
                        );
                    }
                    MouseEventKind::Up(button) => {
                        self.dirty = true;
                        self.mouse_state.position =
                            Vec2::new(event.column as f32, event.row as f32);
                        let button = match button {
                            crossterm::event::MouseButton::Left => MouseButton::Left,
                            crossterm::event::MouseButton::Right => MouseButton::Right,
                            crossterm::event::MouseButton::Middle => MouseButton::Middle,
                        };

                        let mouse_state = match button {
                            MouseButton::Left => &mut self.mouse_state.left,
                            MouseButton::Right => &mut self.mouse_state.right,
                            MouseButton::Middle => &mut self.mouse_state.middle,
                        };
                        mouse_state.pressed = false;

                        if mouse_state.drag_start.is_some() {
                            mouse_state.drag_start = None;
                            let bounds = Bounds::new(
                                Rect::new(0, 0, self.columns.into(), self.lines.into()),
                                Vec2::new(1.0, 1.0),
                                Rounding::Round,
                            );

                            let mouse_interaction = MouseInterction {
                                button,
                                kind: MouseInterctionKind::DragStop,
                                cell_size: Vec2::new(1.0, 1.0),
                                position: self.mouse_state.position,
                                modifiers: self.modifiers,
                            };

                            self.view_tree.handle_mouse(
                                &mut self.runtime.state,
                                bounds,
                                mouse_interaction,
                            );
                        }
                    }
                    MouseEventKind::Drag(button) => {
                        self.dirty = true;
                        let button = match button {
                            crossterm::event::MouseButton::Left => MouseButton::Left,
                            crossterm::event::MouseButton::Right => MouseButton::Right,
                            crossterm::event::MouseButton::Middle => MouseButton::Middle,
                        };

                        let mouse_state = match button {
                            MouseButton::Left => &mut self.mouse_state.left,
                            MouseButton::Right => &mut self.mouse_state.right,
                            MouseButton::Middle => &mut self.mouse_state.middle,
                        };

                        if mouse_state.drag_start.is_none() {
                            mouse_state.drag_start = Some(self.mouse_state.position);
                        }
                        let last_pos = self.mouse_state.position;
                        self.mouse_state.position =
                            Vec2::new(event.column as f32, event.row as f32);

                        mouse_state.pressed = true;

                        let bounds = Bounds::new(
                            Rect::new(0, 0, self.columns.into(), self.lines.into()),
                            Vec2::new(1.0, 1.0),
                            Rounding::Round,
                        );

                        let mouse_interaction = MouseInterction {
                            button,
                            kind: MouseInterctionKind::Drag {
                                drag_start: mouse_state.drag_start.unwrap(),
                                last_pos,
                            },
                            cell_size: Vec2::new(1.0, 1.0),
                            position: self.mouse_state.position,
                            modifiers: self.modifiers,
                        };

                        self.view_tree.handle_mouse(
                            &mut self.runtime.state,
                            bounds,
                            mouse_interaction,
                        );
                    }
                    MouseEventKind::Moved => (),
                };
            }
            _ => (),
        }
    }
}

impl<S, UserEvent> Drop for TermPlatform<S, UserEvent> {
    fn drop(&mut self) {
        if self.keyboard_enhancement {
            let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
        }
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            event::DisableMouseCapture,
            event::DisableBracketedPaste,
            terminal::LeaveAlternateScreen,
        );
        let _ = self.terminal.show_cursor();
    }
}

#[profiling::function]
pub fn overlay(output: &mut tui::buffer::Buffer, input: &tui::buffer::Buffer) {
    for x in input.area.x..(input.area.x + input.area.width) {
        for y in input.area.y..(input.area.y + input.area.height) {
            if let Some(out_cell) = output.cell_mut((x, y)) {
                let in_cell = &input[(x, y)];
                if in_cell != &tui::buffer::Cell::EMPTY {
                    out_cell.set_symbol(in_cell.symbol());
                    if in_cell.fg != tui::buffer::Cell::EMPTY.fg {
                        out_cell.fg = in_cell.fg;
                    }
                    if in_cell.bg != tui::buffer::Cell::EMPTY.bg {
                        out_cell.bg = in_cell.bg;
                    }
                    if in_cell.modifier != tui::buffer::Cell::EMPTY.modifier {
                        out_cell.modifier = in_cell.modifier;
                    }
                }
            }
        }
    }
}
