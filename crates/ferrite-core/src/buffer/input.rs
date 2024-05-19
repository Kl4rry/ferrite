use super::{error::BufferError, Buffer};
use crate::keymap::InputCommand;

impl Buffer {
    pub fn handle_input(&mut self, input: InputCommand) -> Result<(), BufferError> {
        use InputCommand::*;
        match input {
            MoveRight { shift } => self.move_right_char(shift),
            MoveLeft { shift } => self.move_left_char(shift),
            MoveUp { shift, distance } => self.move_up(shift, distance),
            MoveDown { shift, distance } => self.move_down(shift, distance),
            MoveRightWord { shift } => self.move_right_word(shift),
            MoveLeftWord { shift } => self.move_left_word(shift),
            MoveLine(dir) if !self.read_only => self.move_line(dir),
            Insert(text) if !self.read_only => self.insert_text(&text, true),
            Char(ch) if !self.read_only => self.insert_text(&String::from(ch), true),
            NewLine if !self.read_only => self.new_line(),
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
            PastePrimary(column, line) if !self.read_only => self.paste_primary(column, line),
            Tab { back } if !self.read_only => self.tab(back),
            VerticalScroll(distance) => self.vertical_scroll(distance),
            Escape => self.escape(),
            ClickCell(col, line) => self.handle_click(col, line),
            SelectArea { cursor, anchor } => self.select_area(cursor, anchor),
            NextMatch => self.next_match(),
            PrevMatch => self.prev_match(),
            Undo if !self.read_only => self.undo(),
            Redo if !self.read_only => self.redo(),
            RevertBuffer if !self.read_only => self.revert_buffer(),
            _ => (),
        }

        if let Some(searcher) = &mut self.searcher {
            searcher.update_buffer(self.rope.clone(), None);
        }

        Ok(())
    }
}
