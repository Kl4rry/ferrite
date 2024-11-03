use std::time::Duration;

use ferrite_core::{logger::LoggerState, theme::EditorTheme};
use tui::widgets::{Clear, StatefulWidget, Widget};

use crate::glue::convert_style;

pub struct LoggerWidget<'a> {
    theme: &'a EditorTheme,
    render_time: Duration,
    has_focus: bool,
}

impl<'a> LoggerWidget<'a> {
    pub fn new(theme: &'a EditorTheme, render_time: Duration, has_focus: bool) -> Self {
        Self {
            theme,
            render_time,
            has_focus,
        }
    }
}

impl StatefulWidget for LoggerWidget<'_> {
    type State = LoggerState;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        if area.area() == 0 {
            return;
        }

        Clear.render(area, buf);

        buf.set_style(area, convert_style(&self.theme.background));
        for y in 0..area.height.saturating_sub(1) {
            match state.messages.get(y as usize + state.lines_scrolled_up) {
                Some(msg) => {
                    let string = format!("{:>5} {} {}", msg.level, msg.target, msg.fields.message);
                    buf.set_stringn(
                        area.x,
                        area.top() + area.height - y - 2, // TODO fix this - 2
                        string,
                        area.width.into(),
                        convert_style(&self.theme.text),
                    );
                }
                None => break,
            }
        }

        let line_area = tui::layout::Rect {
            x: area.x,
            y: area.y + area.height - 1,
            width: area.width,
            height: 1,
        };

        let style = convert_style(if self.has_focus {
            &self.theme.info_line
        } else {
            &self.theme.info_line_unfocused
        });

        buf.set_style(line_area, style);
        #[cfg(not(feature = "talloc"))]
        let line = format!(" Frame time: {:?}", self.render_time,);

        #[cfg(feature = "talloc")]
        let line = format!(
            " Frame time: {:?} Heap memory usage: {} Heap allocations: {}, Frame allocations: {}",
            self.render_time,
            ferrite_core::byte_size::format_byte_size(
                ferrite_talloc::Talloc::total_memory_allocated()
            ),
            ferrite_talloc::Talloc::num_allocations(),
            ferrite_talloc::Talloc::phase_allocations()
        );

        buf.set_stringn(
            line_area.x,
            line_area.y,
            line,
            line_area.width.into(),
            style,
        );
    }
}
