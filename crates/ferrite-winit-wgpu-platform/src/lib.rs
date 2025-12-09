use std::{
    any::TypeId,
    collections::HashMap,
    iter,
    sync::Arc,
    time::{Duration, Instant},
};

use ferrite_geom::rect::{Rect, Vec2};
use ferrite_runtime::{
    Bounds, Input, Layout, MouseButton, MouseInterction, MouseInterctionKind, MouseState, Painter,
    Runtime, StartOfFrame, Update, View,
    any_view::AnyView,
    event_loop_proxy::{EventLoopControlFlow, EventLoopProxy},
    id::Id,
    input::{
        event::InputEvent,
        keycode::{KeyCode, KeyModifiers},
    },
    painter::{CursorIcon, Rounding},
};
use tui::Terminal;
use winit::{
    application::ApplicationHandler,
    event::{MouseScrollDelta, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

use crate::{backend::WgpuBackend, renderer::Renderer};

mod backend;
mod glue;
mod renderer;
mod srgb;

struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window: Arc<Window>,
    scale_factor: f64,
    renderer: Renderer,
    terminals: HashMap<(TypeId, Id), Terminal<backend::WgpuBackend>>,
    painter: Painter,
    modifiers: KeyModifiers,
    touched: Vec<(TypeId, Id)>,
    mouse_state: MouseState,
    line_height: f32,
    cursor_zones: Vec<(CursorIcon, Rect)>,
    control_flow: EventLoopControlFlow,
}

pub fn create_event_loop<E: Send>() -> (
    winit::event_loop::EventLoop<PlatformEvent<E>>,
    Box<dyn EventLoopProxy<E>>,
) {
    let event_loop = winit::event_loop::EventLoop::<PlatformEvent<E>>::with_user_event()
        .build()
        .unwrap();
    let proxy = EventLoopProxyWrapper(event_loop.create_proxy());
    (event_loop, Box::new(proxy))
}

pub enum PlatformEvent<E> {
    // String describes reason for waking
    Wake(&'static str),
    UserEvent(E),
}

#[derive(Debug, Clone)]
pub struct EventLoopProxyWrapper<E: 'static>(winit::event_loop::EventLoopProxy<PlatformEvent<E>>);

impl<E> EventLoopProxyWrapper<E> {
    pub fn new(proxy: winit::event_loop::EventLoopProxy<PlatformEvent<E>>) -> Self {
        Self(proxy)
    }
}

impl<E: Send> EventLoopProxy<E> for EventLoopProxyWrapper<E> {
    fn send(&self, event: E) {
        let _ = self.0.send_event(PlatformEvent::UserEvent(event));
    }

    fn request_render(&self, reason: &'static str) {
        let _ = self.0.send_event(PlatformEvent::Wake(reason));
    }

    fn dup(&self) -> Box<dyn EventLoopProxy<E>> {
        Box::new(EventLoopProxyWrapper::new(self.0.clone()))
    }
}

struct App<S, UserEvent> {
    runtime: Runtime<S>,
    update: Update<S>,
    input: Input<S, UserEvent>,
    layout: Layout<S>,
    start_of_frame: StartOfFrame<S>,
    view_tree: AnyView<S>,
}

pub struct WinitWgpuPlatform<S, UserEvent> {
    state: Option<State>,
    app: Option<App<S, UserEvent>>,
    dirty: bool,
}

impl<S, UserEvent: 'static + Send> Default for WinitWgpuPlatform<S, UserEvent> {
    fn default() -> Self {
        Self {
            state: None,
            app: None,
            dirty: true,
        }
    }
}

impl<S, UserEvent: 'static + Send> WinitWgpuPlatform<S, UserEvent> {
    fn state_mut(&mut self) -> &mut State {
        self.state.as_mut().unwrap()
    }

    #[profiling::function]
    fn prepare(&mut self) {
        let state = self.state.as_mut().unwrap();
        let app = self.app.as_mut().unwrap();
        state
            .renderer
            .font_system
            .db_mut()
            .set_monospace_family(app.runtime.font_family.clone());
        let metrics = backend::get_metrics(app.runtime.scale);
        let (cell_width, cell_height) = backend::calculate_cell_size(
            &mut state.renderer.font_system,
            metrics,
            glyphon::Weight(app.runtime.font_weight),
        );
        state.line_height = cell_height;

        let (width, height) = (state.config.width, state.config.height);

        {
            app.view_tree = (app.layout)(&mut app.runtime.state);
            let bounds = create_bounds(
                Vec2::new(width as usize, height as usize),
                Vec2::new(cell_width, cell_height),
            );
            app.view_tree
                .render(&mut app.runtime.state, bounds, &mut state.painter);

            state.cursor_zones.clear();
            state
                .cursor_zones
                .extend_from_slice(state.painter.cursor_zones());
        }

        state.touched.clear();
        for (type_id, id, layer) in state.painter.layers() {
            let layer = layer.lock().unwrap();
            state.touched.push((*type_id, *id));
            let terminal = state.terminals.entry((*type_id, *id)).or_insert_with(|| {
                Terminal::new(WgpuBackend::new(
                    &mut state.renderer.font_system,
                    layer.bounds,
                    app.runtime.font_family.clone(),
                    glyphon::Weight(app.runtime.font_weight),
                ))
                .unwrap()
            });

            let backend = terminal.backend_mut();
            backend.set_font_family(&mut state.renderer.font_system, &app.runtime.font_family);
            backend.set_font_weight(
                &mut state.renderer.font_system,
                glyphon::Weight(app.runtime.font_weight),
            );
            backend.set_scale(
                &mut state.renderer.font_system,
                state.scale_factor as f32 * app.runtime.scale,
            );

            let backend_bounds = terminal.backend().bounds();
            if backend_bounds != layer.bounds {
                terminal.backend_mut().resize(layer.bounds);
                terminal.resize(layer.buf.area).unwrap();
            }

            terminal
                .draw(|frame| {
                    frame.buffer_mut().content.clone_from(&layer.buf.content);
                })
                .unwrap();

            terminal
                .backend_mut()
                .set_overlay(layer.painter2d.as_ref().unwrap().get_overlay());
        }
        state.terminals.retain(|k, _v| state.touched.contains(k));
        state.painter.clean_up_frame();
    }

    #[profiling::function]
    fn render(&mut self) {
        self.dirty = false;
        let state = self.state.as_mut().unwrap();

        // Mark terminals clean because we are drawing
        {
            for terminal in state.terminals.values_mut() {
                terminal.backend_mut().redraw = false;
            }
        }

        // TODO: tmp alloc
        let mut terminals: Vec<_> = state.terminals.iter_mut().collect();
        // Sort into correct render order
        {
            profiling::scope!("sort layers");
            terminals.sort_by_cached_key(|(k, _v)| state.touched.iter().position(|i| i == *k));
        }
        // TODO: tmp alloc
        let mut layers = Vec::new();
        for (_, terminal) in terminals {
            let backend = terminal.backend_mut();
            let view_bounds = backend.bounds().view_bounds();
            let scissor = Rect {
                x: view_bounds.x as u32,
                y: view_bounds.y as u32,
                width: view_bounds.width as u32,
                height: view_bounds.height as u32,
            };
            let bundle = backend.prepare(&mut state.renderer.font_system);
            layers.push(renderer::Layer {
                scissor,
                bundles: vec![bundle],
            });
        }

        state
            .renderer
            .prepare(&state.device, &state.queue, &state.config, layers);

        let output = state.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            profiling::scope!("main render pass");
            let bg = self.app.as_ref().unwrap().runtime.default_bg;
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: srgb_to_linear(bg.r) as f64,
                            g: srgb_to_linear(bg.g) as f64,
                            b: srgb_to_linear(bg.b) as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            state.renderer.render(&mut rpass);
        }

        {
            profiling::scope!("queue submit");
            state.queue.submit(iter::once(encoder.finish()));
        }
        {
            profiling::scope!("present");
            output.present();
        }

        state
            .window
            .set_cursor(get_cursor(&state.cursor_zones, state.mouse_state.position));
        let app = self.app.as_mut().unwrap();
        app.runtime.last_render_time = Instant::now().duration_since(app.runtime.start_of_events);
    }

    pub fn run(
        mut self,
        event_loop: EventLoop<PlatformEvent<UserEvent>>,
        mut runtime: Runtime<S>,
        update: Update<S>,
        input: Input<S, UserEvent>,
        layout: Layout<S>,
        start_of_frame: StartOfFrame<S>,
    ) {
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

        let view_tree = (layout)(&mut runtime.state);

        self.app = Some(App {
            runtime,
            update,
            input,
            layout,
            start_of_frame,
            view_tree,
        });
        event_loop.run_app(&mut self).unwrap();
    }

    fn update_control_flow(&self, event_loop: &ActiveEventLoop) {
        match self.state.as_ref().unwrap().control_flow {
            EventLoopControlFlow::Poll => event_loop.set_control_flow(ControlFlow::Poll),
            EventLoopControlFlow::Wait => event_loop.set_control_flow(ControlFlow::Wait),
            EventLoopControlFlow::Exit => event_loop.exit(),
            EventLoopControlFlow::WaitMax(duration) => {
                event_loop.set_control_flow(ControlFlow::wait_duration(duration))
            }
        }
    }
}

impl<S, UserEvent: 'static + Send> ApplicationHandler<PlatformEvent<UserEvent>>
    for WinitWgpuPlatform<S, UserEvent>
{
    #[profiling::function]
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = {
            profiling::scope!("spawn window");
            Arc::new(
                event_loop
                    .create_window(Window::default_attributes().with_title("Ferrite"))
                    .unwrap(),
            )
        };
        // TODO: This fixes the exit segfault by leaking a Arc<Window> so that
        // the window does not get destoryed
        std::mem::forget(window.clone());
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
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::default(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .unwrap();

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
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        let renderer = Renderer::new(&device, &config, size.width as f32, size.height as f32);

        let terminals = HashMap::new();
        let painter = Painter::new(true);

        self.state = Some(State {
            surface,
            device,
            queue,
            config,
            window,
            scale_factor: 1.0,
            renderer,
            terminals,
            painter,
            cursor_zones: Vec::new(),
            modifiers: KeyModifiers::empty(),
            touched: Vec::new(),
            mouse_state: MouseState::default(),
            line_height: 1.0,
            control_flow: EventLoopControlFlow::Wait,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Focused(false) => self.state_mut().modifiers = KeyModifiers::empty(),
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = self.state_mut();
                let modifiers = modifiers.state();
                state.modifiers.set(
                    KeyModifiers::CONTROL,
                    modifiers.contains(ModifiersState::CONTROL),
                );
                state
                    .modifiers
                    .set(KeyModifiers::ALT, modifiers.contains(ModifiersState::ALT));
                state.modifiers.set(
                    KeyModifiers::SHIFT,
                    modifiers.contains(ModifiersState::SHIFT),
                );
            }
            WindowEvent::KeyboardInput { event, .. } => {
                tracing::debug!("{:?}", event);
                self.dirty = true;

                if let Key::Named(key) = event.logical_key {
                    let state = self.state.as_mut().unwrap();
                    match key {
                        NamedKey::Super => {
                            state
                                .modifiers
                                .set(KeyModifiers::SUPER, event.state.is_pressed());
                            return;
                        }
                        NamedKey::Hyper => {
                            state
                                .modifiers
                                .set(KeyModifiers::HYPER, event.state.is_pressed());
                            return;
                        }
                        NamedKey::Meta => {
                            state
                                .modifiers
                                .set(KeyModifiers::META, event.state.is_pressed());
                            return;
                        }
                        _ => (),
                    }
                }

                if !event.state.is_pressed() {
                    return;
                }

                let modifiers = self.state.as_mut().unwrap().modifiers;
                match event.logical_key {
                    Key::Named(key) => {
                        if let Some(keycode) = glue::convert_keycode(key, modifiers) {
                            let app = self.app.as_mut().unwrap();
                            let state = self.state.as_mut().unwrap();
                            (app.input)(
                                &mut app.runtime.state,
                                InputEvent::Key(keycode, modifiers),
                                &mut state.control_flow,
                            );
                            self.update_control_flow(event_loop);
                            return;
                        }
                    }
                    Key::Character(s) => {
                        for ch in s.chars() {
                            let app = self.app.as_mut().unwrap();
                            let state = self.state.as_mut().unwrap();
                            (app.input)(
                                &mut app.runtime.state,
                                InputEvent::Key(KeyCode::Char(ch), modifiers),
                                &mut state.control_flow,
                            );
                            self.update_control_flow(event_loop);
                        }
                        return;
                    }
                    _ => (),
                }

                if let Some(text) = event.text {
                    let app = self.app.as_mut().unwrap();
                    let state = self.state.as_mut().unwrap();
                    (app.input)(
                        &mut app.runtime.state,
                        InputEvent::Text(text.to_string()),
                        &mut state.control_flow,
                    );
                    self.update_control_flow(event_loop);
                }
            }
            WindowEvent::MouseInput {
                state: element_state,
                button,
                ..
            } => {
                self.dirty = true;
                let button = match button {
                    winit::event::MouseButton::Left => MouseButton::Left,
                    winit::event::MouseButton::Right => MouseButton::Right,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    _ => return,
                };

                let state = self.state.as_mut().unwrap();
                let app = self.app.as_mut().unwrap();

                match element_state {
                    winit::event::ElementState::Released => {
                        let mouse_state = match button {
                            MouseButton::Left => &mut state.mouse_state.left,
                            MouseButton::Right => &mut state.mouse_state.right,
                            MouseButton::Middle => &mut state.mouse_state.middle,
                        };
                        mouse_state.pressed = false;

                        if mouse_state.drag_start.is_some() {
                            mouse_state.drag_start = None;
                            let metrics = backend::get_metrics(app.runtime.scale);
                            let (cell_width, cell_height) = backend::calculate_cell_size(
                                &mut state.renderer.font_system,
                                metrics,
                                glyphon::Weight(app.runtime.font_weight),
                            );

                            let (width, height) = (state.config.width, state.config.height);
                            let bounds = create_bounds(
                                Vec2::new(width as usize, height as usize),
                                Vec2::new(cell_width, cell_height),
                            );

                            let mouse_interaction = MouseInterction {
                                button,
                                kind: MouseInterctionKind::DragStop,
                                cell_size: Vec2::new(cell_width, cell_height),
                                position: state.mouse_state.position,
                                modifiers: state.modifiers,
                            };

                            app.view_tree.handle_mouse(
                                &mut app.runtime.state,
                                bounds,
                                mouse_interaction,
                            );
                        }
                    }
                    winit::event::ElementState::Pressed => {
                        let mouse_state = match button {
                            MouseButton::Left => &mut state.mouse_state.left,
                            MouseButton::Right => &mut state.mouse_state.right,
                            MouseButton::Middle => &mut state.mouse_state.middle,
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

                        let metrics = backend::get_metrics(app.runtime.scale);
                        let (cell_width, cell_height) = backend::calculate_cell_size(
                            &mut state.renderer.font_system,
                            metrics,
                            glyphon::Weight(app.runtime.font_weight),
                        );

                        let (width, height) = (state.config.width, state.config.height);
                        let bounds = create_bounds(
                            Vec2::new(width as usize, height as usize),
                            Vec2::new(cell_width, cell_height),
                        );

                        let mouse_interaction = MouseInterction {
                            button,
                            kind: MouseInterctionKind::Click(mouse_state.clicks),
                            cell_size: Vec2::new(cell_width, cell_height),
                            position: state.mouse_state.position,
                            modifiers: state.modifiers,
                        };

                        app.view_tree.handle_mouse(
                            &mut app.runtime.state,
                            bounds,
                            mouse_interaction,
                        );
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let state = self.state.as_mut().unwrap();
                let app = self.app.as_mut().unwrap();

                // Early return as nothing should be done if we just hovered
                if !state.mouse_state.left.pressed
                    && !state.mouse_state.right.pressed
                    && !state.mouse_state.middle.pressed
                {
                    // We need to set mouse position here as it is set last to allow the last position
                    // to be used as drag start.
                    state.mouse_state.position.x = position.x as f32;
                    state.mouse_state.position.y = position.y as f32;
                    state
                        .window
                        .set_cursor(get_cursor(&state.cursor_zones, state.mouse_state.position));
                    return;
                }

                self.dirty = true;

                let metrics = backend::get_metrics(app.runtime.scale);
                let (cell_width, cell_height) = backend::calculate_cell_size(
                    &mut state.renderer.font_system,
                    metrics,
                    glyphon::Weight(app.runtime.font_weight),
                );

                let (width, height) = (state.config.width, state.config.height);
                let bounds = create_bounds(
                    Vec2::new(width as usize, height as usize),
                    Vec2::new(cell_width, cell_height),
                );

                let last_pos = state.mouse_state.position;
                if state.mouse_state.left.pressed {
                    if state.mouse_state.left.drag_start.is_none() {
                        state.mouse_state.left.drag_start = Some(state.mouse_state.position);
                    }
                    let mouse_interaction = MouseInterction {
                        button: MouseButton::Left,
                        kind: MouseInterctionKind::Drag {
                            drag_start: state.mouse_state.left.drag_start.unwrap(),
                            last_pos,
                        },
                        cell_size: Vec2::new(cell_width, cell_height),
                        position: Vec2::new(position.x as f32, position.y as f32),
                        modifiers: state.modifiers,
                    };
                    app.view_tree
                        .handle_mouse(&mut app.runtime.state, bounds, mouse_interaction);
                }
                if state.mouse_state.right.pressed {
                    if state.mouse_state.right.drag_start.is_none() {
                        state.mouse_state.right.drag_start = Some(state.mouse_state.position);
                    }
                    let mouse_interaction = MouseInterction {
                        button: MouseButton::Right,
                        kind: MouseInterctionKind::Drag {
                            drag_start: state.mouse_state.right.drag_start.unwrap(),
                            last_pos,
                        },
                        cell_size: Vec2::new(cell_width, cell_height),
                        position: Vec2::new(position.x as f32, position.y as f32),
                        modifiers: state.modifiers,
                    };
                    app.view_tree
                        .handle_mouse(&mut app.runtime.state, bounds, mouse_interaction);
                }
                if state.mouse_state.middle.pressed {
                    if state.mouse_state.middle.drag_start.is_none() {
                        state.mouse_state.middle.drag_start = Some(state.mouse_state.position);
                    }
                    let mouse_interaction = MouseInterction {
                        button: MouseButton::Middle,
                        kind: MouseInterctionKind::Drag {
                            drag_start: state.mouse_state.middle.drag_start.unwrap(),
                            last_pos,
                        },
                        cell_size: Vec2::new(cell_width, cell_height),
                        position: Vec2::new(position.x as f32, position.y as f32),
                        modifiers: state.modifiers,
                    };
                    app.view_tree
                        .handle_mouse(&mut app.runtime.state, bounds, mouse_interaction);
                }
                state.mouse_state.position.x = position.x as f32;
                state.mouse_state.position.y = position.y as f32;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.dirty = true;
                let app = self.app.as_mut().unwrap();
                let state = self.state.as_mut().unwrap();
                let input_event = match delta {
                    MouseScrollDelta::LineDelta(x, y) => InputEvent::Scroll(x, y),
                    MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition { x: _, y }) => {
                        let distance = y as f32 / state.line_height;
                        InputEvent::Scroll(0.0, distance)
                    }
                };
                (app.input)(&mut app.runtime.state, input_event, &mut state.control_flow);
                self.update_control_flow(event_loop);
            }
            WindowEvent::Resized(size) => {
                profiling::scope!("resized");
                self.dirty = true;
                let state = self.state_mut();
                state.config.width = size.width;
                state.config.height = size.height;
                state.surface.configure(&state.device, &state.config);
                state.renderer.resize(size.width as f32, size.height as f32);
                state.window.request_redraw();
                self.prepare();
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer: _,
            } => {
                profiling::scope!("scale factor changed");
                self.dirty = true;
                let state = self.state_mut();
                state.scale_factor = scale_factor;
                state.window.request_redraw();
                self.prepare();
            }
            _ => (),
        }
    }

    #[profiling::function]
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: PlatformEvent<UserEvent>) {
        self.dirty = true;
        match event {
            PlatformEvent::Wake(reason) => tracing::info!("Woken because: {reason}"),
            PlatformEvent::UserEvent(event) => {
                let app = self.app.as_mut().unwrap();
                let state = self.state.as_mut().unwrap();
                (app.input)(
                    &mut app.runtime.state,
                    InputEvent::UserEvent(event),
                    &mut state.control_flow,
                );
                self.update_control_flow(event_loop);
            }
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        let app = self.app.as_mut().unwrap();
        app.runtime.start_of_events = Instant::now();
        (app.start_of_frame)(&mut app.runtime);
    }

    #[profiling::function]
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        {
            let app = self.app.as_mut().unwrap();
            let state = self.state.as_mut().unwrap();
            (app.update)(&mut app.runtime, &mut state.control_flow);
            self.update_control_flow(event_loop);
        }
        self.prepare();
        let state = self.state.as_mut().unwrap();
        if state.terminals.values().any(|t| t.backend().redraw) {
            state.window.request_redraw();
        }
    }
}

pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn create_bounds(window_size: Vec2, cell_size: Vec2<f32>) -> Bounds {
    Bounds::new(
        Rect::new(0, 0, window_size.x.saturating_sub(1).max(1), window_size.y),
        cell_size,
        Rounding::Round,
    )
}

fn get_cursor(
    cursor_zones: &[(CursorIcon, Rect)],
    position: Vec2<f32>,
) -> winit::window::CursorIcon {
    let position = Vec2::new(position.x as usize, position.y as usize);
    let mut last_icon = CursorIcon::Default;
    for (icon, rect) in cursor_zones {
        if rect.contains(position) {
            last_icon = *icon;
        }
    }
    glue::convert_cursor_icon(last_icon)
}
