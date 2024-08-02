use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// A text editor
#[derive(Parser, Debug)]
#[command(name = "ferrite", version, about, long_about = None)]
pub struct Args {
    /// Path to files that will be opened
    pub files: Vec<PathBuf>,
    /// Line to open file on
    #[arg(long, short, default_value = "0")]
    pub line: u32,
    /// Language
    #[arg(long = "lang")]
    pub language: Option<String>,
    /// Use process local clipboard
    #[arg(long)]
    pub local_clipboard: bool,
    /// Options `error`, `warn`, `info`, `debug` or `trace`
    #[arg(long)]
    pub log_level: Option<String>,
    /// Type UI to use
    #[arg(long)]
    pub ui: Option<Ui>,
    /// Tail log file
    #[arg(long)]
    pub log: bool,
    /// Initialize default config
    #[arg(long)]
    pub init: bool,
    /// Overwrite existing config
    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Ui {
    Tui,
    Gui,
}

pub fn parse() -> Args {
    Args::parse()
}
