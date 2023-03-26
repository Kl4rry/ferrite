use ropey::RopeSlice;
use utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};

use super::buffer::{error::BufferError, Buffer};
use crate::tui_app::{
    event_loop::{TuiAppEvent, TuiEventLoopProxy},
    input::InputCommand,
};

pub mod cmd;
pub mod cmd_parser;

pub enum PaletteState {
    Input {
        buffer: Buffer,
        prompt: String,
        mode: String,
    },
    Message(String),
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

    pub fn reset(&mut self) {
        self.state = PaletteState::Nothing;
    }

    pub fn focus(&mut self, prompt: impl Into<String>, mode: impl Into<String>) {
        let mut buffer = Buffer::new();
        buffer.set_view_lines(1);
        buffer.clamp_cursor = false;
        self.state = PaletteState::Input {
            buffer,
            prompt: prompt.into(),
            mode: mode.into(),
        };
    }

    pub fn has_focus(&self) -> bool {
        matches!(self.state, PaletteState::Input { .. })
    }

    pub fn state(&self) -> &PaletteState {
        &self.state
    }
}

impl CommandPalette {
    pub fn handle_input(&mut self, input: InputCommand) -> Result<(), BufferError> {
        if let PaletteState::Input { buffer, mode, .. } = &mut self.state {
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
        Ok(())
    }
}
