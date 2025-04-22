use ferrite_core::{palette::completer::Completer, theme::EditorTheme};
use tui::widgets::{Clear, StatefulWidget, Widget};
use unicode_width::UnicodeWidthStr;

use crate::glue::convert_style;

pub struct CompleterWidget<'a> {
    theme: &'a EditorTheme,
}

impl<'a> CompleterWidget<'a> {
    pub fn new(theme: &'a EditorTheme) -> Self {
        Self { theme }
    }
}

impl StatefulWidget for CompleterWidget<'_> {
    type State = Completer;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        completer: &mut Self::State,
    ) {
        if completer.options().is_empty() {
            return;
        }

        let widest = completer
            .options()
            .iter()
            .map(|option| option.display().width())
            .max()
            .unwrap()
            + 8;

        let columns = (area.width as usize / widest).max(1);
        let rows = completer
            .options()
            .len()
            .div_ceil(columns)
            .clamp(1, 10)
            .min(area.height.into());

        let completer_area =
            tui::layout::Rect::new(area.x, area.bottom() - rows as u16, area.width, rows as u16);

        Clear.render(completer_area, buf);
        buf.set_style(completer_area, convert_style(&self.theme.completer));

        for row in 0..rows {
            for col in 0..columns {
                let index = col * rows + row;
                let Some(option) = completer.options().get(index) else {
                    break;
                };
                let y = area.bottom() as usize - rows + row;
                let x = area.left() as usize + widest * col;
                let style = if Some(index) == completer.current() {
                    convert_style(&self.theme.completer_selected)
                } else {
                    convert_style(&self.theme.completer)
                };
                buf.set_stringn(x as u16, y as u16, option.display(), widest, style);
            }
        }
    }
}
