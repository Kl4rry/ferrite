use std::{borrow::Cow, marker::PhantomData};

use ferrite_core::{
    config::editor::Editor,
    picker::{Matchable, Picker, Preview},
    theme::EditorTheme,
};
use ferrite_utility::graphemes::RopeGraphemeExt;
use ropey::RopeSlice;
use tui::{
    layout::{Margin, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, StatefulWidget, Widget},
};
use unicode_width::UnicodeWidthStr;

use super::{
    centered_text_widget::CenteredTextWidget, editor_widget::EditorWidget,
    one_line_input_widget::OneLineInputWidget,
};
use crate::glue::convert_style;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Right,
}

pub struct PickerWidget<'a, M> {
    theme: &'a EditorTheme,
    config: &'a Editor,
    title: &'a str,
    text_align: TextAlign,
    _phantom: PhantomData<M>,
}

impl<'a, M> PickerWidget<'a, M> {
    pub fn new(theme: &'a EditorTheme, config: &'a Editor, title: &'a str) -> Self {
        Self {
            theme,
            config,
            title,
            text_align: TextAlign::Right,
            _phantom: PhantomData,
        }
    }

    pub fn set_text_align(mut self, text_align: TextAlign) -> Self {
        self.text_align = text_align;
        self
    }
}

impl<M> StatefulWidget for PickerWidget<'_, M>
where
    M: Matchable,
{
    type State = Picker<M>;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        Clear.render(area, buf);

        let main_block = Block::default()
            .title(self.title)
            .borders(Borders::all())
            .border_style(convert_style(&self.theme.border))
            .border_type(BorderType::Plain)
            .style(convert_style(&self.theme.background));
        main_block.render(area, buf);
        let inner_area = area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });

        let search_field_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(convert_style(&self.theme.border))
            .border_type(BorderType::Plain)
            .style(convert_style(&self.theme.background));

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
                convert_style(&self.theme.text),
            );

            let prompt_width = PROMPT.width() as u16;
            let input_area = Rect {
                x: search_field_area.x + prompt_width,
                y: search_field_area.y,
                width: search_field_area.width.saturating_sub(prompt_width),
                height: 1,
            };

            OneLineInputWidget::new(self.theme, true).render(input_area, buf, state.search_field());

            let count = format!("{}/{}", state.get_matches().len(), state.get_total());
            let count_width = count.width();

            if input_area.width as usize > (count_width * 2 + 2) {
                buf.set_stringn(
                    input_area.x + input_area.width - count_width as u16 - 1,
                    input_area.y,
                    count,
                    input_area.width.into(),
                    convert_style(&self.theme.text),
                );
            }
        }

        if inner_area.height < 3 {
            return;
        }

        let (result_area, preview_area) = {
            let mut result_area = inner_area;
            result_area.y += 2;
            result_area.height -= 2;

            if inner_area.width > 60 && state.has_previewer() {
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
            let selected = state.selected();
            let result = state.get_matches();

            let start = selected / result_area.height as usize;
            let cursor_pos = selected % result_area.height as usize;

            for (i, (fuzzy_match, _)) in result
                .iter()
                .skip(start * result_area.height as usize)
                .take(result_area.height as usize)
                .enumerate()
            {
                let padding: usize = 1;
                let width = result_area.width as usize - padding;

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
                    convert_style(&self.theme.text),
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
                            style: convert_style(&self.theme.text),
                        });
                        current_idx = start;
                    }

                    // TODO: this min call is wierd and should not be needed
                    let end = (start + m.len).max(current_idx);
                    let s: String = chars[current_idx..end].iter().collect();
                    spans.push(Span {
                        content: s.into(),
                        style: convert_style(&self.theme.fuzzy_match),
                    });
                    current_idx = end;
                }

                if result.len() > current_idx {
                    let s: String = chars[current_idx..chars.len()].iter().collect();
                    spans.push(Span {
                        content: s.into(),
                        style: convert_style(&self.theme.text),
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
                        convert_style(&self.theme.selection),
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
                    .border_style(convert_style(&self.theme.border))
                    .border_type(BorderType::Plain)
                    .style(convert_style(&self.theme.background));
                preview_block.render(line_area, buf);
            }

            match state.get_current_preview() {
                Some(Preview::Buffer(buffer)) => {
                    let view_id = buffer.get_first_view_or_create();
                    let mut preview =
                        EditorWidget::new(self.theme, self.config, view_id, false, None, None);
                    preview.line_nr = false;
                    preview.info_line = false;
                    preview.render(preview_area, buf, buffer);
                }
                Some(Preview::SharedBuffer(buffer)) => {
                    let mut guard = buffer.lock().unwrap();
                    let view_id = guard.get_first_view_or_create();
                    let mut preview =
                        EditorWidget::new(self.theme, self.config, view_id, false, None, None);
                    preview.line_nr = false;
                    preview.info_line = false;
                    preview.render(preview_area, buf, &mut *guard);
                }
                Some(Preview::TooLarge) => {
                    let text = CenteredTextWidget::new(self.theme, "Too large");
                    text.render(preview_area, buf);
                }
                Some(Preview::Binary) => {
                    let text = CenteredTextWidget::new(self.theme, "Binary file");
                    text.render(preview_area, buf);
                }
                Some(Preview::Err) => {
                    let text = CenteredTextWidget::new(self.theme, "Error loading preview");
                    text.render(preview_area, buf);
                }
                _ => (),
            }
        }
    }
}
