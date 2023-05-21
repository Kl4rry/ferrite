use std::path::PathBuf;

use super::error::CommandParseError;

#[derive(Debug, Clone)]
pub enum CommandTemplateArg {
    Alternatives(Vec<String>),
    Int,
    String,
    Path,
}

impl CommandTemplateArg {
    fn parse_arg(&self, token: String) -> Result<CommandArg, CommandParseError> {
        match self {
            CommandTemplateArg::Alternatives(alternatives) => {
                if alternatives.contains(&token) {
                    Ok(CommandArg::String(token))
                } else {
                    Err(CommandParseError::UnknownArg(token))
                }
            }
            CommandTemplateArg::Int => Ok(CommandArg::Int(token.parse()?)),
            CommandTemplateArg::String => Ok(CommandArg::String(token)),
            CommandTemplateArg::Path => Ok(CommandArg::Path(token.into())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandTemplate {
    pub name: String,
    pub args: Vec<(String, CommandTemplateArg)>,
    pub optional: usize,
}

impl CommandTemplate {
    pub fn new(
        name: impl Into<String>,
        args: Vec<(&str, CommandTemplateArg)>,
        optional: usize,
    ) -> Self {
        Self {
            name: name.into(),
            args: args
                .into_iter()
                .map(|(name, arg)| (name.to_string(), arg))
                .collect(),
            optional,
        }
    }

    pub fn parse_cmd(
        &self,
        tokens: impl ExactSizeIterator<Item = String>,
    ) -> Result<GenericCommand, CommandParseError> {
        if tokens.len() < self.args.len().saturating_sub(self.optional) {
            return Err(CommandParseError::MissingArgs(self.usage()));
        }

        let mut generic = GenericCommand {
            name: self.name.clone(),
            args: Vec::new(),
        };

        for ((_, arg), token) in self.args.iter().zip(tokens) {
            let arg = arg.parse_arg(token)?;
            generic.args.push(Some(arg));
        }

        while generic.args.len() < self.args.len() {
            generic.args.push(None);
        }

        Ok(generic)
    }

    pub fn usage(&self) -> String {
        let mut usage = self.name.clone();
        for (arg, _) in &self.args {
            usage.push(' ');
            usage.push_str(arg);
        }

        usage
    }
}

#[derive(Debug)]
pub enum CommandArg {
    Int(i64),
    String(String),
    Path(PathBuf),
}

impl CommandArg {
    #[allow(dead_code)]
    pub fn unwrap_int(self) -> i64 {
        match self {
            Self::Int(val) => val,
            _ => panic!("called `CommandArg::unwrap_int()` on a `{:?}`", self),
        }
    }

    #[allow(dead_code)]
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
