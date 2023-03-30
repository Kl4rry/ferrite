use super::{error::BufferError, Buffer};
use crate::tui_app::input::InputCommand;

impl Buffer {
    pub fn handle_input(&mut self, input: InputCommand) -> Result<(), BufferError> {
        match input {
            InputCommand::MoveRight { shift } => self.move_right_char(shift),
            InputCommand::MoveLeft { shift } => self.move_left_char(shift),
            InputCommand::MoveUp { shift } => self.move_up(shift),
            InputCommand::MoveDown { shift } => self.move_down(shift),
            InputCommand::MoveRightWord { shift } => self.move_right_word(shift),
            InputCommand::MoveLeftWord { shift } => self.move_left_word(shift),
            InputCommand::MoveLine(dir) => self.move_line(dir),
            InputCommand::Insert(text) => self.insert_text(&text),
            InputCommand::Char(ch) => self.insert_text(&String::from(ch)),
            InputCommand::Backspace => self.backspace(),
            InputCommand::BackspaceWord => self.backspace_word(),
            InputCommand::Delete => self.delete(),
            InputCommand::DeleteWord => self.delete_word(),
            InputCommand::Home { shift } => self.home(shift),
            InputCommand::End { shift } => self.end(shift),
            InputCommand::Eof { shift } => self.eof(shift),
            InputCommand::Start { shift } => self.start(shift),
            InputCommand::SelectAll => self.select_all(),
            InputCommand::SelectLine => self.select_line(),
            InputCommand::SelectWord => self.select_word(),
            InputCommand::Copy => self.copy(),
            InputCommand::Cut => self.cut(),
            InputCommand::Paste => self.paste(),
            InputCommand::Tab { back } if !back => self.tab(),
            InputCommand::Tab { back } if back => self.back_tab(),
            InputCommand::Scroll(distance) => self.scroll(distance),
            InputCommand::Escape => self.escape(),
            InputCommand::Save => self.save(None)?,
            _ => (),
        }

        Ok(())
    }
}
