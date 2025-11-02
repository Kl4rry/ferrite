use std::{
    fs::{self, OpenOptions},
    process::ExitCode,
    sync::{Mutex, mpsc},
};

use anyhow::Result;
use ferrite_core::{
    cmd::Cmd,
    engine::Engine,
    event_loop_proxy::UserEvent,
    keymap,
    logger::LoggerSink,
    views::{
        container::Container,
        lens::Lens,
        main_view::MainView,
        palette_view::PaletteView,
        pane_view::PaneView,
        picker_view::{PickerView, TextAlign},
        zstack::ZStack,
    },
};
use ferrite_runtime::{
    Runtime,
    any_view::AnyView,
    event_loop_proxy::EventLoopControlFlow,
    input::event::{InputEvent, ScrollDelta},
};
use ferrite_winit_wgpu_platform::{WinitWgpuPlatform, create_event_loop};
use tracing::Level;
use tracing_subscriber::{Registry, filter, fmt, layer::Layer, prelude::*};

fn update(runtime: &mut Runtime<Engine>) {
    // TODO do something with the control flow result
    runtime.state.do_polling(&mut EventLoopControlFlow::Wait);
    runtime.scale = runtime.state.scale;
    runtime.font_family = runtime.state.config.editor.gui.font_family.clone();
    runtime.font_weight = runtime.state.config.editor.gui.font_weight as u16;
    runtime.state.last_render_time = runtime.last_render_time;
}

fn input(engine: &mut Engine, input: InputEvent<UserEvent>) {
    let cmd = match input {
        InputEvent::Key(key, modifiers) => keymap::get_command_from_input(
            key,
            modifiers,
            engine.get_current_keymappings(),
            engine.get_input_ctx(),
        ),
        InputEvent::Text(text) => Some(Cmd::Insert { text }),
        InputEvent::Paste(text) => Some(Cmd::Insert { text }),
        InputEvent::Scroll(delta) => {
            match delta {
                ScrollDelta::Line(_x, y) => {
                    engine.handle_single_input_command(
                        Cmd::VerticalScroll {
                            distance: -y as f64 * 3.0,
                        },
                        &mut EventLoopControlFlow::Poll,
                    );
                }
                ScrollDelta::Pixel(_x, _y) => todo!(),
            }
            None
        }
        InputEvent::UserEvent(event) => {
            // TODO do something with the control flow result
            engine.handle_app_event(event, &mut EventLoopControlFlow::Wait);
            return;
        }
    };
    if let Some(cmd) = cmd {
        // TODO do something with the control flow result
        engine.handle_input_command(cmd, &mut EventLoopControlFlow::Wait);
    }
}

fn layout(engine: &mut Engine) -> AnyView<Engine> {
    profiling::scope!("layout");
    let theme = engine.themes[&engine.config.editor.theme].clone();
    let config = engine.config.editor.clone();

    let m_x = 5;
    let m_y = 3;
    let mut picker_view: Option<AnyView<Engine>> = None;
    if engine.file_picker.is_some() {
        profiling::scope!("render tui file picker");
        let p = Lens::new(
            PickerView::new(theme.clone(), config.clone(), "Open file"),
            |engine: &mut Engine| engine.file_picker.as_mut().unwrap(),
        );
        picker_view = Some(AnyView::new(Container::new(p).margin(m_x, m_y)));
    } else if engine.buffer_picker.is_some() {
        profiling::scope!("render tui buffer picker");
        let p = Lens::new(
            PickerView::new(theme.clone(), config.clone(), "Open buffer"),
            |engine: &mut Engine| engine.buffer_picker.as_mut().unwrap(),
        );
        picker_view = Some(AnyView::new(Container::new(p).margin(m_x, m_y)));
    } else if engine.global_search_picker.is_some() {
        profiling::scope!("render tui search picker");
        let p = Lens::new(
            PickerView::new(theme.clone(), config.clone(), "Matches")
                .set_text_align(TextAlign::Left),
            |engine: &mut Engine| engine.global_search_picker.as_mut().unwrap(),
        );
        picker_view = Some(AnyView::new(Container::new(p).margin(m_x, m_y)));
    };

    let main_view = AnyView::new(MainView::new(
        PaneView::new(engine),
        PaletteView::new(theme.clone(), config.clone(), engine.palette.has_focus()),
    ));
    match picker_view {
        Some(picker_view) => AnyView::new(ZStack::new(vec![main_view, picker_view])),
        None => main_view,
    }
}

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
    let (event_loop, proxy) = create_event_loop::<UserEvent>();
    let platform = WinitWgpuPlatform::new();
    let engine = Engine::new(&args, proxy, rx)?;
    let runtime = Runtime::new(engine);
    platform.run(event_loop, runtime, update, input, layout);
    Ok(ExitCode::SUCCESS)
}
