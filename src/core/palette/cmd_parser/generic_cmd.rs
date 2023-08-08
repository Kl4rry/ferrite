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
    pub aliases: Vec<String>,
    pub args: Option<(String, CommandTemplateArg)>,
    pub optional: bool,
}

impl CommandTemplate {
    pub fn new(
        name: impl Into<String>,
        args: Option<(&str, CommandTemplateArg)>,
        optional: bool,
    ) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            args: args.map(|(name, template)| (name.to_string(), template)),
            optional,
        }
    }

    pub fn add_alias(mut self, arg: impl ToString) -> Self {
        self.aliases.push(arg.to_string());
        self
    }

    pub fn _add_aliases(mut self, args: &[impl ToString]) -> Self {
        self.aliases.extend(args.iter().map(|a| a.to_string()));
        self
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

        for ((_, arg), token) in self.args.iter().zip(tokens) {
            let arg = arg.parse_arg(token)?;
            generic.args.push(Some(arg));
        }

        if self.optional && generic.args.is_empty() {
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
