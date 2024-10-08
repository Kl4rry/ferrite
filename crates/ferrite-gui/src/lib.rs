use std::{
    env, iter,
    sync::{mpsc, Arc},
    time::Instant,
};

use anyhow::Result;
use event_loop_wrapper::EventLoopProxyWrapper;
use ferrite_cli::Args;
use ferrite_core::{
    cmd::Cmd,
    engine::Engine,
    event_loop_proxy::{EventLoopProxy, UserEvent},
    logger::LogMessage,
};
use gui_renderer::GuiRenderer;
use winit::{
    event::{Event, MouseScrollDelta, WindowEvent},
    event_loop::{EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

mod event_loop_wrapper;
mod gui_renderer;

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
    engine: Engine,
    _proxy: Box<dyn EventLoopProxy>,
    gui_renderer: GuiRenderer,
    // rendering stuff
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f64,
}

impl GuiApp {
    pub async fn new(
        args: &Args,
        event_loop: &EventLoop<UserEvent>,
        rx: mpsc::Receiver<LogMessage>,
    ) -> Result<Self> {
        let proxy = Box::new(EventLoopProxyWrapper::new(event_loop.create_proxy()));
        let engine = Engine::new(args, proxy.dup(), rx)?;

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
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        let gui_renderer = GuiRenderer::new(
            &device,
            &queue,
            surface_format,
            size.width as f32,
            size.height as f32,
        );

        let scale_factor = 1.0;

        window.set_visible(true);

        Ok(Self {
            engine,
            _proxy: proxy,
            gui_renderer,
            window,
            surface,
            device,
            queue,
            config,
            size,
            scale_factor,
        })
    }

    pub fn run(mut self, event_loop: EventLoop<UserEvent>) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        event_loop
            .run(move |event, event_loop| match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::Resized(physical_size) => {
                        self.resize(physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                        self.scale_factor = scale_factor;
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        if let Some((buffer, view_id)) = self.engine.get_current_buffer_mut() {
                            if let MouseScrollDelta::LineDelta(_, y) = delta {
                                buffer
                                    .handle_input(view_id, Cmd::VerticalScroll(-y as i64))
                                    .unwrap();
                            }
                        }
                    }
                    WindowEvent::CloseRequested => event_loop.exit(),
                    WindowEvent::RedrawRequested => match self.render() {
                        Ok(_) => (),
                        Err(wgpu::SurfaceError::Lost) => self.resize(self.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprintln!("Surface error: {:?}", e),
                    },
                    event => self.input(event),
                },
                Event::AboutToWait => {
                    self.window.request_redraw();
                }
                _ => {}
            })
            .unwrap();
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.gui_renderer
            .resize(self.size.width as f32, self.size.height as f32);
    }

    pub fn input(&mut self, _event: WindowEvent) {}

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

        let theme = &self.engine.themes[&self.engine.config.editor.theme];

        {
            let color = theme.background.bg.clone().unwrap_or_default();
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // FIXME this color is not actually correct
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

            if let Some((buffer, view_id)) = self.engine.get_current_buffer_mut() {
                let lines = (self.size.height as f32 / 19.0).ceil() as usize;
                eprintln!("lines: {lines}");
                buffer.set_view_lines(view_id, lines);
                //let view = buffer.get_buffer_view(view_id);
                let start = Instant::now();
                let mut render_input = String::new();
                for chunk in buffer.rope().chunks() {
                    render_input.push_str(chunk);
                }
                eprintln!("text: {:?}", Instant::now().duration_since(start));
                //for line in view.lines {
                //let line = line.text.to_string();
                //render_input.push_str(&line);
                //}
                let start = Instant::now();
                self.gui_renderer.prepare(
                    &self.device,
                    &self.queue,
                    render_input,
                    buffer.views[view_id].line_pos,
                );
                eprintln!("prepare: {:?}", Instant::now().duration_since(start));
                let start = Instant::now();
                self.gui_renderer.render(&mut rpass);
                eprintln!("render: {:?}", Instant::now().duration_since(start));
            }
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
