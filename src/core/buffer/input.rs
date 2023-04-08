use super::{error::BufferError, Buffer};
use crate::tui_app::input::InputCommand;

impl Buffer {
    pub fn handle_input(&mut self, input: InputCommand) -> Result<(), BufferError> {
        use InputCommand::*;
        match input {
            MoveRight { shift } => self.move_right_char(shift),
            MoveLeft { shift } => self.move_left_char(shift),
            MoveUp { shift } => self.move_up(shift),
            MoveDown { shift } => self.move_down(shift),
            MoveRightWord { shift } => self.move_right_word(shift),
            MoveLeftWord { shift } => self.move_left_word(shift),
            MoveLine(dir) if !self.read_only => self.move_line(dir),
            Insert(text) if !self.read_only => self.insert_text(&text),
            Char(ch) if !self.read_only => self.insert_text(&String::from(ch)),
            Backspace if !self.read_only => self.backspace(),
            BackspaceWord if !self.read_only => self.backspace_word(),
            Delete if !self.read_only => self.delete(),
            DeleteWord if !self.read_only => self.delete_word(),
            Home { shift } => self.home(shift),
            End { shift } => self.end(shift),
            Eof { shift } => self.eof(shift),
            Start { shift } => self.start(shift),
            SelectAll => self.select_all(),
            SelectLine => self.select_line(),
            SelectWord => self.select_word(),
            Copy => self.copy(),
            Cut if !self.read_only => self.cut(),
            Paste if !self.read_only => self.paste(),
            Tab { back } if !self.read_only => self.tab(back),
            VerticalScroll(distance) => self.vertical_scroll(distance),
            Escape => self.escape(),
            Save => self.save(None)?,
            _ => (),
        }

        Ok(())
    }
}
