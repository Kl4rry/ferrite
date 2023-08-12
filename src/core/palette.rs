use std::fmt;

use ropey::RopeSlice;
use utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};

use self::completer::{Completer, CompleterContext};
use super::buffer::{error::BufferError, Buffer};
use crate::tui_app::{
    event_loop::{TuiAppEvent, TuiEventLoopProxy},
    input::InputCommand,
};

pub mod cmd;
pub mod cmd_parser;
pub mod completer;

#[derive(Debug, Clone)]
pub enum PalettePromptEvent {
    Nop,
    Quit,
    Reload,
    CloseCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedPrompt {
    Alt1,
    Alt2,
    Neither,
}

pub enum PaletteState {
    Input {
        buffer: Buffer,
        prompt: String,
        mode: String,
        focused: bool,
        completer: Completer,
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

    pub fn focus(
        &mut self,
        prompt: impl Into<String>,
        mode: impl Into<String>,
        ctx: CompleterContext,
    ) {
        let mut buffer = Buffer::new();
        buffer.set_view_lines(1);
        let mode = mode.into();
        if let PaletteState::Input {
            mode: input_mode,
            focused,
            ..
        } = &mut self.state
        {
            if input_mode == &mode {
                *focused = true;
                return;
            }
        }
        self.state = PaletteState::Input {
            prompt: prompt.into(),
            mode,
            focused: true,
            completer: Completer::new(&buffer, ctx),
            buffer,
        };
    }

    pub fn set_prompt(
        &mut self,
        prompt: impl Into<String>,
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
            prompt: prompt.into(),
            alt1_char: alt1_char.to_ascii_lowercase(),
            alt1_event,
            alt2_char: alt2_char.to_ascii_lowercase(),
            alt2_event,
        };
    }

    pub fn has_focus(&self) -> bool {
        matches!(
            self.state,
            PaletteState::Input { focused: true, .. } | PaletteState::Prompt { .. }
        )
    }

    pub fn unfocus(&mut self) {
        if let PaletteState::Input { focused, .. } = &mut self.state {
            *focused = false;
        }
    }

    pub fn state(&mut self) -> &mut PaletteState {
        &mut self.state
    }
}

impl CommandPalette {
    pub fn handle_input(
        &mut self,
        input: InputCommand,
        ctx: CompleterContext,
    ) -> Result<(), BufferError> {
        match &mut self.state {
            PaletteState::Input {
                buffer,
                mode,
                completer,
                ..
            } => {
                let mut enter = false;
                buffer.mark_clean();
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
                    InputCommand::Tab { back } if mode == "command" => {
                        if back {
                            completer.backward(buffer)
                        } else {
                            completer.forward(buffer)
                        }
                        if completer.options().len() == 1 {
                            buffer.mark_dirty();
                        }
                    }
                    InputCommand::MoveRight { .. } => {
                        buffer.handle_input(input)?;
                        if buffer.cursor_is_eof() {
                            buffer.mark_dirty();
                        }
                    }
                    input => {
                        buffer.handle_input(input)?;
                    }
                }

                if enter && buffer.rope().len_bytes() > 0 {
                    self.proxy.send(TuiAppEvent::PaletteEvent {
                        mode: mode.clone(),
                        content: buffer.rope().to_string(),
                    });
                } else if buffer.is_dirty() && mode == "command" {
                    completer.update_text(buffer, ctx);
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
