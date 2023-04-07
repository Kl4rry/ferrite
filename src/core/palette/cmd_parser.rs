use once_cell::sync::Lazy;

use self::generic_cmd::{CommandTemplate, CommandTemplateArg};
use super::cmd::Command;
use crate::core::palette::cmd_parser::generic_cmd::GenericCommand;

pub mod lexer;

mod error;
mod generic_cmd;
use error::CommandParseError;

pub fn parse_cmd(input: &str) -> Result<Command, CommandParseError> {
    assert!(!input.is_empty());
    let (name, tokens) = lexer::tokenize(input);

    let Some(cmd) = COMMANDS.iter().find(|cmd| cmd.name == name) else {
        return Err(CommandParseError::UnkownCommand(name));
    };

    let GenericCommand { name, mut args } = cmd.parse_cmd(tokens)?;
    let name = name.as_str();

    #[rustfmt::skip]
    let cmd = match (name, args.as_mut_slice()) {
        ("open", [path]) => Command::OpenFile(path.take().unwrap().unwrap_path()),
        ("save", [path]) => Command::SaveFile(path.take().map(|arg| arg.unwrap_path())),
        ("goto", [line]) => Command::Goto(line.take().unwrap().unwrap_int()),
        ("theme", [theme]) => Command::Theme(theme.take().map(|theme| theme.unwrap_string())),
        ("indent", []) => Command::Indent,
        ("reload", []) => Command::Reload,
        ("logger", []) => Command::Logger,
        ("quit!", []) => Command::ForceQuit,
        ("quit", []) => Command::Quit,
        ("buffers", []) => Command::BrowseBuffers,
        ("browse", []) => Command::BrowseWorkspace,
        ("config", []) => Command::OpenConfig,
        ("close!", []) => Command::ForceClose,
        ("close", []) => Command::Close,
        _ => return Err(CommandParseError::UnkownCommand(name.to_string())),
    };

    Ok(cmd)
}

#[rustfmt::skip]
static COMMANDS: Lazy<Vec<CommandTemplate>> = Lazy::new(|| {
    vec![
        CommandTemplate::new("save", vec![("path", CommandTemplateArg::Path)], 1),
        CommandTemplate::new("open", vec![("path", CommandTemplateArg::Path)], 0),
        CommandTemplate::new("goto", vec![("line", CommandTemplateArg::Int)], 0),
        CommandTemplate::new("theme", vec![("theme", CommandTemplateArg::String)], 1),
        CommandTemplate::new("indent", vec![], 0),
        CommandTemplate::new("reload", vec![], 0),
        CommandTemplate::new("logger", vec![], 0),
        CommandTemplate::new("quit!", vec![], 0),
        CommandTemplate::new("quit", vec![], 0),
        CommandTemplate::new("buffers", vec![], 0),
        CommandTemplate::new("browse", vec![], 0),
        CommandTemplate::new("config", vec![], 0),
        CommandTemplate::new("close!", vec![], 0),
        CommandTemplate::new("close", vec![], 0),
    ]
});
