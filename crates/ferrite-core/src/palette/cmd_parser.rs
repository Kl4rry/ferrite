use std::{str::FromStr, sync::LazyLock};

use ferrite_utility::line_ending::LineEnding;
use generic_cmd::CmdBuilder;

use self::generic_cmd::{CmdTemplateArg, CommandTemplate};
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

static COMMANDS: LazyLock<Vec<CommandTemplate>> = LazyLock::new(|| {
    let mut cmds = vec![
        CmdBuilder::new("pwd", None, true).build(|_| Command::Pwd),
        CmdBuilder::new("replace", None, true).build(|_| Command::Replace),
        CmdBuilder::new("search", None, true).build(|_| Command::Search),
        CmdBuilder::new("about", None, true).build(|_| Command::About),
        CmdBuilder::new("path", None, true).build(|_| Command::Path),
        CmdBuilder::new("git-reload", None, true).build(|_| Command::GitReload),
        CmdBuilder::new("git-diff", None, true).build(|_| Command::GitDiff),
        CmdBuilder::new("reload", None, true).add_alias("r").build(|_| Command::Reload),
        CmdBuilder::new("reload-all", None, true).build(|_| Command::ReloadAll),
        CmdBuilder::new("logger", None, true).add_alias("log").build(|_| Command::Logger),
        CmdBuilder::new("quit!", None, true).add_alias("q!").build(|_| Command::ForceQuit),
        CmdBuilder::new("quit", None, true).add_alias("q").build(|_| Command::Quit),
        CmdBuilder::new("buffer-picker", None, true).build(|_| Command::BufferPickerOpen),
        CmdBuilder::new("file-picker", None, true).build(|_| Command::FilePickerOpen),
        CmdBuilder::new("file-picker-reload", None, true).build(|_| Command::FilePickerReload),
        CmdBuilder::new("config", None, true).build(|_| Command::OpenConfig),
        CmdBuilder::new("default-config", None, true).build(|_| Command::DefaultConfig),
        CmdBuilder::new("close!", None, true).build(|_| Command::ForceClose),
        CmdBuilder::new("close", None, true).build(|_| Command::Close),
        CmdBuilder::new("close-pane", None, true).build(|_| Command::ClosePane),
        CmdBuilder::new("paste", None, true).build(|_| Command::Paste),
        CmdBuilder::new("copy", None, true).build(|_| Command::Copy),
        CmdBuilder::new("format", None, true).build(|_| Command::Format),
        CmdBuilder::new("format-selection", None, true).build(|_| Command::FormatSelection),
        CmdBuilder::new("trash", None, true).build(|_| Command::Trash),
        CmdBuilder::new("url-open", None, true).build(|_| Command::UrlOpen),
        CmdBuilder::new("revert-buffer", None, true).add_alias("rb").build(|_| Command::RevertBuffer),
        CmdBuilder::new("open", Some(("path", CmdTemplateArg::Path)), false).add_alias("o").build(|args| Command::OpenFile(args[0].take().unwrap().unwrap_path())),
        CmdBuilder::new("cd", Some(("path", CmdTemplateArg::Path)), false).build(|args| Command::Cd(args[0].take().unwrap().unwrap_path())),
        CmdBuilder::new("save", Some(("path", CmdTemplateArg::Path)), true).add_alias("s").build(|args| Command::SaveFile(args[0].take().map(|arg| arg.unwrap_path()))),
        CmdBuilder::new("goto", Some(("line", CmdTemplateArg::Int)), false).add_alias("g").build(|args| Command::Goto(args[0].take().unwrap().unwrap_int())),
        CmdBuilder::new("theme", Some(("theme", CmdTemplateArg::Theme)), true).build(|args| Command::Theme(args[0].take().map(|theme| theme.unwrap_string()))),
        CmdBuilder::new("new", Some(("path", CmdTemplateArg::Path)), true).add_alias("n").build(|args| Command::New(args[0].take().map(|arg| arg.unwrap_path()))),
        CmdBuilder::new("indent", Some(("indent", CmdTemplateArg::String)), true).build(|args| Command::Indent(args[0].take().map(|indent| indent.unwrap_string()))),
        CmdBuilder::new("replace-all", Some(("replace-all", CmdTemplateArg::String)), false).build(|args| Command::ReplaceAll(args[0].take().unwrap().unwrap_string())),
        CmdBuilder::new("pipe", Some(("arg", CmdTemplateArg::Path)), false).build(|args| {
            let mut paths = Vec::new();
            for arg in args {
                paths.push(arg.take().unwrap().unwrap_path());
            }
            Command::Shell{ args: paths, pipe: true }
        }),
        CmdBuilder::new("shell", Some(("arg", CmdTemplateArg::Path)), false).add_alias("sh").build(|args| {
            let mut paths = Vec::new();
            for arg in args {
                paths.push(arg.take().unwrap().unwrap_path());
            }
            Command::Shell{ args: paths, pipe: false }
        }),
        CmdBuilder::new("sort", Some(("order", CmdTemplateArg::Alternatives(["asc", "desc"].iter().map(|s| s.to_string()).collect()))), true).build(|args| {
            Command::SortLines(args[0].take().map(|o|o.unwrap_string() == "asc").unwrap_or(true))
        }),
        CmdBuilder::new("split", Some(("direction", CmdTemplateArg::Alternatives(["up", "down", "left", "right"].iter().map(|s| s.to_string()).collect()))), false).build(|args| {
            Command::Split(Direction::from_str(args[0].take().unwrap().unwrap_string().as_str()).unwrap())
        }),
        CmdBuilder::new("case", Some(("case", CmdTemplateArg::Alternatives(["lower", "upper", "snake", "kebab", "camel", "pascal", "title", "train", "screaming-snake", "screaming-kebab"].iter().map(|s| s.to_string()).collect()))), false).build(|args| {
            Command::Case(Case::from_str(args[0].take().unwrap().unwrap_string().as_str()).unwrap())
        }),
        CmdBuilder::new("encoding", Some(("encoding", CmdTemplateArg::Alternatives(get_encoding_names().iter().map(|s| s.to_string()).collect()))), true)
            .set_custom_alternative_error(|encoding, _| format!("`{encoding}` is unknown an encoding, these encodings are supported: https://docs.rs/encoding_rs/latest/encoding_rs"))
            .build(|args| {
                Command::Encoding(args[0].take().map(|encoding| encoding.unwrap_string()))
            }),
        CmdBuilder::new("language", Some(("language", CmdTemplateArg::Alternatives(get_available_languages().iter().map(|s| s.to_string()).collect()))), true)
            .add_alias("lang")
            .build(|args| Command::Language(args[0].take().map(|language| language.unwrap_string()))),
        CmdBuilder::new("line-ending", Some(("line-ending", CmdTemplateArg::Alternatives(vec!["lf".into(), "crlf".into()]))), true)
            .build(|args| {
                Command::LineEnding(args[0].take().map(|line_ending| {
                    match line_ending.unwrap_string().as_str() {
                        "lf" => LineEnding::LF,
                        "crlf" => LineEnding::Crlf,
                        _ => unreachable!(),
                    }
                }))
        }),
    ];
    cmds.sort_by(|cmd1, cmd2| cmd1.name.cmp(&cmd2.name));
    cmds
});
