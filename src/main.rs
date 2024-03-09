#![allow(clippy::type_complexity)]
use std::{
    fs::{self, OpenOptions},
    io::{self, IsTerminal, Read},
    path::PathBuf,
    process::ExitCode,
};

use anyhow::Result;
use clap::Parser;
use ferrite_core::config::Config;
use tracing::Level;
use tracing_subscriber::{filter, fmt, layer::Layer, prelude::*, Registry};
use tui_app::event_loop::TuiEventLoop;

mod ferrite_core;

mod clipboard;
mod tui_app;

/// A text editor
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to files that will be opened
    pub files: Vec<PathBuf>,
    /// Line to open file on
    #[arg(long, short, default_value = "0")]
    pub line: u32,
    /// Language
    #[arg(long = "lang")]
    pub language: Option<String>,
    /// Tail log file
    #[arg(long)]
    pub log_file: bool,
    /// Use process local clipboard
    #[arg(long)]
    pub local_clipboard: bool,
    /// Options `error`, `warn`, `info`, `debug` or `trace`
    #[arg(long)]
    pub log_level: Option<String>,
    /// Initialize default config
    #[arg(long)]
    pub init: bool,
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    if args.init {
        Config::create_default_config()?;
        println!(
            "Created default config at: `{}`",
            Config::get_default_location()?.to_string_lossy()
        );

        #[cfg(feature = "embed-themes")]
        {
            crate::ferrite_core::theme::init_themes()?;
        }

        return Ok(ExitCode::SUCCESS);
    }

    let Some(dirs) = directories::ProjectDirs::from("", "", "ferrite") else {
        eprintln!("Unable to get project directory");
        return Ok(ExitCode::from(1));
    };
    fs::create_dir_all(dirs.data_dir())?;
    let log_file_path = dirs.data_dir().join(".log.txt");
    let log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&log_file_path)?;

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

    let subscriber = Registry::default().with(
        fmt::layer()
            .compact()
            .without_time()
            .with_ansi(true)
            .with_writer(log_file)
            .with_filter(filter::LevelFilter::from_level(log_level)),
    );

    tracing::subscriber::set_global_default(subscriber).unwrap();
    tracing_log::LogTracer::init().unwrap();

    clipboard::init(args.local_clipboard);

    if args.log_file {
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

    {
        let event_loop = TuiEventLoop::new();
        let mut tui_app = tui_app::TuiApp::new(&args, event_loop.create_proxy())?;
        if !io::stdin().is_terminal() {
            let mut stdin = io::stdin().lock();
            let mut text = String::new();
            stdin.read_to_string(&mut text)?;
            let buffer = tui_app.new_buffer_with_text(&text);
            let (_, height) = crossterm::terminal::size()?;
            buffer.set_view_lines(height.saturating_sub(2).into());
            buffer.goto(args.line as i64);
        }

        if !io::stdout().is_terminal() {
            return Ok(ExitCode::SUCCESS);
        }

        tui_app.run(event_loop)?;
    }

    Ok(ExitCode::SUCCESS)
}
