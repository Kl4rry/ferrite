use std::cmp;

use ropey::RopeSlice;
use utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};

use super::buffer::{error::BufferError, Buffer};
use crate::tui_app::input::InputCommand;

pub mod fuzzy_file_find;

#[derive(Debug)]
pub struct SearchBuffer<T> {
    search_field: Buffer,
    selected: usize,
    result_provider: T,
    choice: Option<String>,
}

impl<T> SearchBuffer<T>
where
    T: ResultProvider,
{
    pub fn new(result_provider: T) -> Self {
        let mut search_field = Buffer::new();
        search_field.set_view_lines(1);
        search_field.clamp_cursor = false;
        Self {
            search_field,
            result_provider,
            selected: 0,
            choice: None,
        }
    }

    pub fn search_field(&self) -> &Buffer {
        &self.search_field
    }

    pub fn provider(&mut self) -> &mut T {
        &mut self.result_provider
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn get_choice(&mut self) -> Option<String> {
        self.choice.take()
    }

    pub fn handle_input(&mut self, input: InputCommand) -> Result<(), BufferError> {
        let mut enter = false;
        match input {
            InputCommand::MoveUp { .. } => self.selected = self.selected.saturating_sub(1),
            InputCommand::MoveDown { .. } => self.selected += 1,
            InputCommand::Insert(string) => {
                let rope = RopeSlice::from(string.as_str());
                let line = rope.line_without_line_ending(0);
                self.search_field
                    .handle_input(InputCommand::Insert(line.to_string()))?;
                if line.len_bytes() != rope.len_bytes() {
                    enter = true;
                } else {
                    self.selected = 0;
                    self.result_provider.search(self.search_field.to_string());
                }
            }
            InputCommand::Char(ch) if LineEnding::from_char(ch).is_some() => {
                enter = true;
            }
            input => {
                self.search_field.handle_input(input)?;
                self.result_provider.search(self.search_field.to_string());
            }
        }

        self.selected = cmp::min(
            self.selected,
            self.provider().poll_result().len().saturating_sub(1),
        );

        if enter {
            let selected = self.selected;
            self.choice = self
                .provider()
                .poll_result()
                .get(selected)
                .map(|s| s.to_string())
        }
        Ok(())
    }
}

pub trait ResultProvider {
    fn poll_result(&mut self) -> &[String];
    fn search(&mut self, term: String);
}
