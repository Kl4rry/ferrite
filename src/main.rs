use std::path::PathBuf;

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
    // let mut stdin = io::stdin().lock();
    // let mut content = String::new();
    // stdin.read_to_string(&mut content)?;
    // println!("{content}");

    let args = Args::parse();
    let tui_app = tui_app::TuiApp::new(args)?;
    tui_app.run()?;

    Ok(())
}
