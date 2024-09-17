use self::generic_cmd::CmdTemplateArg;
use crate::{cmd::Cmd, palette::cmd_parser::generic_cmd::GenericCommand};

pub mod cmds;
pub mod lexer;
use cmds::COMMANDS;

mod error;
pub mod generic_cmd;
use error::CommandParseError;

pub fn parse_cmd(input: &str) -> Result<Cmd, CommandParseError> {
    assert!(!input.is_empty());
    let (name, tokens) = lexer::tokenize(input);

    let Some(cmd) = COMMANDS.iter().find(|cmd| cmd.matches(&name.text)) else {
        return Err(CommandParseError::UnkownCommand(name.text));
    };

    let GenericCommand { mut args, .. } = cmd.parse_cmd(tokens.into_iter().map(|t| t.text))?;
    let cmd = cmd.to_cmd(args.as_mut_slice());
    Ok(cmd)
}

pub fn get_command_names() -> Vec<&'static str> {
    COMMANDS.iter().map(|cmd| cmd.name.as_str()).collect()
}

pub fn get_command_input_type(name: &str) -> Option<&'static CmdTemplateArg> {
    COMMANDS
        .iter()
        .find(|cmd| cmd.name == name || cmd.aliases.iter().any(|alias| alias.as_str() == name))
        .map(|cmd| cmd.args.as_ref().map(|(_, input_type)| input_type))
        .unwrap_or_default()
}
