use std::{iter, sync::Arc, time::Instant};

use anyhow::Result;
use event_loop_wrapper::EventLoopProxyWrapper;
use ferrite_cli::Args;
use ferrite_core::{
    engine::Engine,
    event_loop_proxy::{EventLoopProxy, UserEvent},
};
use gui_renderer::GuiRenderer;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

mod event_loop_wrapper;
mod gui_renderer;

pub fn run(args: &Args) -> Result<()> {
    {
        let default_panic = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            println!();
            let _ = std::fs::write("./panic.txt", format!("{info:?}"));
            default_panic(info);
        }));
    }

    let event_loop = EventLoopBuilder::with_user_event().build()?;
    let gui_app = pollster::block_on(GuiApp::new(args, &event_loop))?;
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
    pub async fn new(args: &Args, event_loop: &EventLoop<UserEvent>) -> Result<Self> {
        let proxy = Box::new(EventLoopProxyWrapper::new(event_loop.create_proxy()));
        let engine = Engine::new(args, proxy.dup())?;

        let window = Arc::new(
            WindowBuilder::new()
                .with_title("Ferrite")
                .build(event_loop)
                .unwrap(),
        );
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
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

        let theme = &self.engine.themes[&self.engine.config.theme];

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
                            r: color.r.powf(2.2),
                            g: color.g.powf(2.2),
                            b: color.b.powf(2.2),
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Some(buffer) = self.engine.get_current_buffer() {
                let view = buffer.get_buffer_view();
                let mut render_input = String::new();
                for line in view.lines {
                    let line = line.text.to_string();
                    render_input.push_str(&line);
                }
                let start = Instant::now();
                self.gui_renderer
                    .prepare(&self.device, &self.queue, render_input);
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
