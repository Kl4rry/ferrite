#![allow(clippy::type_complexity)]
use std::{
    fs::{self, OpenOptions},
    process::ExitCode,
};

use anyhow::Result;
use clap::Parser;
use ferrite_cli::{Args, Subcommands};
use ferrite_core::config::Config;
use tracing::Level;
use tracing_subscriber::{filter, fmt, layer::Layer, prelude::*, Registry};

fn main() -> Result<ExitCode> {
    let Some(dirs) = directories::ProjectDirs::from("", "", "ferrite") else {
        eprintln!("Unable to get project directory");
        return Ok(ExitCode::from(1));
    };
    let log_file_path = dirs.data_dir().join(".log.txt");

    let args = Args::parse();
    if let Some(subcmd) = &args.subcommands {
        match subcmd {
            Subcommands::Init { overwrite } => {
                Config::create_default_config(*overwrite)?;
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
            Subcommands::Log => {
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
        }
    }

    fs::create_dir_all(dirs.data_dir())?;
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

    ferrite_core::clipboard::init(args.local_clipboard);

    ferrite_term::run(&args)?;

    Ok(ExitCode::SUCCESS)
}
