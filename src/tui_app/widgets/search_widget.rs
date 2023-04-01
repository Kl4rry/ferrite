use std::{borrow::Cow, marker::PhantomData};

use ropey::RopeSlice;
use tui::{
    layout::{Margin, Rect},
    widgets::{Block, BorderType, Borders, StatefulWidget, Widget},
};
use unicode_width::UnicodeWidthStr;
use utility::graphemes::RopeGraphemeExt;

use super::one_line_input_widget::OneLineInputWidget;
use crate::core::{
    search_buffer::{Matchable, SearchBuffer},
    theme::EditorTheme,
};

pub struct SearchWidget<'a, M> {
    theme: &'a EditorTheme,
    title: &'a str,
    _phantom: PhantomData<M>,
}

impl<'a, M> SearchWidget<'a, M> {
    pub fn new(theme: &'a EditorTheme, title: &'a str) -> Self {
        Self {
            theme,
            title,
            _phantom: PhantomData,
        }
    }
}

impl<M> StatefulWidget for SearchWidget<'_, M>
where
    M: Matchable,
{
    type State = SearchBuffer<M>;

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
                inner_area.y + i,
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

            OneLineInputWidget::new(self.theme).render(input_area, buf, state.search_field());
        }

        if inner_area.height < 3 {
            return;
        }

        {
            let mut result_area = inner_area;
            result_area.y += 2;
            result_area.height -= 2;

            let selected = state.selected();
            let result = state.get_result();

            let start = selected / result_area.height as usize;
            let cursor_pos = selected % result_area.height as usize;

            for (i, result) in result
                .iter()
                .skip(start * result_area.height as usize)
                .take(result_area.height as usize)
                .enumerate()
            {
                let padding: usize = 1;
                let width = result_area.width as usize - padding;

                let result = if result.item.display().width() > width - 3 {
                    let display = result.item.display();
                    let rope = RopeSlice::from(display.as_ref());
                    let slice = rope.last_n_columns(width - 4);
                    let mut shorted = String::with_capacity(slice.len_bytes() + "…".len());
                    shorted.push('…');
                    shorted.push_str(slice.as_str().unwrap());
                    Cow::Owned(shorted)
                } else {
                    result.item.display()
                };

                let result = if i == cursor_pos {
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
    }
}
