use std::{
    fs::{self, OpenOptions},
    io::{self, LineWriter, Read},
    path::PathBuf,
    process::ExitCode,
};

use anyhow::Result;
use clap::Parser;
use tui_app::event_loop::TuiEventLoop;

mod core;

mod tui_app;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to file that will be opened
    pub file: Option<PathBuf>,
    /// Tail log file
    #[arg(long, name = "log-file")]
    pub log_file: bool,
}

fn main() -> Result<ExitCode> {
    let Some(dirs) = directories::ProjectDirs::from("", "", "ferrite") else {
        eprintln!("Unable to get project directory");
        return Ok(ExitCode::from(1));
    };
    fs::create_dir_all(dirs.data_dir())?;
    let log_file_path = dirs.data_dir().join(".log.txt");
    let log_file = LineWriter::new(
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(&log_file_path)?,
    );
    simplelog::WriteLogger::init(log::LevelFilter::Trace, Default::default(), log_file)?;

    let args = Args::parse();

    if args.log_file {
        let mut child = std::process::Command::new("tail")
            .args(["-fn", "1000", &log_file_path.to_string_lossy()])
            .spawn()?;
        let exit_status = child.wait()?;
        return Ok(ExitCode::from(exit_status.code().unwrap_or(0) as u8));
    }

    {
        let event_loop = TuiEventLoop::new();
        let mut tui_app = tui_app::TuiApp::new(args, event_loop.create_proxy())?;
        if atty::isnt(atty::Stream::Stdin) {
            let mut stdin = io::stdin().lock();
            let mut text = String::new();
            stdin.read_to_string(&mut text)?;
            tui_app.new_buffer_with_text(&text)
        }
        tui_app.run(event_loop)?;
    }

    Ok(ExitCode::from(0))
}
