use std::iter::repeat;

use anyhow::Result;

use ::crossterm::{
    event::{self, Event},
    terminal,
};
use crossterm::{
    event::{KeyCode, KeyEventKind},
    execute,
};
use editor::Editor;
use tui::widgets::StatefulWidget;
mod editor;

pub struct EditorWidget {}

impl EditorWidget {
    pub fn new() -> Self {
        Self {}
    }
}

impl StatefulWidget for EditorWidget {
    type State = Editor;

    fn render(
        self,
        area: tui::layout::Rect,
        buf: &mut tui::buffer::Buffer,
        editor: &mut Self::State,
    ) {
        let line_number_max_width = editor.buffer.len_lines().to_string().len();
        let width = area.width;
        let height = area.height;

        {
            let mut line_buffer = String::with_capacity(width.into());
            let view = editor.buffer.get_buffer_view(height.into());

            for (i, (line, line_number)) in view
                .lines
                .into_iter()
                .zip(
                    (editor.buffer.line_pos() + 1)
                        ..=editor.buffer.line_pos() + editor.buffer.len_lines(),
                )
                .enumerate()
            {
                let line_number = line_number.to_string();
                let line_number = format!(
                    "{}{} │",
                    repeat(' ')
                        .take(line_number_max_width - line_number.len())
                        .collect::<String>(),
                    line_number
                );
                buf.set_stringn(0, i as u16, &line_number, width.into(), Default::default());
                let left_offset = line_number.len();

                for chunk in line.chunks() {
                    line_buffer.push_str(chunk);
                }

                if line_buffer.as_bytes().last() == Some(&b'\n') {
                    line_buffer.pop();
                }

                if line_buffer.as_bytes().last() == Some(&b'\r') {
                    line_buffer.pop();
                }

                buf.set_stringn(
                    left_offset as u16,
                    i as u16,
                    &line_buffer,
                    width as usize - left_offset,
                    Default::default(),
                );

                line_buffer.clear();
            }
        }
    }
}

fn main() -> Result<()> {
    // let mut stdin = io::stdin().lock();
    // let mut content = String::new();
    // stdin.read_to_string(&mut content)?;
    // println!("{content}");

    {
        terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(
            stdout,
            terminal::EnterAlternateScreen,
            event::EnableMouseCapture
        )?;
        let backend = tui::backend::CrosstermBackend::new(stdout);
        let mut terminal = tui::Terminal::new(backend)?;

        let mut editor = Editor::new();
        editor
            .buffer
            .set_text(&std::fs::read_to_string("Cargo.toml")?);

        loop {
            match event::read()? {
                Event::Key(event) => {
                    if event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat {
                        match event.code {
                            KeyCode::Char('q') => {
                                break;
                            }
                            KeyCode::PageUp => {
                                editor.buffer.scroll_up();
                            }
                            KeyCode::PageDown => {
                                editor.buffer.scroll_down();
                            }
                            _ => (),
                        }
                    }
                }
                Event::Mouse(event) => match event.kind {
                    event::MouseEventKind::ScrollDown => {
                        editor.buffer.scroll_down();
                    }
                    event::MouseEventKind::ScrollUp => {
                        editor.buffer.scroll_up();
                    }
                    _ => (),
                },
                Event::Paste(_data) => (),
                _ => (),
            }

            terminal.draw(|f| {
                let size = f.size();
                f.render_stateful_widget(EditorWidget::new(), size, &mut editor);
            })?;
        }

        terminal::disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            terminal::LeaveAlternateScreen,
            event::DisableMouseCapture
        )?;
        terminal.show_cursor()?;
    }

    Ok(())
}
