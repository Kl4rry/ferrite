use std::{str::FromStr, sync::LazyLock};

use ferrite_utility::line_ending::LineEnding;

use super::generic_cmd::{CmdBuilder, CmdTemplateArg, CommandTemplate};
use crate::{
    buffer::{case::Case, encoding::get_encoding_names},
    cmd::Cmd,
    language::get_available_languages,
    layout::panes::Direction,
};

pub static COMMANDS: LazyLock<Vec<CommandTemplate>> = LazyLock::new(|| {
    let mut cmds = vec![
        CmdBuilder::new("force-redraw", None, true).build(|_| Cmd::ForceRedraw),
        CmdBuilder::new("pwd", None, true).build(|_| Cmd::Pwd),
        CmdBuilder::new("replace", None, true).build(|_| Cmd::Replace),
        CmdBuilder::new("search", None, true).build(|_| Cmd::Search),
        CmdBuilder::new("about", None, true).build(|_| Cmd::About),
        CmdBuilder::new("path", None, true).build(|_| Cmd::Path),
        CmdBuilder::new("git-reload", None, true).build(|_| Cmd::GitReload),
        CmdBuilder::new("reload", None, true).build(|_| Cmd::Reload),
        CmdBuilder::new("reload-all", None, true).build(|_| Cmd::ReloadAll),
        CmdBuilder::new("logger", None, true).add_alias("log").build(|_| Cmd::Logger),
        CmdBuilder::new("quit!", None, true).add_alias("q!").build(|_| Cmd::ForceQuit),
        CmdBuilder::new("quit", None, true).add_alias("q").build(|_| Cmd::Quit),
        CmdBuilder::new("buffer-picker", None, true).build(|_| Cmd::BufferPickerOpen),
        CmdBuilder::new("file-picker", None, true).build(|_| Cmd::FilePickerOpen),
        CmdBuilder::new("file-picker-reload", None, true).build(|_| Cmd::FilePickerReload),
        CmdBuilder::new("open-config", None, true).build(|_| Cmd::OpenConfig),
        CmdBuilder::new("default-config", None, true).build(|_| Cmd::DefaultConfig),
        CmdBuilder::new("open-languages", None, true).build(|_| Cmd::OpenLanguages),
        CmdBuilder::new("default-languages", None, true).build(|_| Cmd::DefaultLanguages),
        CmdBuilder::new("open-keymap", None, true).build(|_| Cmd::OpenKeymap),
        CmdBuilder::new("default-keymap", None, true).build(|_| Cmd::DefaultKeymap),
        CmdBuilder::new("open-workspace-config", None, true).build(|_| Cmd::OpenWorkspaceConfig),
        CmdBuilder::new("close!", None, true).build(|_| Cmd::ForceClose),
        CmdBuilder::new("close", None, true).build(|_| Cmd::Close),
        CmdBuilder::new("close-pane", None, true).build(|_| Cmd::ClosePane),
        CmdBuilder::new("paste", None, true).build(|_| Cmd::Paste),
        CmdBuilder::new("copy", None, true).build(|_| Cmd::Copy),
        CmdBuilder::new("cut", None, true).build(|_| Cmd::Cut),
        CmdBuilder::new("format", None, true).build(|_| Cmd::Format),
        CmdBuilder::new("format-selection", None, true).build(|_| Cmd::FormatSelection),
        CmdBuilder::new("url-open", None, true).build(|_| Cmd::UrlOpen),
        CmdBuilder::new("save-all", None, true).build(|_| Cmd::SaveAll),
        CmdBuilder::new("zoom-reset", None, true).build(|_| Cmd::ResetZoom),
        CmdBuilder::new("kill-job", None, true).build(|_| Cmd::KillJob),
        CmdBuilder::new("trim-trailing-whitespace", None, true).build(|_| Cmd::TrimTrailingWhitespace),
        CmdBuilder::new("run", Some(("action", CmdTemplateArg::Action)), false).add_alias("r").build(|args| Cmd::RunAction { name: args[0].take().unwrap().unwrap_string() }),
        CmdBuilder::new("open-file-explorer", Some(("path", CmdTemplateArg::Path)), true).build(|args| Cmd::OpenFileExplorer { path: args[0].take().map(|arg| arg.unwrap_path()) }),
        CmdBuilder::new("number", Some(("start", CmdTemplateArg::Int)), true).build(|args| Cmd::Number { start: args[0].take().map(|arg| arg.unwrap_int()) }),
        CmdBuilder::new("revert-buffer", None, true).add_alias("rb").build(|_| Cmd::RevertBuffer),
        CmdBuilder::new("open", Some(("path", CmdTemplateArg::Path)), false).add_alias("o").build(|args| Cmd::OpenFile { path: args[0].take().unwrap().unwrap_path() }),
        CmdBuilder::new("cd", Some(("path", CmdTemplateArg::Path)), false).build(|args| Cmd::Cd { path: args[0].take().unwrap().unwrap_path() }),
        CmdBuilder::new("save", Some(("path", CmdTemplateArg::Path)), true).add_alias("s").build(|args| Cmd::Save {path: args[0].take().map(|arg| arg.unwrap_path()) }),
        CmdBuilder::new("goto", Some(("line", CmdTemplateArg::Int)), false).add_alias("g").build(|args| Cmd::Goto { line: args[0].take().unwrap().unwrap_int() }),
        CmdBuilder::new("theme", Some(("theme", CmdTemplateArg::Theme)), true).build(|args| Cmd::Theme { theme: args[0].take().map(|theme| theme.unwrap_string()) }),
        CmdBuilder::new("new", Some(("path", CmdTemplateArg::Path)), true).add_alias("n").build(|args| Cmd::New { path: args[0].take().map(|arg| arg.unwrap_path()) }),
        CmdBuilder::new("indent", Some(("indent", CmdTemplateArg::String)), true).build(|args| Cmd::Indent { indent: args[0].take().map(|indent| indent.unwrap_string()) }),
        CmdBuilder::new("replace-all", Some(("replace-all", CmdTemplateArg::String)), false).build(|args| Cmd::ReplaceAll{ text: args[0].take().unwrap().unwrap_string() }),
        CmdBuilder::new("pipe", Some(("arg", CmdTemplateArg::Path)), false).build(|args| {
            let mut paths = Vec::new();
            for arg in args {
                paths.push(arg.take().unwrap().unwrap_path());
            }
            Cmd::RunShellCmd { args: paths, pipe: true }
        }),
        CmdBuilder::new("shell", Some(("arg", CmdTemplateArg::Path)), false).add_alias("sh").build(|args| {
            let mut paths = Vec::new();
            for arg in args {
                paths.push(arg.take().unwrap().unwrap_path());
            }
            Cmd::RunShellCmd { args: paths, pipe: false }
        }),
        CmdBuilder::new("sort", Some(("order", CmdTemplateArg::Alternatives(["asc", "desc"].iter().map(|s| s.to_string()).collect()))), true).build(|args| {
            Cmd::SortLines { ascending: args[0].take().map(|o|o.unwrap_string() == "asc").unwrap_or(true)}
        }),
        CmdBuilder::new("split", Some(("direction", CmdTemplateArg::Alternatives(["up", "down", "left", "right"].iter().map(|s| s.to_string()).collect()))), false).build(|args| {
            Cmd::Split { direction: Direction::from_str(args[0].take().unwrap().unwrap_string().as_str()).unwrap()}
        }),
        CmdBuilder::new("case", Some(("case", CmdTemplateArg::Alternatives(["lower", "upper", "snake", "kebab", "camel", "pascal", "title", "train", "screaming-snake", "screaming-kebab"].iter().map(|s| s.to_string()).collect()))), false).build(|args| {
            Cmd::Case { case: Case::from_str(args[0].take().unwrap().unwrap_string().as_str()).unwrap()}
        }),
        CmdBuilder::new("encoding", Some(("encoding", CmdTemplateArg::Alternatives(get_encoding_names().iter().map(|s| s.to_string()).collect()))), true)
            .set_custom_alternative_error(|encoding, _| format!("`{encoding}` is unknown an encoding, these encodings are supported: https://docs.rs/encoding_rs/latest/encoding_rs"))
            .build(|args| {
                Cmd::Encoding { encoding: args[0].take().map(|encoding| encoding.unwrap_string())}
            }),
        CmdBuilder::new("language", Some(("language", CmdTemplateArg::Alternatives(get_available_languages().iter().map(|s| s.to_string()).collect()))), true)
            .add_alias("lang")
            .build(|args| Cmd::Language { language: args[0].take().map(|language| language.unwrap_string())}),
        CmdBuilder::new("line-ending", Some(("line-ending", CmdTemplateArg::Alternatives(vec!["lf".into(), "crlf".into()]))), true)
            .build(|args| {
                Cmd::LineEnding{ line_ending: args[0].take().map(|line_ending| {
                    match line_ending.unwrap_string().as_str() {
                        "lf" => LineEnding::LF,
                        "crlf" => LineEnding::Crlf,
                        _ => unreachable!(),
                    }
                })}
        }),
    ];
    cmds.sort_by(|cmd1, cmd2| cmd1.name.cmp(&cmd2.name));
    cmds
});
