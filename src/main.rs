use std::{
    fs::{self, OpenOptions},
    process::ExitCode,
    sync::{mpsc, Mutex},
};

use anyhow::Result;
use ferrite_cli::Ui;
use ferrite_core::{
    config::{editor::Editor, languages::Languages},
    logger::{LogMessage, LoggerSink},
};
use tracing::Level;
use tracing_subscriber::{filter, fmt, layer::Layer, prelude::*, Registry};

#[cfg(feature = "talloc")]
#[global_allocator]
static GLOBAL: ferrite_talloc::Talloc = ferrite_talloc::Talloc;

#[cfg(not(target_os = "windows"))]
fn maybe_disown(args: &ferrite_cli::Args) {
    use std::{env, io::IsTerminal, process};
    if args.wait || !std::io::stdout().is_terminal() {
        return;
    }
    if let Ok(current_exe) = env::current_exe() {
        let child = process::Command::new(&current_exe)
            .stdin(process::Stdio::null())
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .args(env::args().skip(1))
            .spawn();
        assert!(child.is_ok());
        process::exit(0);
    } else {
        eprintln!("error in disowning process, cannot obtain the path for the current executable, continuing without disowning...");
    }
}

#[cfg(feature = "tui")]
fn run_tui(args: &ferrite_cli::Args, rx: mpsc::Receiver<LogMessage>) -> Result<()> {
    if let Err(err) = ferrite_term::run(args, rx) {
        tracing::error!("{err}");
        return Err(err);
    }
    Ok(())
}

#[cfg(feature = "gui")]
fn run_gui(args: &ferrite_cli::Args, rx: mpsc::Receiver<LogMessage>) -> Result<()> {
    #[cfg(not(target_os = "windows"))]
    maybe_disown(args);
    if let Err(err) = ferrite_gui::run(args, rx) {
        tracing::error!("{err}");
        return Err(err);
    }
    Ok(())
}

fn main() -> Result<ExitCode> {
    let Some(dirs) = directories::ProjectDirs::from("", "", "ferrite") else {
        eprintln!("Unable to get project directory");
        return Ok(ExitCode::from(1));
    };
    let log_file_path = dirs.data_dir().join(".log.txt");

    let args = ferrite_cli::parse();

    if args.init {
        Editor::create_default_config(args.overwrite)?;
        eprintln!(
            "Created default editor config at: `{}`",
            Editor::get_default_location()?.to_string_lossy()
        );

        Languages::create_default_config(args.overwrite)?;
        eprintln!(
            "Created default language config at: `{}`",
            Languages::get_default_location()?.to_string_lossy()
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

    let _puffin_server = if args.profile {
        let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
        let puffin_server = puffin_http::Server::new(&server_addr).unwrap();
        eprintln!("Run this to view profiling data:  puffin_viewer {server_addr}");
        puffin::set_scopes_on(true);
        Some(puffin_server)
    } else {
        None
    };

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
            run_tui(&args, rx)?;

            #[cfg(not(feature = "tui"))]
            {
                eprintln!("Ferrite has not been compiled with tui");
                return Ok(ExitCode::FAILURE);
            }
            return Ok(ExitCode::SUCCESS);
        }
        Some(Ui::Gui) => {
            #[cfg(feature = "gui")]
            run_gui(&args, rx)?;

            #[cfg(not(feature = "gui"))]
            {
                eprintln!("Ferrite has not been compiled with gui");
                return Ok(ExitCode::FAILURE);
            }
            return Ok(ExitCode::SUCCESS);
        }
        _ => {
            #[cfg(feature = "gui")]
            if std::env::var("WAYLAND_DISPLAY").is_ok() {
                run_gui(&args, rx)?;
                return Ok(ExitCode::SUCCESS);
            }

            #[cfg(feature = "tui")]
            if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
                ferrite_term::run(&args, rx)?;
                return Ok(ExitCode::SUCCESS);
            } else {
                #[cfg(not(feature = "gui"))]
                anyhow::bail!("stdout must is not a tty");
            }

            #[cfg(feature = "gui")]
            run_gui(&args, rx)?;
        }
    }

    Ok(ExitCode::SUCCESS)
}
