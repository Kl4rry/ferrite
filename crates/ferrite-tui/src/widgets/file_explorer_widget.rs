use ferrite_core::{file_explorer::FileExplorer, theme::EditorTheme};
use tui::{
    layout::Rect,
    widgets::{Clear, StatefulWidget, Widget},
};

use crate::glue::convert_style;

pub struct FileExplorerWidget<'a> {
    theme: &'a EditorTheme,
    has_focus: bool,
}

impl<'a> FileExplorerWidget<'a> {
    pub fn new(theme: &'a EditorTheme, has_focus: bool) -> Self {
        Self { theme, has_focus }
    }
}

impl StatefulWidget for FileExplorerWidget<'_> {
    type State = FileExplorer;

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

        if area.height > 1 {
            let height = area.height.saturating_sub(1);
            let page = state.index() / height as usize;
            let start = page * height as usize;

            let entries = state.entries();
            for i in 0..height {
                let index = start + i as usize;
                let Some(entry) = entries.get(index) else {
                    continue;
                };
                let Some(file_name) = entry.path.file_name() else {
                    continue;
                };
                let mut file_name = file_name.to_string_lossy();
                if entry.file_type.is_dir() {
                    let mut file = file_name.into_owned();
                    file.push('/');
                    file_name = file.into();
                }

                let style = if i as usize + start == state.index() {
                    convert_style(&self.theme.selection)
                } else {
                    convert_style(&self.theme.text)
                };

                buf.set_stringn(area.x, area.y + i, &file_name, area.width as usize, style);
            }
        }

        let info_line_y = area.y + area.height - 1;
        buf.set_stringn(
            area.x,
            info_line_y,
            format!("Dir: {}", state.directory().to_string_lossy()),
            area.width as usize,
            convert_style(&self.theme.text),
        );
        let info_line_area = Rect::new(area.x, info_line_y, area.width, 1);
        if self.has_focus {
            buf.set_style(info_line_area, convert_style(&self.theme.info_line));
        } else {
            buf.set_style(
                info_line_area,
                convert_style(&self.theme.info_line_unfocused),
            );
        }
    }
}
