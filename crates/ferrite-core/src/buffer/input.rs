use super::{error::BufferError, Buffer};
use crate::cmd::Cmd;

impl Buffer {
    pub fn handle_input(&mut self, input: Cmd) -> Result<(), BufferError> {
        use Cmd::*;
        match input {
            MoveRight { expand_selection } => self.move_right_char(expand_selection),
            MoveLeft { expand_selection } => self.move_left_char(expand_selection),
            MoveUp {
                expand_selection,
                distance,
            } => self.move_up(expand_selection, distance),
            MoveDown {
                expand_selection,
                distance,
            } => self.move_down(expand_selection, distance),
            MoveRightWord { expand_selection } => self.move_right_word(expand_selection),
            MoveLeftWord { expand_selection } => self.move_left_word(expand_selection),
            MoveLine(dir) if !self.read_only => self.move_line(dir),
            Insert(text) if !self.read_only => self.insert_text(&text, true),
            Char(ch) if !self.read_only => self.insert_text(&String::from(ch), true),
            NewLine if !self.read_only => self.new_line(),
            Backspace if !self.read_only => self.backspace(),
            BackspaceWord if !self.read_only => self.backspace_word(),
            Delete if !self.read_only => self.delete(),
            DeleteWord if !self.read_only => self.delete_word(),
            Home { expand_selection } => self.home(expand_selection),
            End { expand_selection } => self.end(expand_selection),
            Eof { expand_selection } => self.eof(expand_selection),
            Start { expand_selection } => self.start(expand_selection),
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
            SelectArea { cursor, anchor } => self.select_area(cursor, anchor, true),
            NextMatch => self.next_match(),
            PrevMatch => self.prev_match(),
            ReplaceCurrentMatch => self.replace_current_match(),
            Undo if !self.read_only => self.undo(),
            Redo if !self.read_only => self.redo(),
            RevertBuffer if !self.read_only => self.revert_buffer(),
            _ => (),
        }

        if let Some(searcher) = &mut self.searcher {
            searcher.update_buffer(self.rope.clone(), None);
        }

        self.update_interact();

        Ok(())
    }
}
