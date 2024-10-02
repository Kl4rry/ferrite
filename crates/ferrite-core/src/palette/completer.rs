use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use ferrite_utility::line_ending::LineEnding;
use sublime_fuzzy::{FuzzySearch, Scoring};

use self::path_completer::complete_file_path;
use super::cmd_parser::{
    generic_cmd::CmdTemplateArg,
    get_command_input_type,
    lexer::{self, Token},
};
use crate::{buffer::Buffer, theme::EditorTheme};

mod path_completer;

pub struct Completer {
    options: Vec<Box<dyn CompletionOption>>,
    index: Option<usize>,
}

impl Completer {
    pub fn new(buffer: &Buffer, ctx: CompleterContext) -> Self {
        let mut new = Self {
            options: Vec::new(),
            index: None,
        };

        new.update_text(buffer, ctx);
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

    pub fn update_text(&mut self, buffer: &Buffer, ctx: CompleterContext) {
        self.index = None;
        let text = buffer.to_string();
        if text.is_empty() {
            self.options.clear();
            self.options.extend(
                super::cmd_parser::get_command_names()
                    .iter()
                    .map(|s| Box::new(s.to_string()) as Box<dyn CompletionOption>),
            );
            return;
        }

        let (cmd, tokens) = lexer::tokenize(&text);

        match get_completion_type(&text, &tokens) {
            CompletionType::Cmd | CompletionType::NewCmd => {
                self.options.clear();
                self.options.extend(
                    super::cmd_parser::get_command_names()
                        .iter()
                        // TODO make this fuzzy
                        .filter(|s| s.starts_with(&cmd.text))
                        .map(|s| Box::new(s.to_string()) as Box<dyn CompletionOption>),
                );

                if self.options.is_empty() {
                    self.index = None;
                }
            }
            CompletionType::Arg | CompletionType::NewArg => {
                self.options.clear();
                if let Some(input_type) = get_command_input_type(&cmd.text) {
                    let text = match tokens.last() {
                        Some(token) => &token.text,
                        None => "",
                    };

                    match input_type {
                        CmdTemplateArg::Path => {
                            self.options.extend(
                                complete_file_path(text)
                                    .into_iter()
                                    .map(|path| Box::new(path) as Box<dyn CompletionOption>),
                            );
                        }
                        CmdTemplateArg::Alternatives(alternatives) => {
                            let mut alternatives = alternatives
                                .iter()
                                .filter_map(|alternative| {
                                    if text.is_empty() {
                                        return Some((0, alternative));
                                    }
                                    FuzzySearch::new(text, alternative)
                                        .score_with(&Scoring::emphasize_distance())
                                        .best_match()
                                        .map(|m| (m.score(), alternative))
                                })
                                .collect::<Vec<_>>();
                            alternatives.sort_by(|a, b| b.0.cmp(&a.0));

                            self.options.extend(alternatives.into_iter().map(|(_, s)| {
                                Box::new(s.to_string()) as Box<dyn CompletionOption>
                            }));
                        }
                        CmdTemplateArg::Theme => {
                            let mut themes = ctx
                                .themes
                                .keys()
                                .filter_map(|alternative| {
                                    if text.is_empty() {
                                        return Some((0, alternative));
                                    }
                                    FuzzySearch::new(text, alternative)
                                        .score_with(&Scoring::emphasize_distance())
                                        .best_match()
                                        .map(|m| (m.score(), alternative))
                                })
                                .collect::<Vec<_>>();
                            themes.sort_by(|a, b| b.0.cmp(&a.0));

                            self.options.extend(themes.into_iter().map(|(_, s)| {
                                Box::new(s.to_string()) as Box<dyn CompletionOption>
                            }));
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

pub struct CompleterContext<'a> {
    themes: &'a HashMap<String, EditorTheme>,
}

impl<'a> CompleterContext<'a> {
    pub fn new(themes: &'a HashMap<String, EditorTheme>) -> Self {
        Self { themes }
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
