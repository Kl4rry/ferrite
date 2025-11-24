use std::path::PathBuf;

/// A text editor
#[derive(argh::FromArgs)]
#[argh(help_triggers("-h", "--help"))]
pub struct Args {
    /// path to files that will be opened
    #[argh(positional)]
    pub files: Vec<PathBuf>,
    /// line to open file on
    #[argh(option, short = 'l', long = "line", default = "0")]
    pub line: u32,
    /// use process local clipboard
    #[argh(switch, long = "local-clipboard")]
    pub local_clipboard: bool,
    /// options `error`, `warn`, `info`, `debug` or `trace`
    #[argh(option, long = "log-level")]
    pub log_level: Option<String>,
    /// tui user interface
    #[argh(switch, long = "tui")]
    pub tui: bool,
    /// graphical user interace
    #[argh(switch, long = "gui")]
    pub gui: bool,
    /// tail log file
    #[argh(switch, long = "log")]
    pub log: bool,
    /// initialize default config
    #[argh(switch, long = "init")]
    pub init: bool,
    /// overwrite existing config
    #[argh(switch, long = "overwrite")]
    pub overwrite: bool,
    /// wait for editor to close
    #[argh(switch, short = 'w', long = "wait")]
    pub wait: bool,
    /// enable profiling
    #[argh(switch, long = "profile")]
    pub profile: bool,
}

pub fn parse() -> Args {
    argh::from_env()
}
