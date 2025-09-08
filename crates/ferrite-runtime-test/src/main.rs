use std::{
    fs::{self, OpenOptions},
    iter,
    process::ExitCode,
    sync::{Arc, Mutex, mpsc},
};

use anyhow::Result;
use ferrite_core::{
    engine::Engine,
    event_loop_proxy::{EventLoopProxy, UserEvent},
    logger::LoggerSink,
};
use ferrite_runtime::{Layout, Platform, Runtime, Update};
use tracing::Level;
use tracing_subscriber::{Registry, filter, fmt, layer::Layer, prelude::*};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};
struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    scale_factor: f64,
}

#[derive(Debug, Clone)]
pub struct EventLoopProxyWrapper<E: 'static>(winit::event_loop::EventLoopProxy<E>);

impl<E> EventLoopProxyWrapper<E> {
    pub fn new(proxy: winit::event_loop::EventLoopProxy<E>) -> Self {
        Self(proxy)
    }
}

impl<E: Send> EventLoopProxy<E> for EventLoopProxyWrapper<E> {
    fn send(&self, event: E) {
        let _ = self.0.send_event(event);
    }

    fn request_render(&self) {
        // TODO: FIX
        // let _ = self.0.send_event(UserEvent::Wake);
    }

    fn dup(&self) -> Box<dyn EventLoopProxy<E>> {
        Box::new(EventLoopProxyWrapper(self.0.clone()))
    }
}

struct App<S> {
    runtime: Runtime<S>,
    update: Update<S>,
    layout: Layout<S>,
}

pub struct WinitWgpuPlatform<S> {
    state: Option<State>,
    app: Option<App<S>>,
}

impl<S> WinitWgpuPlatform<S> {
    pub fn new() -> Self {
        Self {
            state: None,
            app: None,
        }
    }

    fn state_ref(&self) -> &State {
        self.state.as_ref().unwrap()
    }

    fn state_mut(&mut self) -> &mut State {
        self.state.as_mut().unwrap()
    }

    fn render(&mut self) {
        let state = self.state_mut();
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
            let mut _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.5 as f64,
                            g: 0.2 as f64,
                            b: 0.5 as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        {
            profiling::scope!("queue submit");
            state.queue.submit(iter::once(encoder.finish()));
        }
        {
            profiling::scope!("present");
            output.present();
        }

        let state = self.state_mut();
        state.scale_factor += 1.0;
        println!("render {}", state.scale_factor);
    }

    pub fn run<E: 'static + Send>(
        mut self,
        event_loop: EventLoop<E>,
        runtime: Runtime<S>,
        update: Update<S>,
        layout: Layout<S>,
    ) {
        self.app = Some(App {
            runtime,
            update,
            layout,
        });
        event_loop.set_control_flow(ControlFlow::Wait);
        event_loop.run_app(&mut self).unwrap();
    }
}

impl<S, E: 'static + Send> ApplicationHandler<E> for WinitWgpuPlatform<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        self.state = Some(State {
            surface,
            device,
            queue,
            config,
            size,
            window,
            scale_factor: 1.0,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Figure out if redrawing is needed
                self.render();
            }
            WindowEvent::Resized(size) => {
                let state = self.state_mut();
                state.config.width = size.width;
                state.config.height = size.height;
                state.surface.configure(&state.device, &state.config);
                state.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer: _,
            } => {
                let state = self.state_mut();
                state.scale_factor = scale_factor;
                state.window.request_redraw();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

fn update(state: &mut Engine) {}
fn layout(state: &mut Engine) {}

fn main() -> Result<ExitCode> {
    let args = ferrite_cli::parse();

    let Some(dirs) = directories::ProjectDirs::from("", "", "ferrite") else {
        eprintln!("Unable to get project directory");
        return Ok(ExitCode::from(1));
    };
    let log_file_path = dirs.data_dir().join(".log.txt");

    fs::create_dir_all(dirs.data_dir())?;
    let log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&log_file_path)?;

    const GB: u64 = 1_000_000_000;
    if log_file.metadata()?.len() > GB {
        log_file.set_len(0)?;
        tracing::warn!("Log file was truncated as it reached 1Gb in size");
    }

    let var = args
        .log_level
        .as_ref()
        .cloned()
        .unwrap_or_else(|| std::env::var("FERRITE_LOG").unwrap_or_default());
    let log_level = match var.to_ascii_lowercase().as_str() {
        "error" => Level::ERROR,
        "warn" => Level::WARN,
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    let (tx, rx) = mpsc::channel();
    let logger = LoggerSink::new(tx);

    let subscriber = Registry::default()
        .with(
            fmt::layer()
                .compact()
                .without_time()
                .with_ansi(true)
                .with_writer(log_file)
                .with_filter(filter::LevelFilter::from_level(log_level)),
        )
        .with(
            fmt::layer()
                .json()
                .with_writer(Mutex::new(logger))
                .with_filter(filter::LevelFilter::from_level(log_level)),
        );

    tracing::subscriber::set_global_default(subscriber).unwrap();
    tracing_log::LogTracer::init().unwrap();

    // New stuff
    let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event().build()?;
    let proxy = Box::new(EventLoopProxyWrapper(event_loop.create_proxy()));
    let platform = WinitWgpuPlatform::new();
    let engine = Engine::new(&args, proxy, rx)?;
    let runtime = Runtime::new(engine);
    platform.run(event_loop, runtime, update, layout);
    Ok(ExitCode::SUCCESS)
}
