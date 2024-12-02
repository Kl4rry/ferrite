use std::{
    env, iter,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

use anyhow::Result;
use backend::WgpuBackend;
use event_loop_wrapper::EventLoopProxyWrapper;
use ferrite_cli::Args;
use ferrite_core::{
    clipboard,
    cmd::Cmd,
    event_loop_proxy::{EventLoopControlFlow, UserEvent},
    keymap::{self, keycode::KeyModifiers},
    logger::LogMessage,
};
use ferrite_tui::TuiApp;
use ferrite_utility::line_ending::LineEnding;
use glue::convert_keycode;
use winit::{
    event::{Event, MouseScrollDelta, WindowEvent},
    event_loop::{EventLoop, EventLoopBuilder, EventLoopWindowTarget},
    keyboard::Key,
    window::{Window, WindowBuilder},
};

mod backend;
mod event_loop_wrapper;
mod glue;

pub fn run(args: &Args, rx: mpsc::Receiver<LogMessage>) -> Result<()> {
    {
        let default_panic = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            println!();
            let _ = std::fs::write("./panic.txt", format!("{info:?}"));
            default_panic(info);
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
            surface_format,
            size.width as f32,
            size.height as f32,
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
        })
    }

    pub fn run(mut self, event_loop: EventLoop<UserEvent>) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        event_loop
            .run(move |event, event_loop| match event {
                Event::NewEvents(cause) => {
                    self.control_flow = EventLoopControlFlow::Wait;
                    self.tui_app.start_of_events();
                    eprintln!("NEW EVENTS: {:?}\n", cause);
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
                        Err(e) => eprintln!("Surface error: {:?}", e),
                    },
                    event => self.input(event_loop, event),
                },
                Event::AboutToWait => {
                    self.window.request_redraw();
                    self.tui_app.engine.do_polling(&mut self.control_flow);
                    eprintln!("{:?}", self.control_flow);
                    match self.control_flow {
                        EventLoopControlFlow::Poll => {
                            //event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
                        }
                        EventLoopControlFlow::Wait => {
                            //event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
                        }
                        EventLoopControlFlow::Exit => event_loop.exit(),
                        EventLoopControlFlow::WaitMax(duration) => {
                            if duration > Duration::from_secs(10000) {
                                //event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
                            } else {
                                //event_loop.set_control_flow(
                                //    winit::event_loop::ControlFlow::wait_duration(duration),
                                //);
                            }
                        }
                    }
                }
                event => {
                    eprintln!("{:?}", event);
                }
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
    }

    pub fn input(&mut self, event_loop: &EventLoopWindowTarget<UserEvent>, event: WindowEvent) {
        match event {
            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size);
                self.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some((buffer, view_id)) = self.tui_app.engine.get_current_buffer_mut() {
                    if let MouseScrollDelta::LineDelta(_, y) = delta {
                        buffer
                            .handle_input(view_id, Cmd::VerticalScroll(-y as i64 * 3))
                            .unwrap();
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                tracing::trace!("{:?}", event);
                let mut control_flow = self.control_flow;
                if event.state.is_pressed() {
                    match event.logical_key {
                        Key::Named(key) => {
                            match key {
                                winit::keyboard::NamedKey::Control => {
                                    self.modifiers |= KeyModifiers::CONTROL
                                }
                                winit::keyboard::NamedKey::Alt => {
                                    self.modifiers |= KeyModifiers::ALT
                                }
                                winit::keyboard::NamedKey::Shift => {
                                    self.modifiers |= KeyModifiers::SHIFT
                                }
                                winit::keyboard::NamedKey::Super => {
                                    self.modifiers |= KeyModifiers::SUPER
                                }
                                winit::keyboard::NamedKey::Hyper => {
                                    self.modifiers |= KeyModifiers::HYPER
                                }
                                winit::keyboard::NamedKey::Meta => {
                                    self.modifiers |= KeyModifiers::META
                                }
                                _ => (),
                            }
                            if let Some(keycode) = convert_keycode(key) {
                                let cmd = keymap::get_command_from_input(
                                    keycode,
                                    self.modifiers,
                                    self.tui_app.engine.get_current_keymappings(),
                                );
                                if let Some(cmd) = cmd {
                                    self.tui_app
                                        .engine
                                        .handle_input_command(cmd, &mut control_flow);
                                    if control_flow == EventLoopControlFlow::Exit {
                                        event_loop.exit();
                                    }
                                    return;
                                }
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
                                if let Some(cmd) = cmd {
                                    self.tui_app
                                        .engine
                                        .handle_input_command(cmd, &mut control_flow);
                                    if control_flow == EventLoopControlFlow::Exit {
                                        event_loop.exit();
                                    }
                                    return;
                                }
                            } else {
                                self.tui_app.engine.handle_input_command(
                                    Cmd::Insert(s.to_string()),
                                    &mut control_flow,
                                );
                                if control_flow == EventLoopControlFlow::Exit {
                                    event_loop.exit();
                                }
                                return;
                            };
                        }
                        _ => (),
                    }
                } else {
                    if let Key::Named(key) = event.logical_key {
                        match key {
                            winit::keyboard::NamedKey::Control => {
                                self.modifiers.remove(KeyModifiers::CONTROL)
                            }
                            winit::keyboard::NamedKey::Alt => {
                                self.modifiers.remove(KeyModifiers::ALT)
                            }
                            winit::keyboard::NamedKey::Shift => {
                                self.modifiers.remove(KeyModifiers::SHIFT)
                            }
                            winit::keyboard::NamedKey::Super => {
                                self.modifiers.remove(KeyModifiers::SUPER)
                            }
                            winit::keyboard::NamedKey::Hyper => {
                                self.modifiers.remove(KeyModifiers::HYPER)
                            }
                            winit::keyboard::NamedKey::Meta => {
                                self.modifiers.remove(KeyModifiers::META)
                            }
                            _ => (),
                        }
                    }
                }
            }
            _ => (),
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
            let color = theme.background.bg.clone().unwrap_or_default();
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: color.r.powf(2.2) as f64,
                            g: color.g.powf(2.2) as f64,
                            b: color.b.powf(2.2) as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.tui_app.render();

            let start = Instant::now();
            self.tui_app.terminal.backend_mut().prepare(
                &self.device,
                &self.queue,
                &self.tui_app.engine.themes[&self.tui_app.engine.config.editor.theme],
            );
            eprintln!("prepare: {:?}", Instant::now().duration_since(start));

            let start = Instant::now();
            self.tui_app.terminal.backend_mut().render(&mut rpass);
            eprintln!("prepare: {:?}", Instant::now().duration_since(start));
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
        let _ = clipboard::uninit();
    }
}
