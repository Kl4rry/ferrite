use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

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
    #[command(subcommand)]
    pub subcommands: Option<Subcommands>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Ui {
    Tui,
    Gui,
}

#[derive(Debug, Subcommand)]
pub enum Subcommands {
    /// Initialize default config
    Init {
        /// Overwrite existing config
        #[arg(long)]
        overwrite: bool,
    },
    /// Tail log file
    Log,
}

pub fn parse() -> Args {
    Args::parse()
}
