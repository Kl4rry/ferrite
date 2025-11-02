use std::{borrow::Cow, sync::Arc};

use ferrite_runtime::{Bounds, Painter, View};
use ferrite_utility::graphemes::RopeGraphemeExt;
use ropey::RopeSlice;
use tui::{
    layout::{Margin, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Widget},
};
use unicode_width::UnicodeWidthStr;

use super::{
    centered_text_view::CenteredTextView, editor_view::EditorView,
    one_line_input_view::OneLineInputView,
};
use crate::{
    config::editor::Editor,
    picker::{Matchable, Picker, Preview},
    theme::EditorTheme,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Right,
}

pub struct PickerView {
    theme: Arc<EditorTheme>,
    config: Arc<Editor>,
    title: &'static str,
    text_align: TextAlign,
}

impl PickerView {
    pub fn new(theme: Arc<EditorTheme>, config: Arc<Editor>, title: &'static str) -> Self {
        Self {
            theme,
            config,
            title,
            text_align: TextAlign::Right,
        }
    }

    pub fn set_text_align(mut self, text_align: TextAlign) -> Self {
        self.text_align = text_align;
        self
    }
}

impl<M> View<Picker<M>> for PickerView
where
    M: Matchable,
{
    fn render(&self, picker: &mut Picker<M>, bounds: Bounds, painter: &mut Painter) {
        let area = bounds.grid_bounds();
        let layer = painter.create_layer(picker.unique_id(), bounds);
        let mut layer = layer.lock().unwrap();
        let buf = &mut layer.buf;

        let main_block = Block::default()
            .title(self.title)
            .borders(Borders::all())
            .border_style(self.theme.border)
            .border_type(BorderType::Plain)
            .style(self.theme.background);
        main_block.render(area.into(), buf);
        let inner_area = tui::layout::Rect::from(area).inner(Margin {
            horizontal: 1,
            vertical: 1,
        });

        let search_field_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(self.theme.border)
            .border_type(BorderType::Plain)
            .style(self.theme.background);

        let mut search_field_area = inner_area;
        search_field_area.height = 2;
        search_field_block.render(search_field_area, buf);
        search_field_area.height = 1;

        {
            const PROMPT: &str = " > ";
            buf.set_stringn(
                search_field_area.x,
                search_field_area.y,
                PROMPT,
                search_field_area.width.into(),
                self.theme.text,
            );

            let prompt_width = PROMPT.width() as u16;
            let input_area = Rect {
                x: search_field_area.x + prompt_width,
                y: search_field_area.y,
                width: search_field_area.width.saturating_sub(prompt_width),
                height: 1,
            };

            OneLineInputView::new(
                self.theme.clone(),
                self.config.clone(),
                true,
                (picker.unique_id(), "input field"),
            )
            .set_right_prompt(format!(
                "{}/{}",
                picker.get_matches().len(),
                picker.get_total()
            ))
            .render(
                picker.search_field(),
                Bounds::from_grid_bounds(input_area.into(), bounds.cell_size(), bounds.rounding),
                painter,
            );
        }

        if inner_area.height < 3 {
            return;
        }

        let (result_area, preview_area) = {
            let mut result_area = inner_area;
            result_area.y += 2;
            result_area.height -= 2;

            if inner_area.width > 60 && picker.has_previewer() {
                let total_width = result_area.width;
                result_area.width /= 2;
                let rem = total_width - result_area.width * 2;
                let mut preview_area = result_area;
                preview_area.x += result_area.width + 1;
                if rem == 0 {
                    preview_area.width -= 1;
                }
                (result_area, preview_area)
            } else {
                (result_area, Rect::new(0, 0, 0, 0))
            }
        };

        {
            let selected = picker.selected();
            let result = picker.get_matches();

            let start = selected / result_area.height as usize;
            let cursor_pos = selected % result_area.height as usize;

            for (i, (fuzzy_match, _)) in result
                .iter()
                .skip(start * result_area.height as usize)
                .take(result_area.height as usize)
                .enumerate()
            {
                let padding: usize = 1;
                let width = (result_area.width as usize).saturating_sub(padding);

                let elipsies = "…";
                let elipsies_len = elipsies.len();

                let mut diff = 0;
                let result = if fuzzy_match.item.display().width() > width - 3
                    && self.text_align == TextAlign::Right
                {
                    let display = fuzzy_match.item.display();
                    let rope = RopeSlice::from(display.as_ref());
                    let slice = rope.last_n_columns(width - 4);
                    let mut shorted = String::with_capacity(slice.len_bytes() + elipsies_len);
                    shorted.push_str(elipsies);
                    shorted.push_str(slice.as_str().unwrap());

                    let real_len = rope.len_chars();
                    let slice_len = slice.len_chars();

                    diff = real_len as i64 - slice_len as i64 - elipsies.chars().count() as i64;

                    Cow::Owned(shorted)
                } else {
                    fuzzy_match.item.display()
                };

                let prompt = if i == cursor_pos {
                    " > ".to_string()
                } else {
                    "   ".to_string()
                };

                buf.set_stringn(
                    result_area.x,
                    result_area.y + i as u16,
                    &prompt,
                    width,
                    self.theme.text,
                );

                let mut spans = Vec::new();
                let mut current_idx = 0;
                let chars: Vec<_> = result.chars().collect();
                for i in 0..fuzzy_match.matches.len() {
                    let m = fuzzy_match.matches[i];
                    let start = (m.start as i64).saturating_sub(diff).max(0) as usize;
                    if start > current_idx {
                        let s: String = chars[current_idx..start].iter().collect();
                        spans.push(Span {
                            content: s.into(),
                            style: self.theme.text.into(),
                        });
                        current_idx = start;
                    }

                    // TODO: this min call is wierd and should not be needed
                    let end = (start + m.len).max(current_idx);
                    let s: String = chars[current_idx..end].iter().collect();
                    spans.push(Span {
                        content: s.into(),
                        style: self.theme.fuzzy_match.into(),
                    });
                    current_idx = end;
                }

                if result.len() > current_idx {
                    let s: String = chars[current_idx..chars.len()].iter().collect();
                    spans.push(Span {
                        content: s.into(),
                        style: self.theme.text.into(),
                    });
                }

                buf.set_line(
                    result_area.x + prompt.width() as u16,
                    result_area.y + i as u16,
                    &Line::from(spans),
                    (width - prompt.width()) as u16,
                );

                if i == cursor_pos {
                    buf.set_style(
                        Rect {
                            x: result_area.x,
                            y: result_area.y + i as u16,
                            width: result_area.width,
                            height: 1,
                        },
                        self.theme.selection,
                    );
                }
            }
        }

        if preview_area.area() > 0 {
            {
                let line_area =
                    Rect::new(preview_area.x - 1, preview_area.y, 1, preview_area.height);
                let preview_block = Block::default()
                    .borders(Borders::LEFT)
                    .border_style(self.theme.border)
                    .border_type(BorderType::Plain)
                    .style(self.theme.background);
                preview_block.render(line_area, buf);
            }

            match picker.get_current_preview() {
                Some(Preview::Buffer(buffer)) => {
                    // TODO: load buffer view pos
                    let view_id = buffer.get_first_view_or_create();
                    let mut preview = EditorView::new(
                        view_id,
                        self.config.clone(),
                        self.theme.clone(),
                        false,
                        None,
                        None,
                    );
                    preview.line_nr = false;
                    preview.info_line = false;
                    preview.render(
                        buffer,
                        Bounds::from_grid_bounds(
                            preview_area.into(),
                            bounds.cell_size(),
                            bounds.rounding,
                        ),
                        painter,
                    );
                }
                Some(Preview::SharedBuffer(buffer)) => {
                    // TODO: load buffer view pos
                    let mut guard = buffer.lock().unwrap();
                    let view_id = guard.get_first_view_or_create();
                    let mut preview = EditorView::new(
                        view_id,
                        self.config.clone(),
                        self.theme.clone(),
                        false,
                        None,
                        None,
                    );
                    preview.line_nr = false;
                    preview.info_line = false;
                    preview.render(
                        &mut *guard,
                        Bounds::from_grid_bounds(
                            preview_area.into(),
                            bounds.cell_size(),
                            bounds.rounding,
                        ),
                        painter,
                    );
                }
                Some(Preview::TooLarge) => {
                    let text = CenteredTextView::new(
                        self.theme.clone(),
                        "Too large".into(),
                        (picker.unique_id(), "picker preview text"),
                    );
                    text.render(
                        &mut (),
                        Bounds::from_grid_bounds(
                            preview_area.into(),
                            bounds.cell_size(),
                            bounds.rounding,
                        ),
                        painter,
                    );
                }
                Some(Preview::Binary) => {
                    let text = CenteredTextView::new(
                        self.theme.clone(),
                        "Binary file".into(),
                        (picker.unique_id(), "picker preview text"),
                    );
                    text.render(
                        &mut (),
                        Bounds::from_grid_bounds(
                            preview_area.into(),
                            bounds.cell_size(),
                            bounds.rounding,
                        ),
                        painter,
                    );
                }
                Some(Preview::Err) => {
                    let text = CenteredTextView::new(
                        self.theme.clone(),
                        "Error loading preview".into(),
                        (picker.unique_id(), "picker preview text"),
                    );
                    text.render(
                        &mut (),
                        Bounds::from_grid_bounds(
                            preview_area.into(),
                            bounds.cell_size(),
                            bounds.rounding,
                        ),
                        painter,
                    );
                }
                _ => (),
            }
        }
    }
}
