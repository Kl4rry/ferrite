use once_cell::sync::Lazy;
use utility::line_ending::LineEnding;

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
        ("language", [language]) => Command::Language(language.take().map(|language| language.unwrap_string())),
        ("encoding", [encoding]) => Command::Encoding(encoding.take().map(|encoding| encoding.unwrap_string())),
        ("new", []) => Command::New,
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
        ("line-ending", [line_ending]) => Command::LineEnding(line_ending.take().map(|line_ending| {
            match line_ending.unwrap_string().as_str() {
                "lf" => LineEnding::LF,
                "crlf" => LineEnding::Crlf,
                _ => unreachable!(),
            }
        })),
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
        CommandTemplate::new("language", vec![("language", CommandTemplateArg::String)], 1),
        CommandTemplate::new("encoding", vec![("encoding", CommandTemplateArg::String)], 1),
        CommandTemplate::new("new", vec![], 0),
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
        CommandTemplate::new("line-ending", vec![("line-ending", CommandTemplateArg::Alternatives(vec!["lf".into(), "crlf".into()]))], 1),
    ]
});
