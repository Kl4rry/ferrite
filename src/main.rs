use std::{
    fs::OpenOptions,
    io::{self, LineWriter, Read},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;

mod core;

mod tui_app;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to file that will be opened
    pub file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let log_file = LineWriter::new(
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(".log.txt")?,
    );
    simplelog::WriteLogger::init(log::LevelFilter::Trace, Default::default(), log_file)?;

    let args = Args::parse();
    let mut tui_app = tui_app::TuiApp::new(args)?;
    if atty::isnt(atty::Stream::Stdin) {
        let mut stdin = io::stdin().lock();
        let mut text = String::new();
        stdin.read_to_string(&mut text)?;
        tui_app.new_buffer_with_text(&text)
    }
    tui_app.run()?;

    Ok(())
}
