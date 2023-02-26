use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute, terminal,
};
use log::debug;
use slab::Slab;
use tui::layout::Rect;

use self::{
    input::get_default_mappings,
    widgets::{editor_widget::EditorWidget, palette_widget::CmdPaletteWidget},
};
use crate::{
    core::{
        buffer::Buffer,
        palette::{cmd, cmd_parser, CommandPalette},
        theme::EditorTheme,
    },
    tui_app::input::InputCommand,
    Args,
};

pub mod input;
mod widgets;

pub struct TuiApp {
    buffers: Slab<Buffer>,
    current_buffer_id: usize,
    theme: EditorTheme,
    palette: CommandPalette,
    palette_focus: bool,
}

impl TuiApp {
    pub fn new(args: Args) -> Result<Self> {
        let buffer = match args.file {
            Some(file) => Buffer::from_file(file)?,
            None => Buffer::new(),
        };

        let theme = EditorTheme::from_str(include_str!("../themes/onedark.toml"))?;
        let palette = CommandPalette::new();
        let palette_focus = false;

        let mut slab = Slab::new();
        let id = slab.insert(buffer);

        Ok(Self {
            buffers: slab,
            current_buffer_id: id,
            theme,
            palette,
            palette_focus,
        })
    }

    pub fn new_buffer_with_text(&mut self, text: &str) {
        let mut buffer = Buffer::new();
        buffer.set_text(text);
        let id = self.buffers.insert(buffer);
        self.current_buffer_id = id;
    }

    pub fn run(self) -> Result<()> {
        let Self {
            theme,
            mut current_buffer_id,
            mut buffers,
            mut palette,
            mut palette_focus,
            ..
        } = self;

        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode()?;
        execute!(
            stdout,
            terminal::EnterAlternateScreen,
            event::EnableMouseCapture,
        )?;
        let backend = tui::backend::CrosstermBackend::new(stdout);
        let mut terminal = tui::Terminal::new(backend)?;

        let default_mappings = get_default_mappings();

        let mut palette_reciver = None;

        loop {
            let input = match event::read()? {
                Event::Key(event) => {
                    if event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat {
                        debug!("{:?}", event);
                        match input::get_command_from_input(
                            event.code,
                            event.modifiers,
                            &default_mappings,
                        ) {
                            Some(input) => Some(input),
                            None => match event.code {
                                KeyCode::Char(ch) => Some(InputCommand::Char(ch)),
                                _ => None,
                            },
                        }
                    } else {
                        None
                    }
                }
                Event::Mouse(event) => match event.kind {
                    event::MouseEventKind::ScrollUp => Some(InputCommand::Scroll(-3)),
                    event::MouseEventKind::ScrollDown => Some(InputCommand::Scroll(3)),
                    _ => None,
                },
                Event::Paste(text) => Some(InputCommand::Insert(text)),
                _ => None,
            };

            if let Some(input) = input {
                match input {
                    InputCommand::Quit => break,
                    InputCommand::Escape if palette_focus => {
                        palette_focus = false;
                        palette.reset();
                    }
                    InputCommand::FocusPalette if !palette_focus => {
                        palette_focus = true;
                        palette_reciver = Some(palette.focus("> "));
                    }
                    input => {
                        if palette_focus {
                            let _ = palette.handle_input(input);
                        } else if let Err(err) = buffers[current_buffer_id].handle_input(input) {
                            palette.set_msg(err.to_string());
                        }
                    }
                }
            }

            if let Some(reciver) = &palette_reciver {
                if let Ok(cmd) = reciver.try_recv() {
                    palette_reciver = None;
                    palette.reset();
                    palette_focus = false;
                    match cmd_parser::parse_cmd(&cmd) {
                        Ok(cmd) => match cmd {
                            cmd::Command::OpenFile(path) => match Buffer::from_file(path) {
                                Ok(buffer) => {
                                    current_buffer_id = buffers.insert(buffer);
                                }
                                Err(err) => palette.set_msg(err.to_string()),
                            },
                            cmd::Command::SaveFile(path) => {
                                if let Err(err) = buffers[current_buffer_id].save(path) {
                                    palette.set_msg(err.to_string())
                                }
                            }
                            cmd::Command::Reload => {
                                if let Err(err) = buffers[current_buffer_id].reload() {
                                    palette.set_msg(err.to_string())
                                };
                            }
                            cmd::Command::Goto(line) => {
                                buffers[current_buffer_id].goto(line);
                            }
                            cmd::Command::Logger => todo!(),
                        },
                        Err(err) => palette.set_msg(&err.to_string()),
                    }
                }
            }

            terminal.draw(|f| {
                let size = f.size();
                let editor_size = Rect::new(size.x, size.y, size.width, size.height - 1);
                f.render_stateful_widget(
                    EditorWidget::new(&theme, !palette_focus),
                    editor_size,
                    &mut buffers[current_buffer_id],
                );

                let palette_size = Rect::new(size.x, size.height - 1, size.width, 1);
                f.render_stateful_widget(CmdPaletteWidget::new(&theme), palette_size, &mut palette);
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
