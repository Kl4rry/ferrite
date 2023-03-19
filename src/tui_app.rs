use std::{
    fs,
    io::{self, Stdout},
    path::{Path, PathBuf},
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute, terminal,
};
use slab::Slab;
use tui::layout::{Margin, Rect};

use self::{
    event_loop::{TuiAppEvent, TuiEvent, TuiEventLoop, TuiEventLoopControlFlow, TuiEventLoopProxy},
    input::{get_default_mappings, Exclusiveness, Mapping},
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
    terminal: tui::Terminal<tui::backend::CrosstermBackend<Stdout>>,
    buffers: Slab<Buffer>,
    current_buffer_id: usize,
    theme: EditorTheme,
    palette: CommandPalette,
    file_finder: Option<SearchBuffer<FuzzyFileFindProvider>>,
    palette_focus: bool,
    key_mappings: Vec<(Mapping, InputCommand, Exclusiveness)>,
}

impl TuiApp {
    pub fn new(args: Args, proxy: TuiEventLoopProxy) -> Result<Self> {
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

        let palette = CommandPalette::new(proxy);
        let palette_focus = false;

        let mut slab = Slab::new();
        let id = slab.insert(buffer);

        let file_finder = None;

        Ok(Self {
            terminal: tui::Terminal::new(tui::backend::CrosstermBackend::new(std::io::stdout()))?,
            buffers: slab,
            current_buffer_id: id,
            theme,
            palette,
            file_finder,
            palette_focus,
            key_mappings: get_default_mappings(),
        })
    }

    pub fn new_buffer_with_text(&mut self, text: &str) {
        let mut buffer = Buffer::new();
        buffer.set_text(text);
        let id = self.buffers.insert(buffer);
        self.current_buffer_id = id;
    }

    pub fn run(mut self, event_loop: TuiEventLoop) -> Result<()> {
        let mut stdout = std::io::stdout();
        terminal::enable_raw_mode()?;
        execute!(
            stdout,
            event::EnableBracketedPaste,
            terminal::EnterAlternateScreen,
            event::EnableMouseCapture,
        )?;

        event_loop.run(|proxy, event, control_flow| self.handle_event(proxy, event, control_flow));

        terminal::disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            terminal::LeaveAlternateScreen,
            event::DisableMouseCapture,
            event::DisableBracketedPaste,
        )?;
        self.terminal.show_cursor()?;

        Ok(())
    }

    pub fn handle_event(
        &mut self,
        proxy: &TuiEventLoopProxy,
        event: TuiEvent,
        control_flow: &mut TuiEventLoopControlFlow,
    ) {
        match event {
            event_loop::TuiEvent::Crossterm(event) => {
                self.handle_crossterm_event(proxy, event, control_flow)
            }
            event_loop::TuiEvent::AppEvent(event) => {
                self.handle_app_event(proxy, event, control_flow)
            }
            event_loop::TuiEvent::Render => self.render(),
        }
    }

    pub fn render(&mut self) {
        self.terminal
            .draw(|f| {
                let size = f.size();
                let editor_size = Rect::new(size.x, size.y, size.width, size.height - 1);
                f.render_stateful_widget(
                    EditorWidget::new(
                        &self.theme,
                        !self.palette_focus && self.file_finder.is_none(),
                    ),
                    editor_size,
                    &mut self.buffers[self.current_buffer_id],
                );

                if let Some(file_finder) = &mut self.file_finder {
                    let size = size.inner(&Margin {
                        horizontal: 5,
                        vertical: 2,
                    });
                    f.render_stateful_widget(
                        SearchWidget::new(&self.theme, "Open file"),
                        size,
                        file_finder,
                    );
                }

                let palette_size = Rect::new(size.x, size.height - 1, size.width, 1);
                f.render_stateful_widget(
                    CmdPaletteWidget::new(&self.theme),
                    palette_size,
                    &mut self.palette,
                );
            })
            .unwrap();
    }

    pub fn handle_crossterm_event(
        &mut self,
        proxy: &TuiEventLoopProxy,
        event: event::Event,
        control_flow: &mut TuiEventLoopControlFlow,
    ) {
        {
            let input = match event {
                Event::Key(event) => {
                    if event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat {
                        log::debug!("{:?}", event);
                        match input::get_command_from_input(
                            event.code,
                            event.modifiers,
                            &self.key_mappings,
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
                        let unsaved: Vec<_> = self
                            .buffers
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
                            self.palette.set_msg(format!(
                                "You have {} buffer(s): {:?}",
                                unsaved.len(),
                                unsaved
                            ));
                            *control_flow = TuiEventLoopControlFlow::Exit;
                        }
                    }
                    InputCommand::Escape if self.palette_focus => {
                        self.palette_focus = false;
                        self.palette.reset();
                    }
                    InputCommand::FocusPalette if !self.palette_focus => {
                        self.file_finder = None;
                        self.palette_focus = true;
                        self.palette.focus("> ", "command");
                    }
                    InputCommand::PromptGoto => {
                        self.file_finder = None;
                        self.palette_focus = true;
                        self.palette.focus("goto: ", "goto");
                    }
                    InputCommand::Escape if self.file_finder.is_some() => {
                        self.file_finder = None;
                    }
                    InputCommand::FindFile => {
                        self.palette.reset();
                        self.palette_focus = false;
                        self.file_finder = Some(SearchBuffer::new(FuzzyFileFindProvider::new(
                            std::env::current_dir().unwrap_or(PathBuf::from("/")),
                            proxy.clone(),
                        )));
                    }
                    input => {
                        if self.palette_focus {
                            let _ = self.palette.handle_input(input);
                        } else if let Some(finder) = &mut self.file_finder {
                            let _ = finder.handle_input(input);
                            if let Some(path) = finder.get_choice() {
                                self.file_finder = None;
                                self.open_file(path);
                            }
                        } else if let Err(err) =
                            self.buffers[self.current_buffer_id].handle_input(input)
                        {
                            self.palette.set_msg(err.to_string());
                        }
                    }
                }
            }
        }
    }

    pub fn handle_app_event(
        &mut self,
        _proxy: &TuiEventLoopProxy,
        event: TuiAppEvent,
        control_flow: &mut TuiEventLoopControlFlow,
    ) {
        match event {
            event_loop::TuiAppEvent::PaletteEvent { mode, content } => match mode.as_str() {
                "command" => {
                    self.palette.reset();
                    self.palette_focus = false;
                    match cmd_parser::parse_cmd(&content) {
                        Ok(cmd) => match cmd {
                            cmd::Command::OpenFile(path) => self.open_file(path),
                            cmd::Command::SaveFile(path) => {
                                if let Err(err) = self.buffers[self.current_buffer_id].save(path) {
                                    self.palette.set_msg(err.to_string())
                                }
                            }
                            cmd::Command::Indent => {
                                match self.buffers[self.current_buffer_id].indent {
                                    Indentation::Tabs(amount) => {
                                        self.palette.set_msg(format!("{} tabs(s)", amount))
                                    }
                                    Indentation::Spaces(amount) => {
                                        self.palette.set_msg(format!("{} space(s)", amount))
                                    }
                                }
                            }
                            cmd::Command::Reload => {
                                if let Err(err) = self.buffers[self.current_buffer_id].reload() {
                                    self.palette.set_msg(err.to_string())
                                };
                            }
                            cmd::Command::Goto(line) => {
                                self.buffers[self.current_buffer_id].goto(line);
                            }
                            cmd::Command::ForceQuit => {
                                *control_flow = TuiEventLoopControlFlow::Exit;
                            }
                            cmd::Command::Logger => todo!(),
                        },
                        Err(err) => self.palette.set_msg(err.to_string()),
                    }
                }
                "goto" => {
                    self.palette.reset();
                    self.palette_focus = false;
                    if let Ok(line) = content.trim().parse::<i64>() {
                        self.buffers[self.current_buffer_id].goto(line);
                    }
                }
                _ => (),
            },
        }
    }

    pub fn open_file(&mut self, path: impl AsRef<Path>) {
        let real_path = match fs::canonicalize(&path) {
            Ok(path) => path,
            Err(err) => {
                self.palette.set_msg(err.to_string());
                return;
            }
        };

        match self.buffers.iter().find(|(_, buffer)| {
            buffer
                .file()
                .map(|path| fs::canonicalize(path).unwrap())
                .as_deref()
                == Some(&real_path)
        }) {
            Some((id, _)) => self.current_buffer_id = id,
            None => match Buffer::from_file(path) {
                Ok(buffer) => {
                    self.current_buffer_id = self.buffers.insert(buffer);
                }
                Err(err) => self.palette.set_msg(err.to_string()),
            },
        }
    }
}
