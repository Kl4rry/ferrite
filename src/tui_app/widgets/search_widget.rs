use std::{borrow::Cow, marker::PhantomData};

use ropey::RopeSlice;
use tui::{
    layout::{Margin, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, StatefulWidget, Widget},
};
use unicode_width::UnicodeWidthStr;
use utility::graphemes::RopeGraphemeExt;

use crate::core::{
    search_buffer::{ResultProvider, SearchBuffer},
    theme::EditorTheme,
};

pub struct SearchWidget<'a, T> {
    theme: &'a EditorTheme,
    title: &'a str,
    _phantom: PhantomData<T>,
}

impl<'a, T> SearchWidget<'a, T> {
    pub fn new(theme: &'a EditorTheme, title: &'a str) -> Self {
        Self {
            theme,
            title,
            _phantom: PhantomData,
        }
    }
}

impl<T> StatefulWidget for SearchWidget<'_, T>
where
    T: ResultProvider,
{
    type State = SearchBuffer<T>;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        let main_block = Block::default()
            .title(self.title)
            .borders(Borders::all())
            .border_style(self.theme.border)
            .border_type(BorderType::Plain)
            .style(self.theme.background);
        main_block.render(area, buf);
        let inner_area = area.inner(&Margin {
            horizontal: 1,
            vertical: 1,
        });

        // clear inner area
        for i in 0..inner_area.height {
            buf.set_stringn(
                inner_area.x,
                inner_area.y + i as u16,
                " ".repeat(inner_area.width.into()),
                inner_area.width.into(),
                self.theme.text,
            );
        }

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

            buf.set_stringn(
                search_field_area.x,
                search_field_area.y,
                " > ",
                search_field_area.width.into(),
                self.theme.text,
            );

            let prompt_width = 3;
            buf.set_stringn(
                search_field_area.x + prompt_width,
                search_field_area.y,
                state
                    .search_field()
                    .rope()
                    .last_n_columns(search_field_area.width as usize - 3)
                    .to_string(),
                search_field_area.width.into(),
                self.theme.text,
            );
            let cursor = state.search_field().cursor_grapheme_column() as u16;
            let anchor = state.search_field().anchor_grapheme_column() as u16;
            let start = cursor.min(anchor);
            let end = cursor.max(anchor);
            buf.set_style(search_field_area, self.theme.text);
            let rect = Rect {
                x: search_field_area.x + prompt_width + start,
                y: search_field_area.y,
                width: end - start,
                height: 1,
            };
            buf.set_style(rect, self.theme.selection);

            buf.set_style(
                Rect {
                    x: search_field_area.x + prompt_width + cursor,
                    y: search_field_area.y,
                    width: 1,
                    height: 1,
                },
                Style::default().add_modifier(tui::style::Modifier::REVERSED),
            );
        }

        if inner_area.height < 3 {
            return;
        }

        {
            let mut result_area = inner_area;
            result_area.y += 2;
            result_area.height -= 2;

            let selected = state.selected();
            let result = state.provider().poll_result();

            for (i, result) in result.iter().enumerate() {
                if i >= result_area.height.into() {
                    break;
                }

                let padding: usize = 1;
                let width = result_area.width as usize - padding;

                let result = if result.width() > width - 3 {
                    let rope = RopeSlice::from(result.as_str());
                    let slice = rope.last_n_columns(width - 4);
                    let mut shorted = String::with_capacity(slice.len_bytes() + "…".len());
                    shorted.push('…');
                    shorted.push_str(slice.as_str().unwrap());
                    Cow::Owned(shorted)
                } else {
                    Cow::Borrowed(result.as_str())
                };

                let result = if i == selected {
                    format!(" > {result}")
                } else {
                    format!("   {result}")
                };

                buf.set_stringn(
                    result_area.x,
                    result_area.y + i as u16,
                    result,
                    width,
                    self.theme.text,
                );

                if i == selected {
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
    }
}
