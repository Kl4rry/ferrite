use ferrite_core::{config::editor::Editor, file_explorer::FileExplorer, theme::EditorTheme};
use ferrite_utility::trim::trim_path;
use tui::{
    layout::Rect,
    widgets::{Clear, StatefulWidget, Widget},
};

use super::one_line_input_widget::OneLineInputWidget;
use crate::glue::convert_style;

pub struct FileExplorerWidget<'a> {
    theme: &'a EditorTheme,
    config: &'a Editor,
    has_focus: bool,
}

impl<'a> FileExplorerWidget<'a> {
    pub fn new(theme: &'a EditorTheme, config: &'a Editor, has_focus: bool) -> Self {
        Self {
            theme,
            config,
            has_focus,
        }
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

        if area.height > 2 {
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
                let is_dir = entry.file_type.is_dir();
                if is_dir {
                    let mut file = file_name.into_owned();
                    file.push('/');
                    file_name = file.into();
                }

                let style = if is_dir {
                    convert_style(&self.theme.file_explorer_directory)
                } else {
                    convert_style(&self.theme.text)
                };

                buf.set_stringn(area.x, area.y + i, &file_name, area.width as usize, style);
                if i as usize + start == state.index() {
                    buf.set_style(
                        Rect::new(area.x, area.y + i, area.width, 1),
                        convert_style(&self.theme.selection),
                    );
                }
            }
        }

        if area.height > 1 {
            let info_line_y = area.y + area.height - 1;

            // Its a bit bruh to do this every single frame
            let directory = if let Some(directories) = directories::UserDirs::new() {
                let home = directories.home_dir();
                let trimmed = trim_path(&home.to_string_lossy(), state.directory());
                if trimmed.len() < state.directory().to_string_lossy().len() {
                    format!("~/{trimmed}")
                } else {
                    trimmed
                }
            } else {
                state.directory().to_string_lossy().into()
            };

            buf.set_stringn(
                area.x,
                info_line_y,
                format!("Dir: {}", directory),
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

        {
            let prompt = "search: ";
            let input_line_y = area.y + area.height - 2;
            buf.set_stringn(
                area.x,
                input_line_y,
                prompt,
                area.width as usize,
                convert_style(&self.theme.text),
            );
            let input_line_area = Rect::new(
                area.x + prompt.len() as u16,
                input_line_y,
                area.width - prompt.len() as u16,
                1,
            );
            OneLineInputWidget::new(self.theme, self.config, self.has_focus && state.searching)
                .render(input_line_area, buf, &mut state.buffer);
        }
    }
}
