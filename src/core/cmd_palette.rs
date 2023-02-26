use std::sync::mpsc::{self, Receiver, Sender};

use ropey::RopeSlice;
use utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};

use super::buffer::{error::BufferError, Buffer};
use crate::tui_app::input::InputCommand;

pub mod cmd;
pub mod cmd_parser;

pub enum PaletteState {
    Input {
        buffer: Buffer,
        prompt: String,
        sender: Sender<String>,
    },
    Message(String),
    Nothing,
}

pub struct CommandPalette {
    state: PaletteState,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            state: PaletteState::Nothing,
        }
    }

    pub fn set_msg(&mut self, msg: impl Into<String>) {
        self.state = PaletteState::Message(msg.into());
    }

    pub fn reset(&mut self) {
        self.state = PaletteState::Nothing;
    }

    pub fn focus(&mut self, prompt: &str) -> Receiver<String> {
        let (sender, receiver) = mpsc::channel();
        self.state = PaletteState::Input {
            buffer: Buffer::new(),
            prompt: prompt.to_string(),
            sender,
        };
        receiver
    }

    pub fn state(&self) -> &PaletteState {
        &self.state
    }
}

impl CommandPalette {
    pub fn handle_input(&mut self, input: InputCommand) -> Result<(), BufferError> {
        if let PaletteState::Input { buffer, sender, .. } = &mut self.state {
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
                let _ = sender.send(buffer.rope().to_string());
            }
        }
        Ok(())
    }
}
