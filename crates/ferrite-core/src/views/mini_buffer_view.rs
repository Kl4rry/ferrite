use std::{hash::Hash, sync::Arc};

use ferrite_runtime::{Bounds, Painter, View};
use ferrite_utility::tui_buf_ext::TuiBufExt;
use unicode_width::UnicodeWidthStr;

use crate::{
    config::editor::Editor, mini_buffer::MiniBuffer, theme::EditorTheme,
    views::editor_view::EditorView,
};

pub struct MiniBufferView<I> {
    theme: Arc<EditorTheme>,
    editor_view: EditorView,
    id: I,
}

impl<I> MiniBufferView<I> {
    pub fn new(theme: Arc<EditorTheme>, config: Arc<Editor>, focused: bool, id: I) -> Self {
        let mut editor_view =
            EditorView::new(None, config.clone(), theme.clone(), focused, None, None);
        editor_view.line_nr = false;
        editor_view.info_line = false;
        editor_view.draw_rulers = false;
        editor_view.ceil_surface_size = false;
        editor_view.scrollbar = false;
        editor_view.highlight_cursor_line = false;

        Self {
            theme,
            editor_view,
            id,
        }
    }
}

impl<I> View<MiniBuffer> for MiniBufferView<I>
where
    I: Hash + Copy + 'static,
{
    fn render(
        &self,
        MiniBuffer {
            buffer,
            left_prompt,
            right_prompt,
            one_line,
        }: &mut MiniBuffer,
        bounds: Bounds,
        painter: &mut Painter,
    ) {
        let area = bounds.grid_bounds();
        let layer = painter.create_layer(self.id, bounds);
        let mut layer = layer.lock().unwrap();
        let buf = &mut layer.buf;
        let view_id = buffer.get_first_view_or_create();
        if *one_line {
            buffer.set_view_lines(view_id, 1);
        }
        buffer.views[view_id].clamp_cursor = true;
        buffer.set_view_columns(view_id, area.width);
        // This is a bit cheaty and will result in the buffer being clipped
        // weirdly if it is larger then the screen
        buffer.views[view_id].line_pos = 0.0;

        let view_bounds = match left_prompt {
            Some(left_prompt) => {
                let width = left_prompt.width();
                buf.draw_string_i32(
                    area.x as i32,
                    area.y as i32,
                    left_prompt,
                    area.into(),
                    self.theme.text,
                );
                let mut view_bounds = bounds.view_bounds();
                view_bounds =
                    view_bounds.margin_left((bounds.cell_size().x * width as f32) as usize);
                view_bounds
            }
            None => bounds.view_bounds(),
        };

        let text_bounds = Bounds::new(view_bounds, bounds.cell_size(), bounds.rounding);

        self.editor_view.render(buffer, text_bounds, painter);

        // TODO: right prompt is currently covered
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
