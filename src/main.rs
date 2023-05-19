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

mod clipboard;
mod tui_app;

/// A text editor
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to file that will be opened
    pub file: Option<PathBuf>,
    /// Line to open file on
    #[arg(long, short, default_value = "0")]
    pub line: u32,
    /// Language
    #[arg(long = "lang")]
    pub language: Option<String>,
    /// Tail log file
    #[arg(long, name = "log-file")]
    pub log_file: bool,
    /// Use process local clipboard
    #[arg(long, name = "local-clipboard")]
    pub local_clipboard: bool,
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
        if atty::isnt(atty::Stream::Stdin) {
            let mut stdin = io::stdin().lock();
            let mut text = String::new();
            stdin.read_to_string(&mut text)?;
            let buffer = tui_app.new_buffer_with_text(&text);
            let (_, height) = crossterm::terminal::size()?;
            buffer.set_view_lines(height.saturating_sub(2).into());
            buffer.goto(args.line as i64);
        }
        tui_app.run(event_loop)?;
    }

    Ok(ExitCode::from(0))
}
