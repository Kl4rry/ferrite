use super::{Buffer, ViewId, error::BufferError};
use crate::cmd::Cmd;

impl Buffer {
    pub fn handle_input(&mut self, view_id: ViewId, input: Cmd) -> Result<(), BufferError> {
        use Cmd::*;
        match input {
            MoveRight { expand_selection } => self.move_right_char(view_id, expand_selection),
            MoveLeft { expand_selection } => self.move_left_char(view_id, expand_selection),
            MoveUp {
                expand_selection,
                create_cursor,
                distance,
            } => self.move_up(view_id, expand_selection, create_cursor, distance),
            MoveDown {
                expand_selection,
                create_cursor,
                distance,
            } => self.move_down(view_id, expand_selection, create_cursor, distance),
            MoveRightWord { expand_selection } => self.move_right_word(view_id, expand_selection),
            MoveLeftWord { expand_selection } => self.move_left_word(view_id, expand_selection),
            MoveLine { direction } if !self.read_only => self.move_line(view_id, direction),
            Insert { text } if !self.read_only => self.insert_text(view_id, &text, true),
            Char { ch } if !self.read_only => self.insert_text(view_id, &String::from(ch), true),
            Backspace if !self.read_only => self.backspace(view_id),
            BackspaceWord if !self.read_only => self.backspace_word(view_id),
            Delete if !self.read_only => self.delete(view_id),
            DeleteWord if !self.read_only => self.delete_word(view_id),
            Home { expand_selection } => self.home(view_id, expand_selection),
            End { expand_selection } => self.end(view_id, expand_selection),
            Eof { expand_selection } => self.eof(view_id, expand_selection),
            Start { expand_selection } => self.start(view_id, expand_selection),
            SelectAll => self.select_all(view_id),
            SelectWord => self.select_word(view_id),
            SelectLine => self.select_line(view_id),
            RemoveLine if !self.read_only => self.remove_line(view_id),
            Copy => self.copy(view_id),
            Cut if !self.read_only => self.cut(view_id),
            Paste if !self.read_only => self.paste(view_id),
            PastePrimary { column, line } if !self.read_only => {
                self.paste_primary(view_id, column, line)
            }
            TabOrIndent { back } if !self.read_only => self.tab_or_indent(view_id, back),
            VerticalScroll { distance } => self.vertical_scroll(view_id, distance),
            VerticalScrollTo { position } => self.vertical_scroll_to(view_id, position),
            Escape => self.escape(view_id),
            ClickCell {
                spawn_cursor,
                column,
                line,
            } => self.handle_click(view_id, spawn_cursor, column, line),
            SelectArea { cursor, anchor } => self.select_area(view_id, cursor, anchor, true),
            NextMatch => self.next_match(view_id),
            PrevMatch => self.prev_match(view_id),
            ReplaceCurrentMatch if !self.read_only => self.replace_current_match(view_id),
            Undo if !self.read_only => self.undo(view_id),
            Redo if !self.read_only => self.redo(view_id),
            RevertBuffer if !self.read_only => self.revert_buffer(view_id),
            Number { start } if !self.read_only => self.number(view_id, start),
            TrimTrailingWhitespace if !self.read_only => self.trim_trailing_whitespace(),
            NewLineWithoutBreaking if !self.read_only => self.new_line_without_breaking(view_id),
            NewLineAboveWithoutBreaking if !self.read_only => {
                self.new_line_above_without_breaking(view_id)
            }
            SelectAllMatching if !self.read_only => self.select_all_matching(view_id),
            DeleteToEndOfLine if !self.read_only => self.delete_to_end_of_line(view_id),
            BackspaceToStartOfLine if !self.read_only => self.backspace_to_start_of_line(view_id),
            PageUp => self.page_up(view_id),
            PageDown => self.page_down(view_id),
            Nop => self.update_interact(Some(view_id)),
            _ => return Ok(()),
        }

        self.update_searchers();
        self.update_interact(Some(view_id));

        Ok(())
    }
}
