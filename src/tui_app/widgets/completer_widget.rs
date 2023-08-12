use tui::widgets::StatefulWidget;
use unicode_width::UnicodeWidthStr;

use crate::ferrite_core::{palette::completer::Completer, theme::EditorTheme};

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
        let rows = (completer.options().len() / columns)
            .clamp(1, 10)
            .min(area.height.into());

        // clear inner area
        for i in 0..rows {
            buf.set_stringn(
                area.left(),
                area.bottom() - rows as u16 + i as u16,
                " ".repeat(area.width.into()),
                area.width.into(),
                self.theme.completer,
            );
        }

        for row in 0..rows {
            for col in 0..columns {
                let index = col * rows + row;
                let Some(option) = completer.options().get(index) else {
                    break;
                };
                let y = area.bottom() as usize - rows + row;
                let x = area.left() as usize + widest * col;
                let style = if Some(index) == completer.current() {
                    self.theme.completer_selected
                } else {
                    Default::default()
                };
                buf.set_stringn(x as u16, y as u16, option.display(), widest, style);
            }
        }
    }
}
