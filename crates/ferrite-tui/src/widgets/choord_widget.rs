use ferrite_core::{
    cmd::Cmd,
    keymap::{Exclusiveness, Mapping},
    theme::EditorTheme,
};
use tui::{
    layout,
    widgets::{Block, BorderType, Borders, Clear, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::glue::convert_style;

pub struct ChoordWidget<'a> {
    theme: &'a EditorTheme,
    key_mappings: &'a [(Mapping, Cmd, Exclusiveness)],
}

impl<'a> ChoordWidget<'a> {
    pub fn new(theme: &'a EditorTheme, key_mappings: &'a [(Mapping, Cmd, Exclusiveness)]) -> Self {
        Self {
            theme,
            key_mappings,
        }
    }
}

impl Widget for ChoordWidget<'_> {
    fn render(self, total_area: layout::Rect, buf: &mut tui::buffer::Buffer) {
        let height = total_area.height.min(
            self.key_mappings
                .iter()
                .filter(|(_, cmd, _)| *cmd != Cmd::Escape && *cmd != Cmd::Choord)
                .count() as u16
                + 2,
        );

        let mut lines = Vec::new();
        let mut longest = 0;
        let mut left_col_width = 0;
        for (input, command, _) in self
            .key_mappings
            .iter()
            .filter(|(_, cmd, _)| *cmd != Cmd::Escape && *cmd != Cmd::Choord)
            .take(height.into())
        {
            let mapping = format!("{}{} ", input.keycode, input.modifiers);
            let cmd = command.to_string();
            longest = longest.max(mapping.width() + cmd.width() + 1);
            left_col_width = left_col_width.max(mapping.width());
            lines.push((mapping, cmd));
        }

        let width = total_area.width.min(longest as u16 + 4);

        if width < 3 || height < 3 {
            return;
        }

        let left = total_area.width - width;
        let top = total_area.height - height;
        let area = layout::Rect::new(left, top, width, height);

        Clear.render(area, buf);

        Block::default()
            .title("Choords")
            .borders(Borders::ALL)
            .border_style(convert_style(&self.theme.border))
            .border_type(BorderType::Plain)
            .style(convert_style(&self.theme.background))
            .render(area, buf);

        let inner_area = area.inner(layout::Margin::new(1, 1));
        for (i, (mapping, cmd)) in lines.into_iter().enumerate() {
            let mut line = format!(" {mapping}");
            line.push_str(&" ".repeat(left_col_width - mapping.width() + 1));
            line.push_str(&cmd);
            line.push_str(&" ".repeat(inner_area.width as usize - line.width() as usize));

            buf.set_stringn(
                inner_area.left(),
                inner_area.top() + i as u16,
                line,
                inner_area.width.into(),
                convert_style(&self.theme.text),
            );
        }
    }
}
