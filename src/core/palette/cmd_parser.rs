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

    let cmd = match (name, args.as_mut_slice()) {
        ("open", [path]) => Ok(Command::OpenFile(path.take().unwrap().unwrap_path())),
        ("save", [path]) => Ok(Command::SaveFile(path.take().map(|arg| arg.unwrap_path()))),
        ("goto", [line]) => Ok(Command::Goto(line.take().unwrap().unwrap_int())),
        ("indent", []) => Ok(Command::Indent),
        ("reload", []) => Ok(Command::Reload),
        ("logger", []) => Ok(Command::Logger),
        ("quit!", []) => Ok(Command::ForceQuit),
        _ => Err(CommandParseError::UnkownCommand(name.to_string())),
    };

    cmd
}

static COMMANDS: Lazy<Vec<CommandTemplate>> = Lazy::new(|| {
    vec![
        CommandTemplate::new("save", vec![("path", CommandTemplateArg::Path)], 1),
        CommandTemplate::new("open", vec![("path", CommandTemplateArg::Path)], 0),
        CommandTemplate::new("goto", vec![("line", CommandTemplateArg::Int)], 0),
        CommandTemplate::new("indent", vec![], 0),
        CommandTemplate::new("reload", vec![], 0),
        CommandTemplate::new("logger", vec![], 0),
        CommandTemplate::new("quit!", vec![], 0),
    ]
});
