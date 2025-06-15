use std::{
    collections::HashMap,
    iter,
    sync::{Arc, mpsc},
    time::Instant,
};

use anyhow::Result;
use backend::{WgpuBackend, calculate_cell_size, get_metrics};
use event_loop_wrapper::EventLoopProxyWrapper;
use ferrite_cli::Args;
use ferrite_core::{
    buffer::ViewId,
    clipboard,
    cmd::Cmd,
    config::editor::{FontWeight, default_font},
    event_loop_proxy::{EventLoopControlFlow, UserEvent},
    keymap::{self, keycode::KeyModifiers},
    layout::panes::PaneKind,
    logger::LogMessage,
    workspace::BufferId,
};
use ferrite_tui::{
    TuiApp,
    glue::{ferrite_to_tui_rect, tui_to_ferrite_rect},
    widgets::editor_widget::lines_to_left_offset,
};
use ferrite_utility::{
    chars::char_is_line_ending,
    geom::{Rect, Vec2},
    line_ending::LineEnding,
    point::Point,
};
use glue::convert_keycode;
use renderer::{Layer, Renderer};
use tui::{Terminal, layout::Position};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{CursorIcon, Window},
};

use crate::renderer::{
    Bundle,
    geometry_renderer::{Geometry, Quad},
};

mod backend;
mod event_loop_wrapper;
mod glue;
pub mod renderer;
pub mod srgb;

pub fn run(args: &Args, rx: mpsc::Receiver<LogMessage>) -> Result<()> {
    {
        std::panic::set_hook(Box::new(move |info| {
            println!();
            let _ = std::fs::write("./panic.txt", format!("{info:?}"));
            let backtrace = std::backtrace::Backtrace::force_capture();
            let panic_info = format!("{backtrace}\n{info}");
            let _ = std::fs::write("panic.txt", &panic_info);
            println!("{panic_info}");
        }));
    }

    let event_loop = EventLoop::with_user_event().build()?;
    let gui_app = pollster::block_on(GuiApp::new(args, &event_loop, rx))?;
    gui_app.run(event_loop);

    Ok(())
}

pub struct Drag {
    drag_start: Vec2<f64>,
    pane_kind: PaneKind,
    scrollbar: bool,
}

pub struct TerminalPane {
    terminal: Terminal<WgpuBackend>,
    pane_rect: Rect<usize>,
    touched: bool,
}

struct GuiApp {
    tui_app: TuiApp,
    terminals: [Terminal<WgpuBackend>; 1],
    terminal_panes: HashMap<PaneKind, TerminalPane>,
    control_flow: EventLoopControlFlow,
    renderer: Renderer,
    // rendering stuff
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f64,
    window: Arc<Window>,
    // input
    modifiers: KeyModifiers,
    mouse_position: PhysicalPosition<f64>,
    drag: Option<Drag>,
    primary_mouse_button_pressed: bool,
}

impl GuiApp {
    pub async fn new(
        args: &Args,
        event_loop: &EventLoop<UserEvent>,
        rx: mpsc::Receiver<LogMessage>,
    ) -> Result<Self> {
        let event_loop_wrapper = EventLoopProxyWrapper::new(event_loop.create_proxy());

        #[allow(deprecated)]
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("Ferrite"))
                .unwrap(),
        );
        let size = window.inner_size();

        let backends = if cfg!(windows) {
            wgpu::Backends::DX12
        } else if cfg!(target_os = "macos") {
            wgpu::Backends::PRIMARY
        } else {
            wgpu::Backends::all()
        };

        let instance_descriptor = wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        };

        let instance = wgpu::Instance::new(&instance_descriptor);

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        let mut renderer = Renderer::new(&device, &config, size.width as f32, size.height as f32);

        let terminal = Terminal::new(WgpuBackend::new(
            &mut renderer.font_system,
            size.width as f32,
            size.height as f32,
            default_font(),
            FontWeight::Normal,
        ))?;

        let term_size = terminal.size()?;
        let tui_app = TuiApp::new(
            args,
            event_loop_wrapper,
            rx,
            term_size.width,
            term_size.height,
        )?;

        let terminals = [terminal];
        let terminal_panes = HashMap::new();

        let scale_factor = 1.0;

        window.set_visible(true);

        let control_flow = EventLoopControlFlow::Wait;

        Ok(Self {
            tui_app,
            terminals,
            terminal_panes,
            renderer,
            control_flow,
            window,
            surface,
            device,
            queue,
            config,
            size,
            scale_factor,
            modifiers: KeyModifiers::empty(),
            mouse_position: PhysicalPosition::default(),
            drag: None,
            primary_mouse_button_pressed: false,
        })
    }

    pub fn run(mut self, event_loop: EventLoop<UserEvent>) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        #[allow(deprecated)]
        event_loop
            .run(move |event, event_loop| match event {
                Event::NewEvents(_) => {
                    // Padding should always be 0 in gui
                    self.tui_app.engine.workspace.panes.padding = 0;
                    self.tui_app.start_of_events();
                }
                Event::UserEvent(event) => {
                    profiling::scope!("user event");
                    self.tui_app
                        .engine
                        .handle_app_event(event, &mut self.control_flow);
                    if self.control_flow == EventLoopControlFlow::Exit {
                        event_loop.exit();
                    }
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => event_loop.exit(),
                    WindowEvent::RedrawRequested => match self.render() {
                        Ok(()) => (),
                        Err(wgpu::SurfaceError::Lost) => self.resize(self.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => tracing::error!("Surface error: {:?}", e),
                    },
                    event => {
                        profiling::scope!("window event");
                        self.input(event_loop, event)
                    }
                },
                Event::AboutToWait => {
                    profiling::scope!("about to wait");

                    self.tui_app.engine.do_polling(&mut self.control_flow);
                    match self.control_flow {
                        EventLoopControlFlow::Poll => {
                            event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
                        }
                        EventLoopControlFlow::Wait => {
                            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
                        }
                        EventLoopControlFlow::Exit => event_loop.exit(),
                        EventLoopControlFlow::WaitMax(duration) => {
                            event_loop.set_control_flow(
                                winit::event_loop::ControlFlow::wait_duration(duration),
                            );
                        }
                    }
                    for terminal in &mut self.terminals {
                        terminal.backend_mut().set_font_family(
                            &mut self.renderer.font_system,
                            &self.tui_app.engine.config.editor.gui.font_family,
                        );
                        terminal.backend_mut().set_font_weight(
                            &mut self.renderer.font_system,
                            self.tui_app.engine.config.editor.gui.font_weight,
                        );
                        terminal
                            .backend_mut()
                            .set_scale(&mut self.renderer.font_system, self.tui_app.engine.scale);
                    }

                    for pane in self.terminal_panes.values_mut() {
                        pane.terminal.backend_mut().set_font_family(
                            &mut self.renderer.font_system,
                            &self.tui_app.engine.config.editor.gui.font_family,
                        );
                        pane.terminal.backend_mut().set_font_weight(
                            &mut self.renderer.font_system,
                            self.tui_app.engine.config.editor.gui.font_weight,
                        );
                        pane.terminal
                            .backend_mut()
                            .set_scale(&mut self.renderer.font_system, self.tui_app.engine.scale);
                    }

                    self.render_tui();
                    if self.terminals.iter().any(|t| t.backend().redraw)
                        || self
                            .terminal_panes
                            .values()
                            .any(|t| t.terminal.backend().redraw)
                    {
                        self.window.request_redraw();
                        for terminal in &mut self.terminals {
                            terminal.backend_mut().redraw = false;
                        }
                        for pane in self.terminal_panes.values_mut() {
                            pane.terminal.backend_mut().redraw = false;
                        }
                    }
                }
                _event => (),
            })
            .unwrap();
    }

    #[profiling::function]
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.renderer
            .resize(self.size.width as f32, self.size.height as f32);
        for terminal in &mut self.terminals {
            terminal
                .backend_mut()
                .resize(self.size.width as f32, self.size.height as f32);
            let backend = terminal.backend();
            let columns = backend.columns;
            let lines = backend.lines;
            let _ = terminal.resize(tui::layout::Rect {
                x: 0,
                y: 0,
                width: columns,
                height: lines,
            });
        }
        self.render_tui();
    }

    #[profiling::function]
    pub fn input(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::Focused(false) => {
                self.modifiers = KeyModifiers::empty();
            }
            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size);
                self.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_, y) => {
                    self.tui_app.engine.handle_single_input_command(
                        Cmd::VerticalScroll {
                            distance: -y as f64 * 3.0,
                        },
                        &mut EventLoopControlFlow::Poll,
                    );
                }
                MouseScrollDelta::PixelDelta(physical_pos) => {
                    let line_height = self.terminals[0].backend().line_height() as f64;
                    let distance = physical_pos.y / line_height;
                    self.tui_app.engine.handle_single_input_command(
                        Cmd::VerticalScroll { distance },
                        &mut EventLoopControlFlow::Poll,
                    );
                }
            },
            WindowEvent::ModifiersChanged(modifiers) => {
                let modifiers = modifiers.state();
                self.modifiers.set(
                    KeyModifiers::CONTROL,
                    modifiers.contains(ModifiersState::CONTROL),
                );
                self.modifiers
                    .set(KeyModifiers::ALT, modifiers.contains(ModifiersState::ALT));
                self.modifiers.set(
                    KeyModifiers::SHIFT,
                    modifiers.contains(ModifiersState::SHIFT),
                );
            }
            WindowEvent::KeyboardInput { event, .. } => {
                tracing::trace!("{:?}", event);
                let mut control_flow = self.control_flow;

                if let Key::Named(key) = event.logical_key {
                    match key {
                        NamedKey::Super => {
                            self.modifiers
                                .set(KeyModifiers::SUPER, event.state.is_pressed());
                            return;
                        }
                        NamedKey::Hyper => {
                            self.modifiers
                                .set(KeyModifiers::HYPER, event.state.is_pressed());
                            return;
                        }
                        NamedKey::Meta => {
                            self.modifiers
                                .set(KeyModifiers::META, event.state.is_pressed());
                            return;
                        }
                        _ => (),
                    }
                }

                if !event.state.is_pressed() {
                    return;
                }

                let cmd = 'block: {
                    match event.logical_key {
                        Key::Named(key) => {
                            if let Some(keycode) = convert_keycode(key, self.modifiers) {
                                let cmd = keymap::get_command_from_input(
                                    keycode,
                                    self.modifiers,
                                    self.tui_app.engine.get_current_keymappings(),
                                    self.tui_app.engine.get_input_ctx(),
                                );
                                break 'block cmd;
                            }
                        }
                        Key::Character(s) => {
                            if s.chars().count() == 1 {
                                let ch = s.chars().next().unwrap();
                                let cmd = if LineEnding::from_char(ch).is_some() {
                                    Some(Cmd::Enter)
                                } else {
                                    keymap::get_command_from_input(
                                        keymap::keycode::KeyCode::Char(s.chars().next().unwrap()),
                                        self.modifiers,
                                        self.tui_app.engine.get_current_keymappings(),
                                        self.tui_app.engine.get_input_ctx(),
                                    )
                                };
                                break 'block cmd;
                            } else {
                                break 'block Some(Cmd::Insert {
                                    text: s.to_string(),
                                });
                            };
                        }
                        _ => (),
                    }
                    None
                };

                if let Some(cmd) = cmd {
                    self.tui_app
                        .engine
                        .handle_input_command(cmd, &mut control_flow);
                    if control_flow == EventLoopControlFlow::Exit {
                        event_loop.exit();
                    }
                    return;
                }

                if let Some(text) = event.text {
                    let text: String = text
                        .chars()
                        .filter(|ch| !ch.is_ascii_control() || char_is_line_ending(*ch))
                        .collect();
                    if !text.is_empty() {
                        self.tui_app
                            .engine
                            .handle_input_command(Cmd::Insert { text }, &mut control_flow);
                    }
                    if control_flow == EventLoopControlFlow::Exit {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = position;
                if self.primary_mouse_button_pressed {
                    self.handle_drag(self.mouse_position.x, self.mouse_position.y);
                }
                self.handle_hover(self.mouse_position.x, self.mouse_position.y);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_click(self.mouse_position.x, self.mouse_position.y, state, button);
            }
            _ => (),
        }
    }

    pub fn pixel_to_cell(&self, x: f64, y: f64) -> Point<u16> {
        let (cell_width, cell_height) = self.get_cell_size();
        let column = (x / cell_width as f64).round() as u16;
        let line = (y / cell_height as f64) as u16;
        Point::new(column, line)
    }

    pub fn get_cell_size(&self) -> (f32, f32) {
        let backend = self.terminals[0].backend();
        (backend.cell_width, backend.cell_height)
    }

    pub fn handle_hover(&mut self, x: f64, y: f64) {
        let (cell_width, cell_height) = self.get_cell_size();
        let Point { column, line } = self.pixel_to_cell(x, y);
        let mut cursor = CursorIcon::Default;
        for (pane_kind, pane_rect) in self
            .tui_app
            .engine
            .workspace
            .panes
            .get_pane_bounds(self.tui_app.engine.buffer_area)
        {
            if let PaneKind::Buffer(buffer_id, view_id) = pane_kind {
                let rect = self.get_scrollbar_bounds(
                    &pane_rect,
                    buffer_id,
                    view_id,
                    cell_width,
                    cell_height,
                );
                if rect.contains(Vec2::new(x as f32, y as f32)) {
                    cursor = CursorIcon::Pointer;
                } else {
                    let buffer = &self.tui_app.engine.workspace.buffers[buffer_id];
                    let (_, left_offset) = lines_to_left_offset(buffer.len_lines());
                    let mut rect = ferrite_to_tui_rect(pane_rect);
                    rect.x += left_offset as u16;
                    rect.width = rect.width.saturating_sub(left_offset as u16);
                    rect.height = rect.height.saturating_sub(1);
                    if rect.contains(Position::new(column, line)) {
                        cursor = CursorIcon::Text;
                    }
                }
            }
        }
        self.window.set_cursor(cursor);
    }

    pub fn handle_click(&mut self, x: f64, y: f64, state: ElementState, button: MouseButton) {
        let (cell_width, cell_height) = calculate_cell_size(
            &mut self.renderer.font_system,
            get_metrics(self.tui_app.engine.scale),
            self.tui_app.engine.config.editor.gui.font_weight,
        );
        let Point { column, line } = self.pixel_to_cell(x, y);
        let input = 'block: {
            match (state, button) {
                (ElementState::Pressed, MouseButton::Middle) => {
                    if let Some((pane_kind, pane_rect)) = self
                        .tui_app
                        .engine
                        .workspace
                        .panes
                        .get_pane_bounds(self.tui_app.engine.buffer_area)
                        .iter()
                        .find(|(_, pane_rect)| {
                            pane_rect.contains(Vec2::new(column as usize, line as usize))
                        })
                    {
                        self.tui_app.engine.workspace.panes.make_current(*pane_kind);
                        if let PaneKind::Buffer(buffer_id, view_id) = *pane_kind {
                            let buffer = &self.tui_app.engine.workspace.buffers[buffer_id];
                            let (_, left_offset) = lines_to_left_offset(buffer.len_lines());
                            let column = ((column as usize) + buffer.col_pos(view_id))
                                .saturating_sub(pane_rect.x)
                                .saturating_sub(left_offset);
                            let line = (line as usize + buffer.line_pos(view_id))
                                .saturating_sub(pane_rect.y);
                            break 'block Some(Cmd::PastePrimary { column, line });
                        }
                    }

                    None
                }
                (ElementState::Pressed, MouseButton::Left) => {
                    self.primary_mouse_button_pressed = true;
                    let Point { column, line } = self.pixel_to_cell(x, y);
                    if let Some((pane_kind, pane_rect)) = self
                        .tui_app
                        .engine
                        .workspace
                        .panes
                        .get_pane_bounds(self.tui_app.engine.buffer_area)
                        .iter()
                        .find(|(_, pane_rect)| {
                            pane_rect.contains(Vec2::new(column as usize, line as usize))
                        })
                    {
                        self.tui_app.engine.workspace.panes.make_current(*pane_kind);
                        if let PaneKind::Buffer(buffer_id, view_id) = *pane_kind {
                            let scrollbar_rect = self.get_scrollbar_bounds(
                                pane_rect,
                                buffer_id,
                                view_id,
                                cell_width,
                                cell_height,
                            );
                            let buffer = &self.tui_app.engine.workspace.buffers[buffer_id];

                            if scrollbar_rect.contains(Vec2::new(x as f32, y as f32)) {
                                // TODO make this the whole height of the editor and move the scroll bar
                                self.drag = Some(Drag {
                                    drag_start: Vec2::new(x, y),
                                    pane_kind: PaneKind::Buffer(buffer_id, view_id),
                                    scrollbar: true,
                                });
                                break 'block None;
                            }

                            self.drag = Some(Drag {
                                drag_start: Vec2::new(
                                    column as f64 + buffer.col_pos(view_id) as f64,
                                    line as f64 + buffer.line_pos(view_id) as f64,
                                ),
                                pane_kind: PaneKind::Buffer(buffer_id, view_id),
                                scrollbar: false,
                            });

                            let (_, left_offset) = lines_to_left_offset(buffer.len_lines());
                            let column = ((column as usize) + buffer.col_pos(view_id))
                                .saturating_sub(pane_rect.x)
                                .saturating_sub(left_offset);
                            let line = (line as usize + buffer.line_pos(view_id))
                                .saturating_sub(pane_rect.y);
                            break 'block Some(Cmd::ClickCell {
                                spawn_cursor: self.modifiers.contains(KeyModifiers::ALT),
                                column,
                                line,
                            });
                        }
                    }

                    None
                }
                (ElementState::Released, MouseButton::Left) => {
                    self.drag = None;
                    self.primary_mouse_button_pressed = false;
                    None
                }
                _ => None,
            }
        };
        if let Some(input) = input {
            self.tui_app
                .engine
                // EventLoopControlFlow is just a dummy value as mouse input should not affect control flow
                .handle_input_command(input, &mut EventLoopControlFlow::Poll);
        }
    }

    pub fn handle_drag(&mut self, x: f64, y: f64) {
        let Point {
            column: drag_column,
            line: drag_line,
        } = self.pixel_to_cell(x, y);
        let mut input = None;
        if let Some((_, pane_rect)) = self
            .tui_app
            .engine
            .workspace
            .panes
            .get_pane_bounds(self.tui_app.engine.buffer_area)
            .iter()
            .find(|(_, pane_rect)| {
                pane_rect.contains(Vec2::new(drag_column.into(), drag_line.into()))
            })
        {
            let (_, cell_height) = self.get_cell_size();
            // TODO maybe scroll more of the buffer into view when going outside its bounds
            if let Some(Drag {
                drag_start,
                pane_kind: PaneKind::Buffer(buffer_id, view_id),
                scrollbar,
            }) = &mut self.drag
            {
                if !(self
                    .tui_app
                    .engine
                    .workspace
                    .buffers
                    .get(*buffer_id)
                    .is_some()
                    && self.tui_app.engine.workspace.buffers[*buffer_id]
                        .views
                        .get(*view_id)
                        .is_some())
                {
                    self.drag = None;
                    return;
                }
                let buffer = &mut self.tui_app.engine.workspace.buffers[*buffer_id];

                if *scrollbar {
                    let moved_distance = (drag_start.y - y) as f32;
                    let len_lines = (buffer.len_lines() + pane_rect.height.saturating_sub(1)) - 1;
                    let text_height = pane_rect.height.saturating_sub(1);
                    let scrollbar_ratio = text_height as f32 / len_lines as f32;
                    let line_distance = (moved_distance / cell_height) / scrollbar_ratio;

                    drag_start.y = y;
                    input = Some(Cmd::VerticalScroll {
                        distance: -line_distance as f64,
                    });
                } else {
                    let column = drag_start.x as usize;
                    let line = drag_start.y as usize;
                    let (_, left_offset) = lines_to_left_offset(buffer.len_lines());

                    let anchor = {
                        let column = column
                            .saturating_sub(left_offset)
                            .saturating_sub(pane_rect.x);
                        let line = line.saturating_sub(pane_rect.y);
                        Point::new(column, line)
                    };

                    let cursor = {
                        let column = ((drag_column as usize) + buffer.col_pos(*view_id))
                            .saturating_sub(left_offset)
                            .saturating_sub(pane_rect.x);
                        let line = (drag_line as usize + buffer.line_pos(*view_id))
                            .saturating_sub(pane_rect.y);
                        Point::new(column, line)
                    };

                    input = Some(Cmd::SelectArea { cursor, anchor });
                }
            }
        }

        if let Some(input) = input {
            self.tui_app
                .engine
                // EventLoopControlFlow is just a dummy value as mouse input should not affect control flow
                .handle_input_command(input, &mut EventLoopControlFlow::Poll);
        }
    }

    #[profiling::function]
    pub fn render_tui(&mut self) {
        for terminal in self.terminal_panes.values_mut() {
            terminal.touched = false;
        }

        let size = self.terminals[0].get_frame().area();
        let editor_size = tui::layout::Rect::new(
            size.x,
            size.y,
            size.width,
            size.height
                .saturating_sub(self.tui_app.engine.palette.height() as u16),
        );
        self.tui_app.engine.buffer_area = tui_to_ferrite_rect(editor_size);

        let panes = self
            .tui_app
            .engine
            .workspace
            .panes
            .get_pane_bounds(tui_to_ferrite_rect(editor_size));

        let (cell_width, cell_height) = calculate_cell_size(
            &mut self.renderer.font_system,
            get_metrics(self.tui_app.engine.scale),
            self.tui_app.engine.config.editor.gui.font_weight,
        );

        for (pane, pane_rect) in &panes {
            let mut new = false;
            let terminal_pane = self.terminal_panes.entry(*pane).or_insert_with(|| {
                new = true;
                TerminalPane {
                    terminal: Terminal::new(WgpuBackend::new(
                        &mut self.renderer.font_system,
                        pane_rect.width as f32 * cell_width,
                        pane_rect.height as f32 * cell_height,
                        self.tui_app.engine.config.editor.gui.font_family.clone(),
                        self.tui_app.engine.config.editor.gui.font_weight,
                    ))
                    .unwrap(),
                    pane_rect: *pane_rect,
                    touched: false,
                }
            });
            let terminal = &mut terminal_pane.terminal;
            let backend = terminal.backend_mut();

            if &terminal_pane.pane_rect != pane_rect || new {
                terminal_pane.pane_rect = *pane_rect;
                backend.x = pane_rect.x as f32 * cell_width;
                backend.y = pane_rect.y as f32 * cell_height;
                backend.resize(
                    pane_rect.width as f32 * cell_width,
                    pane_rect.height as f32 * cell_height,
                );
                let columns = backend.columns;
                let lines = backend.lines;
                terminal
                    .resize(tui::layout::Rect::new(0, 0, columns, lines))
                    .unwrap();
            }
            terminal_pane.touched = true;
        }

        self.terminal_panes.retain(|_, v| v.touched);

        for (pane, _) in &panes {
            let terminal = &mut self.terminal_panes.get_mut(pane).unwrap().terminal;
            terminal
                .draw(|f| {
                    let area = f.area();
                    match pane {
                        PaneKind::Buffer(buffer_id, view_id) => {
                            self.tui_app
                                .draw_buffer(f.buffer_mut(), area, *buffer_id, *view_id);
                        }
                        PaneKind::FileExplorer(file_explorer_id) => {
                            self.tui_app.draw_file_explorer(
                                f.buffer_mut(),
                                area,
                                *file_explorer_id,
                            );
                        }
                        PaneKind::Logger => {
                            self.tui_app.draw_logger(f.buffer_mut(), area);
                        }
                    }
                })
                .unwrap();
        }

        self.terminals[0]
            .draw(|f| {
                let area = f.area();
                f.render_widget(tui::widgets::Clear, area);
                self.tui_app.draw_overlays(f.buffer_mut(), area);
            })
            .unwrap();
    }

    #[profiling::function]
    pub fn draw_buffer_overlay(&mut self) -> Geometry {
        let mut geometry = Geometry::default();

        let (cell_width, cell_height) = calculate_cell_size(
            &mut self.renderer.font_system,
            get_metrics(self.tui_app.engine.scale),
            self.tui_app.engine.config.editor.gui.font_weight,
        );

        let size = self.terminals[0].get_frame().area();
        let editor_size = tui::layout::Rect::new(
            size.x,
            size.y,
            size.width,
            size.height
                .saturating_sub(self.tui_app.engine.palette.height() as u16),
        );
        let panes = self
            .tui_app
            .engine
            .workspace
            .panes
            .get_pane_bounds(tui_to_ferrite_rect(editor_size));

        let theme = &self.tui_app.engine.themes[&self.tui_app.engine.config.editor.theme];
        let border_color = crate::glue::convert_style(&theme.pane_border)
            .0
            .unwrap_or(glyphon::Color::rgb(0, 0, 0));
        let (scroll_fg, scroll_bg) = crate::glue::convert_style(&theme.scrollbar);
        let scroll_fg = scroll_fg.unwrap_or(glyphon::Color::rgb(0, 0, 0));
        let scroll_bg = scroll_bg.unwrap_or(glyphon::Color::rgb(0, 0, 0));

        for (pane, pane_rect) in &panes {
            if pane_rect.x != 0 {
                let x = pane_rect.x as f32 * cell_width;
                let y = pane_rect.y as f32 * cell_height;
                let width = 1.0 * self.tui_app.engine.scale;
                let height = pane_rect.height as f32 * cell_height;
                geometry.quads.push(Quad {
                    x,
                    y,
                    width,
                    height,
                    color: border_color,
                });
            }

            // Draw scrollbars
            if let PaneKind::Buffer(buffer_id, view_id) = pane {
                let rect = self.get_scrollbar_bounds(
                    pane_rect,
                    *buffer_id,
                    *view_id,
                    cell_width,
                    cell_height,
                );
                geometry.quads.push(Quad {
                    x: (pane_rect.x + pane_rect.width) as f32 * cell_width - cell_width,
                    y: pane_rect.y as f32 * cell_height,
                    width: cell_width,
                    height: pane_rect.height as f32 * cell_height - cell_height,
                    color: scroll_bg,
                });
                geometry.quads.push(Quad {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                    color: scroll_fg,
                });
            }
        }
        geometry
    }

    #[profiling::function]
    pub fn render(&mut self) -> std::result::Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let overlay_geometry = self.draw_buffer_overlay();

        let theme = &self.tui_app.engine.themes[&self.tui_app.engine.config.editor.theme];

        let mut layers = Vec::new();
        for terminal_pane in self.terminal_panes.values_mut() {
            let bundle = terminal_pane.terminal.backend_mut().prepare(
                &self.tui_app.engine.themes[&self.tui_app.engine.config.editor.theme],
                &mut self.renderer.font_system,
            );
            layers.push(Layer {
                bundles: vec![bundle],
            });
        }

        let tmp = Geometry::default();
        layers.push(Layer {
            bundles: vec![Bundle {
                text_area: None,
                bottom_geometry: &tmp,
                top_geometry: &overlay_geometry,
            }],
        });

        let bundles: Vec<_> = self
            .terminals
            .iter_mut()
            .map(|t| {
                t.backend_mut().prepare(
                    &self.tui_app.engine.themes[&self.tui_app.engine.config.editor.theme],
                    &mut self.renderer.font_system,
                )
            })
            .collect();
        layers.push(Layer { bundles });

        self.renderer
            .prepare(&self.device, &self.queue, &self.config, layers);

        {
            let color = theme.background.bg.unwrap_or_default();
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: srgb::srgb_to_linear(color.r) as f64,
                            g: srgb::srgb_to_linear(color.g) as f64,
                            b: srgb::srgb_to_linear(color.b) as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.renderer.render(&mut rpass);
        }

        {
            profiling::scope!("queue submit");
            self.queue.submit(iter::once(encoder.finish()));
        }
        {
            profiling::scope!("present");
            output.present();
        }

        self.tui_app.engine.last_render_time =
            Instant::now().duration_since(self.tui_app.engine.start_of_events);

        Ok(())
    }

    fn get_scrollbar_bounds(
        &self,
        rect: &Rect<usize>,
        buffer_id: BufferId,
        view_id: ViewId,
        cell_width: f32,
        cell_height: f32,
    ) -> Rect<f32> {
        let buffer = &self.tui_app.engine.workspace.buffers[buffer_id];
        let len_lines = (buffer.len_lines() + rect.height.saturating_sub(1)) - 1;
        let line_pos = buffer.views[view_id].line_pos;
        let text_height = rect.height.saturating_sub(1);

        let scrollbar_ratio = text_height as f32 / len_lines as f32;
        let scrollbar_pos_ratio = line_pos as f32 / len_lines as f32;

        let scrollbar_height = scrollbar_ratio * cell_height * text_height as f32;
        let scrollbar_pos = scrollbar_pos_ratio * cell_height * text_height as f32;

        let x = rect.x as f32 * cell_width + rect.width.saturating_sub(1) as f32 * cell_width;
        let y = rect.y as f32 * cell_height + scrollbar_pos;
        Rect::new(x, y, cell_width, scrollbar_height)
    }
}

impl Drop for GuiApp {
    fn drop(&mut self) {
        clipboard::uninit();
    }
}
