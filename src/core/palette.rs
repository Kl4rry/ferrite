use std::fmt;

use ropey::RopeSlice;
use utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};

use super::buffer::{error::BufferError, Buffer};
use crate::tui_app::{
    event_loop::{TuiAppEvent, TuiEventLoopProxy},
    input::InputCommand,
};

pub mod cmd;
pub mod cmd_parser;

#[derive(Debug, Clone)]
pub enum PalettePromptEvent {
    Nop,
    Quit,
    Reload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedPrompt {
    Alt1,
    Alt2,
    Neither,
}

#[derive(Debug)]
pub enum PaletteState {
    Input {
        buffer: Buffer,
        prompt: String,
        mode: String,
    },
    Prompt {
        selected: SelectedPrompt,
        prompt: String,
        alt1_char: char,
        alt1_event: PalettePromptEvent,
        alt2_char: char,
        alt2_event: PalettePromptEvent,
    },
    Message(String),
    Error(String),
    Nothing,
}

pub struct CommandPalette {
    proxy: TuiEventLoopProxy,
    state: PaletteState,
}

impl CommandPalette {
    pub fn new(proxy: TuiEventLoopProxy) -> Self {
        Self {
            state: PaletteState::Nothing,
            proxy,
        }
    }

    pub fn set_msg(&mut self, msg: impl Into<String>) {
        self.state = PaletteState::Message(msg.into());
    }

    pub fn set_error(&mut self, msg: impl fmt::Display) {
        self.state = PaletteState::Error(msg.to_string());
    }

    pub fn reset(&mut self) {
        self.state = PaletteState::Nothing;
    }

    pub fn focus(&mut self, prompt: impl Into<String>, mode: impl Into<String>) {
        let mut buffer = Buffer::new();
        buffer.set_view_lines(1);
        self.state = PaletteState::Input {
            buffer,
            prompt: prompt.into(),
            mode: mode.into(),
        };
    }

    pub fn set_prompt(
        &mut self,
        prompt: String,
        (alt1_char, alt1_event): (char, PalettePromptEvent),
        (alt2_char, alt2_event): (char, PalettePromptEvent),
    ) {
        assert!(
            alt1_char.is_ascii_alphabetic()
                && alt2_char.is_ascii_alphabetic()
                && alt1_char != alt2_char
        );
        self.state = PaletteState::Prompt {
            selected: SelectedPrompt::Neither,
            prompt,
            alt1_char: alt1_char.to_ascii_lowercase(),
            alt1_event,
            alt2_char: alt2_char.to_ascii_lowercase(),
            alt2_event,
        };
    }

    pub fn has_focus(&self) -> bool {
        matches!(
            self.state,
            PaletteState::Input { .. } | PaletteState::Prompt { .. }
        )
    }

    pub fn state(&mut self) -> &mut PaletteState {
        &mut self.state
    }
}

impl CommandPalette {
    pub fn handle_input(&mut self, input: InputCommand) -> Result<(), BufferError> {
        match &mut self.state {
            PaletteState::Input { buffer, mode, .. } => {
                let mut enter = false;
                match input {
                    InputCommand::Insert(string) => {
                        let rope = RopeSlice::from(string.as_str());
                        let line = rope.line_without_line_ending(0);
                        buffer.handle_input(InputCommand::Insert(line.to_string()))?;
                        if line.len_bytes() != rope.len_bytes() {
                            enter = true;
                        }
                    }
                    InputCommand::Char(ch) if LineEnding::from_char(ch).is_some() => {
                        enter = true;
                    }
                    input => buffer.handle_input(input)?,
                }

                if enter && buffer.rope().len_bytes() > 0 {
                    self.proxy.send(TuiAppEvent::PaletteEvent {
                        mode: mode.clone(),
                        content: buffer.rope().to_string(),
                    });
                }
            }
            PaletteState::Prompt {
                selected,
                alt1_char,
                alt1_event,
                alt2_char,
                alt2_event,
                ..
            } => {
                let mut chars = Vec::new();
                match input {
                    InputCommand::Char(ch) => chars.push(ch),
                    InputCommand::Insert(string) => chars.extend(string.chars()),
                    _ => (),
                }
                for ch in chars {
                    if ch == *alt1_char {
                        *selected = SelectedPrompt::Alt1;
                    }

                    if ch == *alt2_char {
                        *selected = SelectedPrompt::Alt2;
                    }

                    if LineEnding::from_char(ch).is_some() {
                        match selected {
                            SelectedPrompt::Alt1 => {
                                self.proxy
                                    .send(TuiAppEvent::PromptEvent(alt1_event.clone()));
                                self.reset();
                                break;
                            }
                            SelectedPrompt::Alt2 => {
                                self.proxy
                                    .send(TuiAppEvent::PromptEvent(alt2_event.clone()));
                                self.reset();
                                break;
                            }
                            SelectedPrompt::Neither => (),
                        }
                    }
                }
            }
            _ => (),
        }

        Ok(())
    }
}
