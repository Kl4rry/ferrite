use std::{
    fs::{self, OpenOptions},
    process::ExitCode,
    sync::{mpsc, Mutex},
};

use anyhow::Result;
use ferrite_cli::Ui;
use ferrite_core::{
    config::{editor::Editor, keymap::Keymap, languages::Languages},
    logger::LoggerSink,
};
use tracing::Level;
use tracing_subscriber::{filter, fmt, layer::Layer, prelude::*, Registry};

#[cfg(feature = "talloc")]
#[global_allocator]
static GLOBAL: ferrite_talloc::Talloc = ferrite_talloc::Talloc;

fn main() -> Result<ExitCode> {
    let Some(dirs) = directories::ProjectDirs::from("", "", "ferrite") else {
        eprintln!("Unable to get project directory");
        return Ok(ExitCode::from(1));
    };
    let log_file_path = dirs.data_dir().join(".log.txt");

    let args = ferrite_cli::parse();

    if args.init {
        Editor::create_default_config(args.overwrite)?;
        println!(
            "Created default editor config at: `{}`",
            Editor::get_default_location()?.to_string_lossy()
        );

        Languages::create_default_config(args.overwrite)?;
        println!(
            "Created default language config at: `{}`",
            Languages::get_default_location()?.to_string_lossy()
        );

        Keymap::create_default_config(args.overwrite)?;
        println!(
            "Created default keymap at: `{}`",
            Keymap::get_default_location()?.to_string_lossy()
        );

        ferrite_core::theme::init_themes()?;

        return Ok(ExitCode::SUCCESS);
    }

    if args.log {
        let mut cmd = std::process::Command::new("tail");
        cmd.args(["-fn", "1000", &log_file_path.to_string_lossy()]);

        #[cfg(not(target_family = "unix"))]
        {
            let mut child = cmd.spawn()?;
            let exit_status = child.wait()?;
            return Ok(ExitCode::from(exit_status.code().unwrap_or(0) as u8));
        }

        #[cfg(target_family = "unix")]
        {
            use std::os::unix::process::CommandExt;
            Err(cmd.exec())?;
        }
    }

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
        #[cfg(debug_assertions)]
        _ => Level::TRACE,
        #[cfg(not(debug_assertions))]
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

    ferrite_core::clipboard::init(args.local_clipboard);

    #[cfg(not(any(feature = "tui", feature = "gui")))]
    compile_error!("You must enable either tui or gui");

    match args.ui {
        Some(Ui::Tui) => {
            #[cfg(feature = "tui")]
            if let Err(err) = ferrite_tui::run(&args, rx) {
                tracing::error!("{err}");
                return Err(err);
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!("Ferrite has not been compiled with tui");
                return Ok(ExitCode::FAILURE);
            }
        }
        Some(Ui::Gui) => {
            #[cfg(feature = "gui")]
            if let Err(err) = ferrite_gui::run(&args, rx) {
                tracing::error!("{err}");
                return Err(err);
            }
            #[cfg(not(feature = "gui"))]
            {
                eprintln!("Ferrite has not been compiled with gui");
                return Ok(ExitCode::FAILURE);
            }
        }
        None => {
            #[cfg(feature = "tui")]
            ferrite_tui::run(&args, rx)?;
            #[cfg(not(feature = "tui"))]
            ferrite_gui::run(&args)?;
        }
    }

    Ok(ExitCode::SUCCESS)
}
