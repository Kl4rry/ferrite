use std::{fs, io, path::PathBuf};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute, terminal,
};
use slab::Slab;
use tui::layout::{Margin, Rect};

use self::{
    event_loop::{TuiEventLoop, TuiEventLoopControlFlow},
    input::get_default_mappings,
    widgets::{
        editor_widget::EditorWidget, palette_widget::CmdPaletteWidget, search_widget::SearchWidget,
    },
};
use crate::{
    core::{
        buffer::Buffer,
        indent::Indentation,
        palette::{cmd, cmd_parser, CommandPalette},
        search_buffer::{fuzzy_file_find::FuzzyFileFindProvider, SearchBuffer},
        theme::EditorTheme,
    },
    tui_app::input::InputCommand,
    Args,
};

pub mod event_loop;
pub mod input;
mod widgets;

pub struct TuiApp {
    buffers: Slab<Buffer>,
    current_buffer_id: usize,
    theme: EditorTheme,
    palette: CommandPalette,
    file_finder: Option<SearchBuffer<FuzzyFileFindProvider>>,
    palette_focus: bool,
    event_loop: TuiEventLoop,
}

impl TuiApp {
    pub fn new(args: Args) -> Result<Self> {
        let buffer = match args.file {
            Some(file) if file.is_dir() => {
                std::env::set_current_dir(file)?;
                // TODO open file searcher here
                Buffer::new()
            }
            Some(file) => match Buffer::from_file(&file) {
                Ok(buffer) => buffer,
                Err(err) => match err.kind() {
                    io::ErrorKind::NotFound => Buffer::with_path(file),
                    _ => Buffer::new(),
                },
            },
            None => Buffer::new(),
        };

        let theme = EditorTheme::from_str(include_str!("../themes/onedark.toml"))?;

        let event_loop = TuiEventLoop::new();

        let palette = CommandPalette::new(event_loop.create_proxy());
        let palette_focus = false;

        let mut slab = Slab::new();
        let id = slab.insert(buffer);

        let file_finder = None;

        Ok(Self {
            buffers: slab,
            current_buffer_id: id,
            theme,
            palette,
            file_finder,
            palette_focus,
            event_loop,
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
            mut file_finder,
            event_loop,
            ..
        } = self;

        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode()?;
        execute!(
            stdout,
            event::EnableBracketedPaste,
            terminal::EnterAlternateScreen,
            event::EnableMouseCapture,
        )?;

        let backend = tui::backend::CrosstermBackend::new(stdout);
        let mut terminal = tui::Terminal::new(backend)?;

        let default_mappings = get_default_mappings();

        event_loop.run(|proxy, event, control_flow| match event {
            event_loop::TuiEvent::Crossterm(event) => {
                let input = match event {
                    Event::Key(event) => {
                        if event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat {
                            log::debug!("{:?}", event);
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
                    Event::Paste(text) => {
                        log::debug!("paste: {text}");
                        Some(InputCommand::Insert(text))
                    }
                    _ => None,
                };

                if let Some(input) = input {
                    match input {
                        InputCommand::Quit => {
                            let unsaved: Vec<_> = buffers
                                .iter()
                                .filter_map(|(_, buffer)| {
                                    if buffer.is_dirty() {
                                        Some(buffer.name().unwrap_or_else(|| "scratch".into()))
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            if unsaved.is_empty() {
                                *control_flow = TuiEventLoopControlFlow::Exit;
                            } else {
                                palette.set_msg(format!(
                                    "You have {} buffer(s): {:?}",
                                    unsaved.len(),
                                    unsaved
                                ));
                                *control_flow = TuiEventLoopControlFlow::Exit;
                            }
                        }
                        InputCommand::Escape if palette_focus => {
                            palette_focus = false;
                            palette.reset();
                        }
                        InputCommand::FocusPalette if !palette_focus => {
                            file_finder = None;
                            palette_focus = true;
                            palette.focus("> ", "command");
                        }
                        InputCommand::PromptGoto => {
                            file_finder = None;
                            palette_focus = true;
                            palette.focus("goto: ", "goto");
                        }
                        InputCommand::Escape if file_finder.is_some() => {
                            file_finder = None;
                        }
                        InputCommand::FindFile => {
                            palette.reset();
                            palette_focus = false;
                            file_finder = Some(SearchBuffer::new(FuzzyFileFindProvider::new(
                                std::env::current_dir().unwrap_or(PathBuf::from("/")),
                                proxy.clone(),
                            )));
                        }
                        input => {
                            if palette_focus {
                                let _ = palette.handle_input(input);
                            } else if let Some(finder) = &mut file_finder {
                                let _ = finder.handle_input(input);
                                if let Some(choice) = finder.get_choice() {
                                    file_finder = None;
                                    // FIXME dedupe this code
                                    // FIXME remove these unwrap
                                    let real_path = fs::canonicalize(&choice).unwrap();
                                    match buffers.iter().find(|(_, buffer)| {
                                        buffer
                                            .file()
                                            .map(|path| fs::canonicalize(path).unwrap())
                                            .as_deref()
                                            == Some(&real_path)
                                    }) {
                                        Some((id, _)) => current_buffer_id = id,
                                        None => match Buffer::from_file(choice) {
                                            Ok(buffer) => {
                                                current_buffer_id = buffers.insert(buffer);
                                            }
                                            Err(err) => palette.set_msg(err.to_string()),
                                        },
                                    }
                                }
                            } else if let Err(err) = buffers[current_buffer_id].handle_input(input)
                            {
                                palette.set_msg(err.to_string());
                            }
                        }
                    }
                }
            }
            event_loop::TuiEvent::AppEvent(event) => match event {
                event_loop::TuiAppEvent::PaletteEvent { mode, content } => match mode.as_str() {
                    "command" => {
                        palette.reset();
                        palette_focus = false;
                        match cmd_parser::parse_cmd(&content) {
                            Ok(cmd) => match cmd {
                                cmd::Command::OpenFile(path) => {
                                    // FIXME dedupe this code
                                    // FIXME remove these unwrap
                                    let real_path = fs::canonicalize(&path).unwrap();
                                    match buffers.iter().find(|(_, buffer)| {
                                        buffer
                                            .file()
                                            .map(|path| fs::canonicalize(path).unwrap())
                                            .as_deref()
                                            == Some(&real_path)
                                    }) {
                                        Some((id, _)) => current_buffer_id = id,
                                        None => match Buffer::from_file(path) {
                                            Ok(buffer) => {
                                                current_buffer_id = buffers.insert(buffer);
                                            }
                                            Err(err) => palette.set_msg(err.to_string()),
                                        },
                                    }
                                }
                                cmd::Command::SaveFile(path) => {
                                    if let Err(err) = buffers[current_buffer_id].save(path) {
                                        palette.set_msg(err.to_string())
                                    }
                                }
                                cmd::Command::Indent => match buffers[current_buffer_id].indent {
                                    Indentation::Tabs(amount) => {
                                        palette.set_msg(format!("{} tabs(s)", amount))
                                    }
                                    Indentation::Spaces(amount) => {
                                        palette.set_msg(format!("{} space(s)", amount))
                                    }
                                },
                                cmd::Command::Reload => {
                                    if let Err(err) = buffers[current_buffer_id].reload() {
                                        palette.set_msg(err.to_string())
                                    };
                                }
                                cmd::Command::Goto(line) => {
                                    buffers[current_buffer_id].goto(line);
                                }
                                cmd::Command::ForceQuit => {
                                    *control_flow = TuiEventLoopControlFlow::Exit;
                                }
                                cmd::Command::Logger => todo!(),
                            },
                            Err(err) => palette.set_msg(err.to_string()),
                        }
                    }
                    "goto" => {
                        palette.reset();
                        palette_focus = false;
                        if let Ok(line) = content.trim().parse::<i64>() {
                            buffers[current_buffer_id].goto(line);
                        }
                    }
                    _ => (),
                },
            },
            event_loop::TuiEvent::Render => {
                terminal
                    .draw(|f| {
                        let size = f.size();
                        let editor_size = Rect::new(size.x, size.y, size.width, size.height - 1);
                        f.render_stateful_widget(
                            EditorWidget::new(&theme, !palette_focus && file_finder.is_none()),
                            editor_size,
                            &mut buffers[current_buffer_id],
                        );

                        if let Some(file_finder) = &mut file_finder {
                            let size = size.inner(&Margin {
                                horizontal: 5,
                                vertical: 2,
                            });
                            f.render_stateful_widget(SearchWidget::new(&theme, "Open file"), size, file_finder);
                        }

                        let palette_size = Rect::new(size.x, size.height - 1, size.width, 1);
                        f.render_stateful_widget(
                            CmdPaletteWidget::new(&theme),
                            palette_size,
                            &mut palette,
                        );
                    })
                    .unwrap();
            }
        });

        terminal::disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            terminal::LeaveAlternateScreen,
            event::DisableMouseCapture,
            event::DisableBracketedPaste,
        )?;
        terminal.show_cursor()?;

        Ok(())
    }
}
