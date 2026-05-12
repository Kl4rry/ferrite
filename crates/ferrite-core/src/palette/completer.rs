use std::{borrow::Cow, cmp::Ordering, path::PathBuf};

use ferrite_ctx::ArenaVec;
use ferrite_utility::line_ending::LineEnding;

use self::path_completer::complete_file_path;
use super::cmd_parser::{
    generic_cmd::CmdTemplateArg,
    get_command_input_type,
    lexer::{self, Token},
};
use crate::buffer::Buffer;

mod path_completer;

pub struct Completer {
    options: Vec<Box<dyn CompletionOption>>,
    index: Option<usize>,
    ctx: CompleterContext,
}

impl Completer {
    pub fn new(buffer: &Buffer, ctx: CompleterContext) -> Self {
        let mut new = Self {
            options: Vec::new(),
            index: None,
            ctx,
        };

        new.update_text(buffer);
        new
    }

    pub fn forward(&mut self, buffer: &mut Buffer) {
        if self.options.is_empty() {
            return;
        }
        match &mut self.index {
            Some(index) => {
                *index += 1;
                if *index >= self.options.len() {
                    *index = 0;
                }
            }
            None => {
                self.index = Some(0);
            }
        };
        self.do_completion(buffer);
    }

    pub fn backward(&mut self, buffer: &mut Buffer) {
        if self.options.is_empty() {
            return;
        }
        match &mut self.index {
            Some(index) => {
                *index = index.saturating_sub(1);
            }
            None => {
                self.index = Some(0);
            }
        };
        self.do_completion(buffer);
    }

    fn do_completion(&self, buffer: &mut Buffer) {
        let view_id = buffer.get_first_view_or_create();
        buffer.trim_start(view_id);

        let option = &*self.options[self.index.unwrap()];
        let text = buffer.to_string();

        let (cmd, tokens) = lexer::tokenize(&text);

        let mut replacement = String::new();
        let mut quote = false;
        for ch in option.replacement().chars() {
            if LineEnding::from_char(ch).is_some() {
                replacement.push_str("\\n");
                quote = true;
            } else if ch == '"' {
                replacement.push_str("\\\"");
            } else {
                if ch == '\'' || ch.is_whitespace() {
                    quote = true;
                }
                replacement.push(ch);
            }
        }

        if quote {
            replacement.insert(0, '"');
            replacement.push('"');
        }

        let view_id = buffer.get_first_view_or_create();
        match get_completion_type(&text, &tokens) {
            CompletionType::NewCmd | CompletionType::NewArg => {
                buffer.insert_text(view_id, &replacement, false);
            }
            CompletionType::Cmd => {
                buffer.replace(view_id, cmd.start..(cmd.start + cmd.len), &replacement);
            }
            CompletionType::Arg => {
                let last = tokens.last().unwrap();
                buffer.replace(view_id, last.start..(last.start + last.len), &replacement);
            }
        }
        buffer.eof(view_id, false);

        buffer.mark_clean();
    }

    pub fn options(&self) -> &[Box<dyn CompletionOption>] {
        &self.options
    }

    pub fn current(&self) -> Option<usize> {
        self.index
    }

    pub fn update_text(&mut self, buffer: &Buffer) {
        self.index = None;
        self.options.clear();
        let text = buffer.to_string();
        if text.is_empty() && !self.ctx.external {
            self.options.extend(
                super::cmd_parser::get_command_names()
                    .iter()
                    .map(|s| Box::new(s.to_string()) as Box<dyn CompletionOption>),
            );
            if self.options.is_empty() {
                self.index = None;
            }
            return;
        }

        let (cmd, tokens) = lexer::tokenize(&text);

        match get_completion_type(&text, &tokens) {
            CompletionType::Cmd | CompletionType::NewCmd => {
                if self.ctx.external && text.contains(std::path::MAIN_SEPARATOR) {
                    self.options.extend(
                        complete_file_path(&cmd.text, true)
                            .into_iter()
                            .map(|path| Box::new(path) as Box<dyn CompletionOption>),
                    );
                    return;
                }

                // TODO: rm tmp alloc
                let cmds: Vec<_> = if self.ctx.external && !cmd.text.is_empty() {
                    executable_finder::unique_executables()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|exe| exe.name.into())
                        .collect()
                } else if !cmd.text.is_empty() {
                    super::cmd_parser::get_command_names()
                        .into_iter()
                        .map(Cow::Borrowed)
                        .collect()
                } else {
                    Vec::new()
                };

                self.options.extend(
                    fuzzy_match(&cmd.text, cmds.into_iter())
                        .into_iter()
                        .map(|s| Box::new(s.to_string()) as Box<dyn CompletionOption>),
                );

                if self.options.is_empty() {
                    self.index = None;
                }
            }
            CompletionType::Arg | CompletionType::NewArg => {
                let mut input_type = self.ctx.force_arg_type.as_ref();
                if input_type.is_none() {
                    input_type = get_command_input_type(&cmd.text);
                }
                if let Some(input_type) = input_type {
                    let text = match tokens.last() {
                        Some(token) => &token.text,
                        None => "",
                    };

                    match input_type {
                        CmdTemplateArg::Path => {
                            self.options.extend(
                                complete_file_path(text, false)
                                    .into_iter()
                                    .map(|path| Box::new(path) as Box<dyn CompletionOption>),
                            );

                            if self.options.is_empty() {
                                self.index = None;
                            }
                        }
                        CmdTemplateArg::Alternatives(alternatives) => {
                            self.options.extend(
                                fuzzy_match(text, alternatives.iter())
                                    .into_iter()
                                    .map(|s| Box::new(s.to_string()) as Box<dyn CompletionOption>),
                            );

                            if self.options.is_empty() {
                                self.index = None;
                            }
                        }
                        CmdTemplateArg::Theme => {
                            self.options.extend(
                                fuzzy_match(text, self.ctx.themes.iter())
                                    .into_iter()
                                    .map(|s| Box::new(s.to_string()) as Box<dyn CompletionOption>),
                            );

                            if self.options.is_empty() {
                                self.index = None;
                            }
                        }
                        CmdTemplateArg::Action => {
                            self.options.extend(
                                fuzzy_match(text, self.ctx.actions.iter())
                                    .into_iter()
                                    .map(|s| Box::new(s.to_string()) as Box<dyn CompletionOption>),
                            );

                            if self.options.is_empty() {
                                self.index = None;
                            }
                        }
                        _ => (),
                    }
                }

                if self.options.is_empty() {
                    self.index = None;
                }
            }
        }
    }
}

pub struct CompleterContext {
    themes: Vec<String>,
    actions: Vec<String>,
    external: bool,
    force_arg_type: Option<CmdTemplateArg>,
}

impl CompleterContext {
    pub fn new(
        themes: Vec<String>,
        actions: Vec<String>,
        external: bool,
        force_arg_type: Option<CmdTemplateArg>,
    ) -> Self {
        Self {
            themes,
            actions,
            external,
            force_arg_type,
        }
    }
}

pub trait CompletionOption {
    fn display(&self) -> Cow<str>;
    fn replacement(&self) -> Cow<str>;
}

impl CompletionOption for String {
    fn display(&self) -> Cow<str> {
        self.into()
    }

    fn replacement(&self) -> Cow<str> {
        self.into()
    }
}

impl CompletionOption for PathBuf {
    fn display(&self) -> Cow<str> {
        self.file_name().unwrap().to_string_lossy()
    }

    fn replacement(&self) -> Cow<str> {
        self.to_string_lossy()
    }
}

#[derive(Debug, Clone, Copy)]
enum CompletionType {
    NewCmd,
    Cmd,
    NewArg,
    Arg,
}

fn get_completion_type(text: &str, tokens: &[Token]) -> CompletionType {
    let text = text.trim_start();
    if text.is_empty() {
        return CompletionType::NewCmd;
    }

    let ends_with_space = text.chars().last().unwrap().is_whitespace();

    if tokens.is_empty() && ends_with_space {
        return CompletionType::NewArg;
    }

    if tokens.is_empty() && !ends_with_space {
        return CompletionType::Cmd;
    }

    if ends_with_space {
        CompletionType::NewArg
    } else {
        CompletionType::Arg
    }
}

fn fuzzy_match<S: AsRef<str> + Clone + std::fmt::Debug + std::cmp::Ord>(
    needle: &str,
    haystack: impl Iterator<Item = S>,
) -> Vec<S> {
    let arena = ferrite_ctx::Ctx::arena();
    let mut tmp = ArenaVec::new_in(&arena);
    tmp.extend(haystack);

    if needle.is_empty() {
        return tmp.to_vec();
    }

    let mut matches = frizbee::match_list(needle, &tmp, &Default::default());

    matches.sort_by(|a, b| match b.score.cmp(&a.score) {
        Ordering::Equal => {
            if tmp[a.index as usize].as_ref().starts_with(needle) {
                Ordering::Less
            } else if tmp[b.index as usize].as_ref().starts_with(needle) {
                Ordering::Greater
            } else {
                tmp[a.index as usize]
                    .as_ref()
                    .len()
                    .cmp(&tmp[b.index as usize].as_ref().len())
            }
        }
        Ordering::Greater => Ordering::Greater,
        Ordering::Less => Ordering::Less,
    });

    let mut output = Vec::new();

    for m in matches {
        output.push(tmp[m.index as usize].clone());
    }

    output
}
