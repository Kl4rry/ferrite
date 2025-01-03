use std::{
    env, iter,
    sync::{mpsc, Arc},
    time::Instant,
};

use anyhow::Result;
use backend::WgpuBackend;
use event_loop_wrapper::EventLoopProxyWrapper;
use ferrite_cli::Args;
use ferrite_core::{
    clipboard,
    cmd::Cmd,
    config::editor::{default_font, FontWeight},
    event_loop_proxy::{EventLoopControlFlow, UserEvent},
    keymap::{self, keycode::KeyModifiers},
    layout::panes::PaneKind,
    logger::LogMessage,
};
use ferrite_tui::{
    glue::{ferrite_to_tui_rect, tui_to_ferrite_rect},
    widgets::editor_widget::lines_to_left_offset,
    TuiApp,
};
use ferrite_utility::{line_ending::LineEnding, point::Point};
use glue::convert_keycode;
use tui::layout::Position;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{EventLoop, EventLoopBuilder, EventLoopWindowTarget},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{CursorIcon, Window, WindowBuilder},
};

mod backend;
mod event_loop_wrapper;
mod glue;
pub mod srgb;

pub fn run(args: &Args, rx: mpsc::Receiver<LogMessage>) -> Result<()> {
    {
        std::panic::set_hook(Box::new(move |info| {
            println!();
            let _ = std::fs::write("./panic.txt", format!("{info:?}"));
            let backtrace = std::backtrace::Backtrace::force_capture();
            let panic_info = format!("{backtrace}\n{info}");
            let _ = std::fs::write("panic.txt", &panic_info);
            println!("{}", panic_info);
        }));
    }

    let event_loop = EventLoopBuilder::with_user_event().build()?;
    let gui_app = pollster::block_on(GuiApp::new(args, &event_loop, rx))?;
    gui_app.run(event_loop);

    Ok(())
}

struct GuiApp {
    tui_app: TuiApp<WgpuBackend>,
    control_flow: EventLoopControlFlow,
    // rendering stuff
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f64,
    window: Arc<Window>,
    modifiers: KeyModifiers,
    mouse_position: PhysicalPosition<f64>,
    primary_mouse_button_pressed: bool,
    vertical_scroll_delta: f64,
}

impl GuiApp {
    pub async fn new(
        args: &Args,
        event_loop: &EventLoop<UserEvent>,
        rx: mpsc::Receiver<LogMessage>,
    ) -> Result<Self> {
        let event_loop_wrapper = EventLoopProxyWrapper::new(event_loop.create_proxy());

        let window = Arc::new(
            WindowBuilder::new()
                .with_title("Ferrite")
                .build(event_loop)
                .unwrap(),
        );
        let size = window.inner_size();

        let mut backends = if cfg!(windows) {
            wgpu::Backends::DX12
        } else if cfg!(target_os = "macos") {
            wgpu::Backends::PRIMARY
        } else {
            wgpu::Backends::all()
        };

        if let Ok(gpu_backend) = env::var("FERRITE_GPU_BACKEND") {
            backends = wgpu::util::parse_backends_from_comma_list(&gpu_backend);
        } else if let Ok(gpu_backend) = env::var("WGPU_BACKEND") {
            backends = wgpu::util::parse_backends_from_comma_list(&gpu_backend);
        };

        let instance_descriptor = wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        };

        let instance = wgpu::Instance::new(instance_descriptor);

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
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::default(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
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

        let backend = WgpuBackend::new(
            &device,
            &queue,
            &config,
            size.width as f32,
            size.height as f32,
            default_font(),
            FontWeight::Normal,
        );

        let tui_app = TuiApp::new(args, event_loop_wrapper, backend, rx)?;

        let scale_factor = 1.0;

        window.set_visible(true);

        let control_flow = EventLoopControlFlow::Wait;

        Ok(Self {
            tui_app,
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
            primary_mouse_button_pressed: false,
            vertical_scroll_delta: 0.0,
        })
    }

    pub fn run(mut self, event_loop: EventLoop<UserEvent>) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        event_loop
            .run(move |event, event_loop| match event {
                Event::NewEvents(_) => {
                    self.tui_app.start_of_events();
                }
                Event::UserEvent(event) => {
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
                    event => self.input(event_loop, event),
                },
                Event::AboutToWait => {
                    profiling::scope!("about to wait");
                    let backend = self.tui_app.terminal.backend_mut();
                    if backend.scale() != self.tui_app.engine.scale {
                        backend.set_scale(self.tui_app.engine.scale);
                    }
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
                    self.tui_app
                        .terminal
                        .backend_mut()
                        .set_font_family(&self.tui_app.engine.config.editor.gui.font_family);
                    self.tui_app
                        .terminal
                        .backend_mut()
                        .set_font_weight(self.tui_app.engine.config.editor.gui.font_weight);
                    self.tui_app.render();
                    if self.tui_app.terminal.backend().redraw {
                        self.window.request_redraw();
                        self.tui_app.terminal.backend_mut().redraw = false;
                    }
                }
                _event => (),
            })
            .unwrap();
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.tui_app
            .terminal
            .backend_mut()
            .resize(self.size.width as f32, self.size.height as f32);
        let backend = self.tui_app.terminal.backend();
        let columns = backend.columns;
        let lines = backend.lines;
        let _ = self.tui_app.terminal.resize(tui::layout::Rect {
            x: 0,
            y: 0,
            width: columns,
            height: lines,
        });
        self.tui_app.render();
    }

    pub fn input(&mut self, event_loop: &EventLoopWindowTarget<UserEvent>, event: WindowEvent) {
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
                        Cmd::VerticalScroll(-y as i64 * 3),
                        &mut EventLoopControlFlow::Poll,
                    );
                }
                MouseScrollDelta::PixelDelta(physical_pos) => {
                    self.vertical_scroll_delta += physical_pos.y;
                    let line_height = self.tui_app.terminal.backend().line_height() as f64;
                    loop {
                        if self.vertical_scroll_delta >= line_height {
                            self.vertical_scroll_delta -= line_height;
                            self.tui_app.engine.handle_single_input_command(
                                Cmd::VerticalScroll(-1),
                                &mut EventLoopControlFlow::Poll,
                            );
                        } else if self.vertical_scroll_delta <= -line_height {
                            self.vertical_scroll_delta += line_height;
                            self.tui_app.engine.handle_single_input_command(
                                Cmd::VerticalScroll(1),
                                &mut EventLoopControlFlow::Poll,
                            );
                        } else {
                            break;
                        }
                    }
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
                                );
                                break 'block cmd;
                            }
                        }
                        Key::Character(s) => {
                            if s.chars().count() == 1 {
                                let ch = s.chars().next().unwrap();
                                let cmd = if LineEnding::from_char(ch).is_some() {
                                    Some(Cmd::Char('\n'))
                                } else {
                                    keymap::get_command_from_input(
                                        keymap::keycode::KeyCode::Char(s.chars().next().unwrap()),
                                        self.modifiers,
                                        self.tui_app.engine.get_current_keymappings(),
                                    )
                                };
                                break 'block cmd;
                            } else {
                                break 'block Some(Cmd::Insert(s.to_string()));
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
                    self.tui_app
                        .engine
                        .handle_input_command(Cmd::Insert(text.to_string()), &mut control_flow);
                    if control_flow == EventLoopControlFlow::Exit {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let backend = self.tui_app.terminal.backend();
                self.mouse_position = position;

                let column = (self.mouse_position.x / backend.cell_width as f64).round() as u16;
                let line = (self.mouse_position.y / backend.cell_height as f64) as u16;
                if self.primary_mouse_button_pressed {
                    self.handle_drag(column, line);
                }
                self.handle_hover(column, line);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let backend = self.tui_app.terminal.backend();

                let column = (self.mouse_position.x / backend.cell_width as f64).round() as u16;
                let line = (self.mouse_position.y / backend.cell_height as f64) as u16;
                self.handle_click(column, line, state, button);
            }
            _ => (),
        }
    }

    pub fn handle_hover(&mut self, column: u16, line: u16) {
        let mut cursor = CursorIcon::Default;
        for (pane_kind, pane_rect) in self
            .tui_app
            .engine
            .workspace
            .panes
            .get_pane_bounds(tui_to_ferrite_rect(self.tui_app.buffer_area))
        {
            if let PaneKind::Buffer(buffer_id, _) = pane_kind {
                let buffer = &self.tui_app.engine.workspace.buffers[buffer_id];
                let (_, left_offset) = lines_to_left_offset(buffer.len_lines());
                let mut rect = ferrite_to_tui_rect(pane_rect);
                rect.x += left_offset as u16;
                rect.width = rect.width.saturating_sub(left_offset as u16);
                rect.height = rect.height.saturating_sub(1);
                if rect.contains(Position::new(column, line)) {
                    cursor = CursorIcon::Text
                }
            }
        }
        self.window.set_cursor_icon(cursor);
    }

    pub fn handle_click(
        &mut self,
        column: u16,
        line: u16,
        state: ElementState,
        button: MouseButton,
    ) {
        let input = 'block: {
            match (state, button) {
                (ElementState::Pressed, MouseButton::Middle) => {
                    for (pane_kind, pane_rect) in self
                        .tui_app
                        .engine
                        .workspace
                        .panes
                        .get_pane_bounds(tui_to_ferrite_rect(self.tui_app.buffer_area))
                    {
                        if ferrite_to_tui_rect(pane_rect).contains(Position::new(column, line)) {
                            self.tui_app.engine.workspace.panes.make_current(pane_kind);
                            if let PaneKind::Buffer(buffer_id, view_id) = pane_kind {
                                let buffer = &self.tui_app.engine.workspace.buffers[buffer_id];
                                let (_, left_offset) = lines_to_left_offset(buffer.len_lines());
                                let column = ((column as usize) + buffer.col_pos(view_id))
                                    .saturating_sub(pane_rect.x)
                                    .saturating_sub(left_offset);
                                let line = (line as usize + buffer.line_pos(view_id))
                                    .saturating_sub(pane_rect.y);
                                break 'block Some(Cmd::PastePrimary(column, line));
                            }
                        }
                    }

                    None
                }
                (ElementState::Pressed, MouseButton::Left) => {
                    self.primary_mouse_button_pressed = true;
                    for (pane_kind, pane_rect) in self
                        .tui_app
                        .engine
                        .workspace
                        .panes
                        .get_pane_bounds(tui_to_ferrite_rect(self.tui_app.buffer_area))
                    {
                        if ferrite_to_tui_rect(pane_rect).contains(Position::new(column, line)) {
                            self.tui_app.engine.workspace.panes.make_current(pane_kind);
                            if let PaneKind::Buffer(buffer_id, view_id) = pane_kind {
                                let buffer = &self.tui_app.engine.workspace.buffers[buffer_id];
                                self.tui_app.drag_start = Some(Point::new(
                                    column as usize + buffer.col_pos(view_id),
                                    line as usize + buffer.line_pos(view_id),
                                ));

                                let (_, left_offset) = lines_to_left_offset(buffer.len_lines());
                                let column = ((column as usize) + buffer.col_pos(view_id))
                                    .saturating_sub(pane_rect.x)
                                    .saturating_sub(left_offset);
                                let line = (line as usize + buffer.line_pos(view_id))
                                    .saturating_sub(pane_rect.y);
                                break 'block Some(Cmd::ClickCell(
                                    self.modifiers.contains(KeyModifiers::ALT),
                                    column,
                                    line,
                                ));
                            }
                        }
                    }

                    None
                }
                (ElementState::Released, MouseButton::Left) => {
                    self.tui_app.drag_start = None;
                    self.primary_mouse_button_pressed = false;
                    None
                }
                _ => None,
            }
        };
        self.tui_app.engine.buffer_area = tui_to_ferrite_rect(self.tui_app.buffer_area);
        if let Some(input) = input {
            self.tui_app
                .engine
                // EventLoopControlFlow is just a dummy value as mouse input should not affect control flow
                .handle_input_command(input, &mut EventLoopControlFlow::Poll);
        }
    }

    pub fn handle_drag(&mut self, drag_column: u16, drag_line: u16) {
        let input = 'block: {
            for (pane_kind, pane_rect) in self
                .tui_app
                .engine
                .workspace
                .panes
                .get_pane_bounds(tui_to_ferrite_rect(self.tui_app.buffer_area))
            {
                if ferrite_to_tui_rect(pane_rect).contains(Position::new(drag_column, drag_line)) {
                    self.tui_app.engine.workspace.panes.make_current(pane_kind);
                    if let PaneKind::Buffer(buffer_id, view_id) = pane_kind {
                        // TODO maybe scroll more of the buffer into view when going outside its bounds
                        if let Some(Point { line, column }) = self.tui_app.drag_start {
                            let buffer = &mut self.tui_app.engine.workspace.buffers[buffer_id];
                            let (_, left_offset) = lines_to_left_offset(buffer.len_lines());

                            let anchor = {
                                let column = column
                                    .saturating_sub(left_offset)
                                    .saturating_sub(pane_rect.x);
                                let line = line.saturating_sub(pane_rect.y);
                                Point::new(column, line)
                            };

                            let cursor = {
                                let column = ((drag_column as usize) + buffer.col_pos(view_id))
                                    .saturating_sub(left_offset)
                                    .saturating_sub(pane_rect.x);
                                let line = (drag_line as usize + buffer.line_pos(view_id))
                                    .saturating_sub(pane_rect.y);
                                Point::new(column, line)
                            };

                            break 'block Some(Cmd::SelectArea { cursor, anchor });
                        }
                    }
                }
            }
            None
        };

        self.tui_app.engine.buffer_area = tui_to_ferrite_rect(self.tui_app.buffer_area);
        if let Some(input) = input {
            self.tui_app
                .engine
                // EventLoopControlFlow is just a dummy value as mouse input should not affect control flow
                .handle_input_command(input, &mut EventLoopControlFlow::Poll);
        }
    }

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

        let theme = &self.tui_app.engine.themes[&self.tui_app.engine.config.editor.theme];

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

            self.tui_app.terminal.backend_mut().prepare(
                &self.device,
                &self.queue,
                &self.tui_app.engine.themes[&self.tui_app.engine.config.editor.theme],
            );

            self.tui_app.terminal.backend_mut().render(&mut rpass);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        self.tui_app.engine.last_render_time =
            Instant::now().duration_since(self.tui_app.engine.start_of_events);

        Ok(())
    }
}

impl Drop for GuiApp {
    fn drop(&mut self) {
        clipboard::uninit();
    }
}
