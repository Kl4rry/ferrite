use std::{any::TypeId, collections::HashMap, iter, sync::Arc, time::Instant};

use ferrite_geom::rect::{Rect, Vec2};
use ferrite_runtime::{
    Bounds, Input, Layout, Painter, Runtime, Update, View,
    event_loop_proxy::EventLoopProxy,
    id::Id,
    input::{
        event::{InputEvent, ScrollDelta},
        keycode::{KeyCode, KeyModifiers},
    },
    painter::Rounding,
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
}

pub struct WinitWgpuPlatform<S, UserEvent> {
    state: Option<State>,
    app: Option<App<S, UserEvent>>,
}

impl<S, UserEvent: 'static + Send> WinitWgpuPlatform<S, UserEvent> {
    pub fn new() -> Self {
        Self {
            state: None,
            app: None,
        }
    }

    // TODO: remove
    fn state_mut(&mut self) -> &mut State {
        self.state.as_mut().unwrap()
    }

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

        let (width, height) = (state.config.width, state.config.height);

        {
            let view = (app.layout)(&mut app.runtime.state);
            let bounds = Bounds::new(
                Rect::new(0, 0, width as usize, height as usize),
                Vec2::new(cell_width, cell_height),
                Rounding::Round,
            );
            view.render(&mut app.runtime.state, bounds, &mut state.painter);
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
                terminal.resize(layer.buf.area.into()).unwrap();
            }

            terminal
                .draw(|frame| {
                    frame.buffer_mut().content.clone_from(&layer.buf.content);
                })
                .unwrap();

            terminal
                .backend_mut()
                .overlay_gemoetry
                .clone_from(layer.painter2d.as_ref().unwrap().get_overlay());
        }
        state.painter.clean_up_frame();
        state.terminals.retain(|k, _v| state.touched.contains(k));
    }

    fn render(&mut self) {
        let state = self.state.as_mut().unwrap();

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
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.0,
                            b: 0.0,
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
        let app = self.app.as_mut().unwrap();
        app.runtime.last_render_time = Instant::now().duration_since(app.runtime.start_of_events);
    }

    pub fn run(
        mut self,
        event_loop: EventLoop<PlatformEvent<UserEvent>>,
        runtime: Runtime<S>,
        update: Update<S>,
        input: Input<S, UserEvent>,
        layout: Layout<S>,
    ) {
        self.app = Some(App {
            runtime,
            update,
            input,
            layout,
        });
        event_loop.set_control_flow(ControlFlow::Wait);
        event_loop.run_app(&mut self).unwrap();
    }
}

impl<S, UserEvent: 'static + Send> ApplicationHandler<PlatformEvent<UserEvent>>
    for WinitWgpuPlatform<S, UserEvent>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("Ferrite"))
                .unwrap(),
        );
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
            modifiers: KeyModifiers::empty(),
            touched: Vec::new(),
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
                tracing::trace!("{:?}", event);
                let state = self.state.as_mut().unwrap();
                let app = self.app.as_mut().unwrap();

                if let Key::Named(key) = event.logical_key {
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

                match event.logical_key {
                    Key::Named(key) => {
                        if let Some(keycode) = glue::convert_keycode(key, state.modifiers) {
                            (app.input)(
                                &mut app.runtime.state,
                                InputEvent::Key(keycode, state.modifiers),
                            );
                            return;
                        }
                    }
                    Key::Character(s) => {
                        for ch in s.chars() {
                            (app.input)(
                                &mut app.runtime.state,
                                InputEvent::Key(KeyCode::Char(ch), state.modifiers),
                            );
                        }
                        return;
                    }
                    _ => (),
                }

                if let Some(text) = event.text {
                    (app.input)(&mut app.runtime.state, InputEvent::Text(text.to_string()));
                }
            }
            WindowEvent::MouseInput {
                state: _,
                button: _,
                ..
            } => {
                // TODO
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let app = self.app.as_mut().unwrap();
                let input_event = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        InputEvent::Scroll(ScrollDelta::Line(x, y))
                    }
                    MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition { x, y }) => {
                        InputEvent::Scroll(ScrollDelta::Pixel(x as f32, y as f32))
                    }
                };
                (app.input)(&mut app.runtime.state, input_event);
            }
            WindowEvent::Resized(size) => {
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
                let state = self.state_mut();
                state.scale_factor = scale_factor;
                state.window.request_redraw();
                self.prepare();
            }
            _ => (),
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: PlatformEvent<UserEvent>) {
        match event {
            PlatformEvent::Wake(reason) => tracing::info!("Woken because: {reason}"),
            PlatformEvent::UserEvent(event) => {
                let app = self.app.as_mut().unwrap();
                (app.input)(&mut app.runtime.state, InputEvent::UserEvent(event));
            }
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: StartCause) {
        self.app.as_mut().unwrap().runtime.start_of_events = Instant::now();
        // #[cfg(feature = "talloc")]
        // ferrite_talloc::Talloc::reset_phase_allocations();
        profiling::finish_frame!();
        ferrite_ctx::Ctx::arena_mut().reset();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        {
            let app = self.app.as_mut().unwrap();
            (app.update)(&mut app.runtime);
        }
        self.prepare();
        let state = self.state.as_mut().unwrap();
        if state.terminals.values().any(|t| t.backend().redraw) {
            state.window.request_redraw();
            for terminal in state.terminals.values_mut() {
                terminal.backend_mut().redraw = false;
            }
        }

        event_loop.set_control_flow(ControlFlow::Wait);
    }
}
