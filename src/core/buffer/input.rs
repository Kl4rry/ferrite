use super::{error::BufferError, Buffer};
use crate::tui_app::input::InputCommand;

impl Buffer {
    pub fn handle_input(&mut self, input: InputCommand) -> Result<(), BufferError> {
        match input {
            InputCommand::MoveRight { shift } => self.move_right_char(shift),
            InputCommand::MoveLeft { shift } => self.move_left_char(shift),
            InputCommand::MoveUp { shift } => self.move_up(shift),
            InputCommand::MoveDown { shift } => self.move_down(shift),
            InputCommand::MoveLine(dir) => self.move_line(dir),
            InputCommand::Insert(text) => self.insert_text(&text),
            InputCommand::Char(ch) => self.insert_text(&String::from(ch)),
            InputCommand::Backspace => self.backspace(),
            InputCommand::Delete => self.delete(),
            InputCommand::Home { shift } => self.home(shift),
            InputCommand::End { shift } => self.end(shift),
            InputCommand::Eof { shift } => self.eof(shift),
            InputCommand::Start { shift } => self.start(shift),
            InputCommand::SelectAll => self.select_all(),
            InputCommand::SelectLine => self.select_line(),
            InputCommand::Copy => self.copy(),
            InputCommand::Tab { back } => self.tab(back),
            InputCommand::Scroll(distance) => self.scroll(distance),
            InputCommand::Save => self.save(None)?,
            InputCommand::Quit => (),
            InputCommand::FocusPalette => (),
            InputCommand::Escape => (),
        }
        Ok(())
    }
}
