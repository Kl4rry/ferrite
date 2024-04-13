use std::path::PathBuf;

use clap::{Parser, Subcommand};

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
    /// Use process local clipboard
    #[arg(long)]
    pub local_clipboard: bool,
    /// Options `error`, `warn`, `info`, `debug` or `trace`
    #[arg(long)]
    pub log_level: Option<String>,
    #[command(subcommand)]
    pub subcommands: Option<Subcommands>,
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
