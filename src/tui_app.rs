use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, terminal,
};
use tui::layout::Rect;

use self::widgets::editor_widget::EditorWidget;
use crate::{
    core::{editor::Editor, theme::EditorTheme},
    Args,
};

mod widgets;

pub struct TuiApp {
    editor: Editor,
    theme: EditorTheme,
}

impl TuiApp {
    pub fn new(args: Args) -> Result<Self> {
        let editor = match args.file {
            Some(file) => Editor::from_file(file)?,
            None => Editor::new(),
        };

        let theme = EditorTheme::from_str(include_str!("../themes/onedark.toml"))?;

        Ok(Self { editor, theme })
    }

    pub fn run(self) -> Result<()> {
        let Self { mut editor, theme } = self;

        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode()?;
        execute!(
            stdout,
            terminal::EnterAlternateScreen,
            event::EnableMouseCapture,
        )?;
        let backend = tui::backend::CrosstermBackend::new(stdout);
        let mut terminal = tui::Terminal::new(backend)?;

        loop {
            match event::read()? {
                Event::Key(event) => {
                    if event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat {
                        match event.code {
                            KeyCode::Char('q')
                                if event.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                break;
                            }
                            KeyCode::Char(c) => {
                                let s = String::from(c);
                                editor.buffer.insert_text(&s);
                            }
                            KeyCode::Enter => {
                                editor.buffer.insert_text("\n");
                            }
                            KeyCode::Backspace => {
                                editor.buffer.backspace();
                            }
                            KeyCode::PageUp => {
                                editor.buffer.scroll(-50);
                            }
                            KeyCode::PageDown => {
                                editor.buffer.scroll(50);
                            }
                            KeyCode::Right => {
                                editor.buffer.move_right();
                            }
                            KeyCode::Left => {
                                editor.buffer.move_left();
                            }
                            KeyCode::Down => {
                                editor.buffer.move_down();
                            }
                            KeyCode::Up => {
                                editor.buffer.move_up();
                            }
                            _ => (),
                        }
                    }
                }
                Event::Mouse(event) => match event.kind {
                    event::MouseEventKind::ScrollUp => {
                        editor.buffer.scroll(-3);
                    }
                    event::MouseEventKind::ScrollDown => {
                        editor.buffer.scroll(3);
                    }
                    _ => (),
                },
                Event::Paste(_data) => (),
                _ => (),
            }

            terminal.draw(|f| {
                let size = f.size();
                let editor_size = Rect::new(size.x, size.y, size.width, size.height - 1);
                f.render_stateful_widget(EditorWidget::new(&theme), editor_size, &mut editor);
            })?;
        }

        terminal::disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            terminal::LeaveAlternateScreen,
            event::DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }
}
