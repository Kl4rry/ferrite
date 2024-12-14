use std::path::PathBuf;

use super::error::CommandParseError;
use crate::cmd::Cmd;

#[derive(Debug, Clone)]
pub enum CmdTemplateArg {
    Alternatives(Vec<String>),
    Int,
    String,
    Path,
    Theme,
    Action,
}

impl CmdTemplateArg {
    fn parse_arg(&self, token: String) -> Result<CommandArg, CommandParseError> {
        match self {
            CmdTemplateArg::Alternatives(alternatives) => {
                if alternatives.contains(&token) {
                    Ok(CommandArg::String(token))
                } else {
                    Err(CommandParseError::UnknownArg(token))
                }
            }
            CmdTemplateArg::Int => Ok(CommandArg::Int(token.parse()?)),
            CmdTemplateArg::String => Ok(CommandArg::String(token)),
            CmdTemplateArg::Theme => Ok(CommandArg::String(token)),
            CmdTemplateArg::Action => Ok(CommandArg::String(token)),
            CmdTemplateArg::Path => {
                let home_dir = if let Some(directories) = directories::UserDirs::new() {
                    directories.home_dir().into()
                } else {
                    PathBuf::new()
                };

                let mut token = token;
                if token.starts_with("~") {
                    token.replace_range(..1, &home_dir.to_string_lossy());
                }
                Ok(CommandArg::Path(token.into()))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CmdBuilder {
    pub name: String,
    pub aliases: Vec<String>,
    pub args: Option<(String, CmdTemplateArg)>,
    pub optional: bool,
    custom_alternative_error: Option<fn(&str, &[String]) -> String>,
}

impl CmdBuilder {
    pub fn new(
        name: impl Into<String>,
        args: Option<(&str, CmdTemplateArg)>,
        optional: bool,
    ) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            args: args.map(|(name, template)| (name.to_string(), template)),
            optional,
            custom_alternative_error: None,
        }
    }

    pub fn add_alias(mut self, arg: impl ToString) -> Self {
        self.aliases.push(arg.to_string());
        self
    }

    pub fn set_custom_alternative_error(mut self, f: fn(&str, &[String]) -> String) -> Self {
        self.custom_alternative_error = Some(f);
        self
    }

    pub fn build(self, map: fn(&mut [Option<CommandArg>]) -> Cmd) -> CommandTemplate {
        CommandTemplate {
            name: self.name,
            aliases: self.aliases,
            args: self.args,
            optional: self.optional,
            custom_alternative_error: self.custom_alternative_error,
            map,
        }
    }
}

pub struct CommandTemplate {
    pub name: String,
    pub aliases: Vec<String>,
    pub args: Option<(String, CmdTemplateArg)>,
    pub optional: bool,
    custom_alternative_error: Option<fn(&str, &[String]) -> String>,
    map: fn(&mut [Option<CommandArg>]) -> Cmd,
}

impl CommandTemplate {
    pub fn parse_cmd(
        &self,
        tokens: impl ExactSizeIterator<Item = String>,
    ) -> Result<GenericCommand, CommandParseError> {
        if !self.optional && tokens.len() == 0 {
            return Err(CommandParseError::MissingArgs(self.usage()));
        }

        let mut generic = GenericCommand {
            name: self.name.clone(),
            args: Vec::new(),
        };

        if let Some((_, template)) = &self.args {
            for token in tokens {
                let arg = match template.parse_arg(token) {
                    Ok(arg) => arg,
                    Err(CommandParseError::UnknownArg(arg))
                        if self.custom_alternative_error.is_some()
                            && matches!(template, CmdTemplateArg::Alternatives(_)) =>
                    {
                        match template {
                            CmdTemplateArg::Alternatives(alts) => {
                                let error_creator = self.custom_alternative_error.as_ref().unwrap();
                                return Err(CommandParseError::Custom(error_creator(
                                    arg.as_str(),
                                    alts,
                                )));
                            }
                            _ => unreachable!(),
                        }
                    }
                    Err(err) => return Err(err),
                };
                generic.args.push(Some(arg));
            }
        }

        if self.optional && generic.args.is_empty() {
            generic.args.push(None);
        }

        Ok(generic)
    }

    pub fn matches(&self, query: impl AsRef<str>) -> bool {
        let query = query.as_ref();
        if self.name == query {
            return true;
        }

        for alias in &self.aliases {
            if alias == query {
                return true;
            }
        }

        false
    }

    pub fn usage(&self) -> String {
        let mut usage = self.name.clone();
        if let Some((arg, _)) = &self.args {
            usage.push(' ');
            usage.push_str(arg);
        }

        usage
    }

    pub fn to_cmd(&self, args: &mut [Option<CommandArg>]) -> Cmd {
        (self.map)(args)
    }
}

#[derive(Debug)]
pub enum CommandArg {
    Int(i64),
    String(String),
    Path(PathBuf),
}

impl CommandArg {
    pub fn unwrap_int(self) -> i64 {
        match self {
            Self::Int(val) => val,
            _ => panic!("called `CommandArg::unwrap_int()` on a `{:?}`", self),
        }
    }

    pub fn unwrap_string(self) -> String {
        match self {
            Self::String(val) => val,
            _ => panic!("called `CommandArg::unwrap_string()` on a `{:?}`", self),
        }
    }

    pub fn unwrap_path(self) -> PathBuf {
        match self {
            Self::Path(val) => val,
            _ => panic!("called `CommandArg::unwrap_path()` on a `{:?}`", self),
        }
    }
}

#[derive(Debug)]
pub struct GenericCommand {
    pub name: String,
    pub args: Vec<Option<CommandArg>>,
}
