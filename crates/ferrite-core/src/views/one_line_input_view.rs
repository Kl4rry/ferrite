use std::{hash::Hash, sync::Arc};

use ferrite_geom::rect::Rect;
use ferrite_runtime::{Bounds, Painter, View};
use ferrite_utility::tui_buf_ext::TuiBufExt;
use unicode_width::UnicodeWidthStr;

use crate::{
    buffer::Buffer,
    config::editor::{CursorType, Editor},
    theme::EditorTheme,
};

pub struct OneLineInputState {
    pub buffer: Buffer,
    left_prompt: Option<String>,
    right_prompt: Option<String>,
}

impl Default for OneLineInputState {
    fn default() -> Self {
        Self::new()
    }
}

impl OneLineInputState {
    pub fn new() -> Self {
        let mut buffer = Buffer::builder().simple(true).build().unwrap();
        let view_id = buffer.create_view();
        buffer.set_view_lines(view_id, 1);
        buffer.views[view_id].clamp_cursor = true;
        Self {
            buffer,
            left_prompt: None,
            right_prompt: None,
        }
    }

    pub fn set_left_prompt(&mut self, left_prompt: String) -> &mut Self {
        self.left_prompt = Some(left_prompt);
        self
    }

    pub fn set_right_prompt(&mut self, right_prompt: String) -> &mut Self {
        self.right_prompt = Some(right_prompt);
        self
    }
}

pub struct OneLineInputView<I> {
    theme: Arc<EditorTheme>,
    config: Arc<Editor>,
    focused: bool,
    id: I,
}

impl<I> OneLineInputView<I> {
    pub fn new(theme: Arc<EditorTheme>, config: Arc<Editor>, focused: bool, id: I) -> Self {
        Self {
            theme,
            config,
            focused,
            id,
        }
    }
}

impl<I> View<OneLineInputState> for OneLineInputView<I>
where
    I: Hash + Copy + 'static,
{
    fn render(
        &self,
        OneLineInputState {
            buffer,
            left_prompt,
            right_prompt,
        }: &mut OneLineInputState,
        bounds: Bounds,
        painter: &mut Painter,
    ) {
        let area = bounds.grid_bounds();
        let layer = painter.create_layer(self.id, bounds);
        let mut layer = layer.lock().unwrap();
        let buf = &mut layer.buf;
        assert_eq!(area.height, 1);
        let view_id = buffer.get_first_view_or_create();
        buffer.set_view_lines(view_id, 1);
        buffer.set_view_columns(view_id, area.width);
        buffer.views[view_id].cursors.clear();
        buffer.views[view_id].clamp_cursor = true;
        let view = buffer.get_buffer_view(view_id);

        let mut left_prompt_width = 0;
        if let Some(left_prompt) = left_prompt {
            left_prompt_width = left_prompt.width();

            buf.draw_string(
                area.x as u16,
                area.y as u16,
                left_prompt,
                area.into(),
                self.theme.text,
            );
        }

        if area.x + left_prompt_width < area.width {
            let text_area = Rect {
                x: area.x + left_prompt_width,
                y: area.y,
                width: area.width.saturating_sub(left_prompt_width),
                height: 1,
            };

            buf.draw_string(
                text_area.x as u16,
                text_area.y as u16,
                view.lines[0].text.to_string(),
                text_area.into(),
                self.theme.text,
            );

            let cursor =
                buffer.cursor_grapheme_column(view_id, 0) as i64 - buffer.col_pos(view_id) as i64;
            let anchor =
                buffer.anchor_grapheme_column(view_id, 0) as i64 - buffer.col_pos(view_id) as i64;
            let start = cursor.min(anchor).clamp(0, text_area.width as i64);
            let end = cursor.max(anchor).clamp(0, text_area.width as i64);
            let rect = Rect {
                x: text_area.x + start as usize,
                y: text_area.y,
                width: (end - start) as usize,
                height: 1,
            };
            buf.set_style(rect.into(), self.theme.selection);

            let cursor_area = Rect {
                x: text_area.x + cursor as usize,
                y: text_area.y,
                width: 1,
                height: 1,
            };

            if cursor_area.intersects(&text_area) && self.focused {
                match self.config.gui.cursor_type {
                    CursorType::Line if painter.has_painter2d() => {
                        buf.set_style(
                            cursor_area.into(),
                            tui::style::Style::default()
                                .add_modifier(tui::style::Modifier::SLOW_BLINK),
                        );
                    }
                    _ => {
                        buf.set_style(
                            cursor_area.into(),
                            tui::style::Style::from(self.theme.text)
                                .add_modifier(tui::style::Modifier::REVERSED),
                        );
                    }
                }
            }
        }

        if let Some(right_prompt) = right_prompt {
            let right_prompt_width = right_prompt.width();

            if area.width > (right_prompt_width * 2 + 2) {
                buf.draw_string(
                    (area.x + area.width - right_prompt_width) as u16 - 1,
                    area.y as u16,
                    right_prompt,
                    area.into(),
                    self.theme.text,
                );
            }
        }
    }
}
