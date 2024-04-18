use std::str::FromStr;

use ferrite_utility::line_ending::LineEnding;
use once_cell::sync::Lazy;

use self::generic_cmd::{CommandTemplate, CommandTemplateArg};
use super::cmd::Command;
use crate::{
    buffer::{case::Case, encoding::get_encoding_names},
    language::get_available_languages,
    palette::cmd_parser::generic_cmd::GenericCommand,
    panes::Direction,
};

pub mod lexer;

mod error;
pub mod generic_cmd;
use error::CommandParseError;

pub fn parse_cmd(input: &str) -> Result<Command, CommandParseError> {
    assert!(!input.is_empty());
    let (name, tokens) = lexer::tokenize(input);

    let Some(cmd) = COMMANDS.iter().find(|cmd| cmd.matches(&name.text)) else {
        return Err(CommandParseError::UnkownCommand(name.text));
    };

    let GenericCommand { name, mut args } = cmd.parse_cmd(tokens.into_iter().map(|t| t.text))?;
    let name = name.as_str();

    #[rustfmt::skip]
    let cmd = match (name, args.as_mut_slice()) {
        ("open", [path, ..]) => Command::OpenFile(path.take().unwrap().unwrap_path()),
        ("cd", [path, ..]) => Command::Cd(path.take().unwrap().unwrap_path()),
        ("save", [path, ..]) => Command::SaveFile(path.take().map(|arg| arg.unwrap_path())),
        ("goto", [line, ..]) => Command::Goto(line.take().unwrap().unwrap_int()),
        ("theme", [theme, ..]) => Command::Theme(theme.take().map(|theme| theme.unwrap_string())),
        ("language", [language, ..]) => Command::Language(language.take().map(|language| language.unwrap_string())),
        ("encoding", [encoding, ..]) => Command::Encoding(encoding.take().map(|encoding| encoding.unwrap_string())),
        ("indent", [indent, ..]) => Command::Indent(indent.take().map(|indent| indent.unwrap_string())),
        ("git-reload", [..]) =>  Command::GitReload,
        ("new", [..]) => Command::New,
        ("pwd", [..]) => Command::Pwd,
        ("reload", [..]) => Command::Reload,
        ("logger", [..]) => Command::Logger,
        ("quit!", [..]) => Command::ForceQuit,
        ("quit", [..]) => Command::Quit,
        ("buffers", [..]) => Command::BrowseBuffers,
        ("browse", [..]) => Command::BrowseWorkspace,
        ("config", [..]) => Command::OpenConfig,
        ("close!", [..]) => Command::ForceClose,
        ("close", [..]) => Command::Close,
        ("paste", [..]) => Command::Paste,
        ("copy", [..]) => Command::Copy,
        ("format", [..]) => Command::Format,
        ("format-selection", [..]) => Command::FormatSelection,
        ("revert-buffer", [..]) => Command::RevertBuffer,
        ("delete", [..]) => Command::Delete,
        ("split", [direction, ..]) => {
            Command::Split(Direction::from_str(direction.take().unwrap().unwrap_string().as_str()).unwrap())
        },
        ("shell", args) => {
            let mut paths = Vec::new();
            for arg in args {
                paths.push(arg.take().unwrap().unwrap_path());
            }
            Command::Shell(paths)
        }
        ("case", [case, ..]) =>  {
            Command::Case(Case::from_str(case.take().unwrap().unwrap_string().as_str()).unwrap())
        }
        ("line-ending", [line_ending, ..]) => Command::LineEnding(line_ending.take().map(|line_ending| {
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

pub fn get_command_names() -> Vec<&'static str> {
    COMMANDS.iter().map(|cmd| cmd.name.as_str()).collect()
}

pub fn get_command_input_type(name: &str) -> Option<&'static CommandTemplateArg> {
    COMMANDS
        .iter()
        .find(|cmd| cmd.name == name || cmd.aliases.iter().any(|alias| alias.as_str() == name))
        .map(|cmd| cmd.args.as_ref().map(|(_, input_type)| input_type))
        .unwrap_or_default()
}

#[rustfmt::skip]
static COMMANDS: Lazy<Vec<CommandTemplate>> = Lazy::new(|| {
    let mut cmds = vec![
        CommandTemplate::new("open", Some(("path", CommandTemplateArg::Path)), false).add_alias("o"),
        CommandTemplate::new("cd", Some(("path", CommandTemplateArg::Path)), false),
        CommandTemplate::new("save", Some(("path", CommandTemplateArg::Path)), true).add_alias("s"),
        CommandTemplate::new("goto", Some(("line", CommandTemplateArg::Int)), false).add_alias("g"),
        CommandTemplate::new("theme", Some(("theme", CommandTemplateArg::Theme)), true),
        CommandTemplate::new("new", None, true).add_alias("n"),
        CommandTemplate::new("pwd", None, true),
        CommandTemplate::new("indent", Some(("indent", CommandTemplateArg::String)), true),
        CommandTemplate::new("git-reload", None, true),
        CommandTemplate::new("reload", None, true).add_alias("r"),
        CommandTemplate::new("logger", None, true).add_alias("log"),
        CommandTemplate::new("quit!", None, true).add_alias("q!"),
        CommandTemplate::new("quit", None, true).add_alias("q"),
        CommandTemplate::new("buffers", None, true),
        CommandTemplate::new("browse", None, true),
        CommandTemplate::new("config", None, true),
        CommandTemplate::new("close!", None, true),
        CommandTemplate::new("close", None, true),
        CommandTemplate::new("paste", None, true),
        CommandTemplate::new("copy", None, true),
        CommandTemplate::new("format", None, true),
        CommandTemplate::new("format-selection", None, true),
        CommandTemplate::new("delete", None, true),
        CommandTemplate::new("revert-buffer", None, true).add_alias("rb"),
        CommandTemplate::new("shell", Some(("arg", CommandTemplateArg::Path)), false),
        CommandTemplate::new("split", Some(("direction", CommandTemplateArg::Alternatives(["up", "down", "left", "right"].iter().map(|s| s.to_string()).collect()))), false),
        CommandTemplate::new("case", Some(("encoding", CommandTemplateArg::Alternatives(["lower", "upper", "snake", "kebab", "camel", "pascal", "title", "train", "screaming-snake", "screaming-kebab"].iter().map(|s| s.to_string()).collect()))), false),
        CommandTemplate::new("encoding", Some(("encoding", CommandTemplateArg::Alternatives(get_encoding_names().iter().map(|s| s.to_string()).collect()))), true)
            .set_custom_alternative_error(|encoding, _| format!("`{encoding}` is unknown an encoding, these encodings are supported: https://docs.rs/encoding_rs/latest/encoding_rs")),
        CommandTemplate::new("language", Some(("language", CommandTemplateArg::Alternatives(get_available_languages().iter().map(|s| s.to_string()).collect()))), true).add_alias("lang"),
        CommandTemplate::new("line-ending", Some(("line-ending", CommandTemplateArg::Alternatives(vec!["lf".into(), "crlf".into()]))), true),
    ];
    cmds.sort_by(|cmd1, cmd2| cmd1.name.cmp(&cmd2.name) );
    cmds
});
