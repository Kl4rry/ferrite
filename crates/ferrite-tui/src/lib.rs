use std::{
    io::{self, IsTerminal},
    sync::mpsc,
    time::Instant,
};

use anyhow::{bail, Result};
use ferrite_cli::Args;
use ferrite_core::{
    engine::Engine,
    event_loop_proxy::EventLoopProxy,
    layout::panes::PaneKind,
    logger::{self, LogMessage},
    picker::{buffer_picker::BufferItem, global_search_picker::GlobalSearchMatch},
};
use ferrite_utility::point::Point;
use glue::{convert_style, ferrite_to_tui_rect, tui_to_ferrite_rect};
use tui::{
    layout::{Margin, Rect, Size},
    prelude::Backend,
};
use widgets::{
    choord_widget::ChoordWidget, file_explorer_widget::FileExplorerWidget,
    logger_widget::LoggerWidget,
};

use self::widgets::{
    background_widget::BackgroundWidget, editor_widget::EditorWidget,
    palette_widget::CmdPaletteWidget, picker_widget::PickerWidget, splash::SplashWidget,
};

#[rustfmt::skip]
pub mod glue;
pub mod rect_ext;
pub mod widgets;

pub struct TuiApp<B: Backend> {
    pub terminal: tui::Terminal<B>,
    pub buffer_area: Rect,
    pub drag_start: Option<Point<usize>>,
    pub engine: Engine,
    pub keyboard_enhancement: bool,
}

impl<B> TuiApp<B>
where
    B: Backend,
{
    pub fn new<P: EventLoopProxy + 'static>(
        args: &Args,
        proxy: P,
        backend: B,
        recv: mpsc::Receiver<LogMessage>,
    ) -> Result<Self> {
        let engine = Engine::new(args, Box::new(proxy), recv)?;

        logger::set_proxy(engine.proxy.dup());

        let Size { width, height } = backend.size()?;

        if !io::stdout().is_terminal() {
            bail!("Stdout must be a terminal");
        }

        Ok(Self {
            terminal: tui::Terminal::new(backend)?,
            buffer_area: Rect {
                x: 0,
                y: 0,
                width,
                height: height.saturating_sub(2),
            },
            drag_start: None,
            engine,
            keyboard_enhancement: false,
        })
    }

    pub fn start_of_events(&mut self) {
        self.engine.start_of_events = Instant::now();
        #[cfg(feature = "talloc")]
        ferrite_talloc::Talloc::reset_phase_allocations();
    }

    pub fn render(&mut self) {
        if self.engine.force_redraw {
            self.engine.force_redraw = false;
            let _ = self.terminal.clear();
        }
        self.terminal
            .draw(|f| {
                let theme = &self.engine.themes[&self.engine.config.editor.theme];
                f.render_widget(BackgroundWidget::new(theme), f.area());
                let size = f.area();
                let editor_size = Rect::new(
                    size.x,
                    size.y,
                    size.width,
                    size.height
                        .saturating_sub(self.engine.palette.height() as u16),
                );

                for x in editor_size.x..(editor_size.x + editor_size.width) {
                    for y in editor_size.y..(editor_size.y + editor_size.height) {
                        let cell = f.buffer_mut().cell_mut((x, y)).unwrap();
                        cell.set_symbol("│");
                        cell.set_style(convert_style(&theme.pane_border));
                    }
                }

                self.buffer_area = editor_size;
                let current_pane = self.engine.workspace.panes.get_current_pane();
                for (pane, pane_rect) in self
                    .engine
                    .workspace
                    .panes
                    .get_pane_bounds(tui_to_ferrite_rect(editor_size))
                {
                    match pane {
                        PaneKind::Buffer(buffer_id, view_id) => {
                            f.render_stateful_widget(
                                EditorWidget::new(
                                    theme,
                                    &self.engine.config.editor,
                                    view_id,
                                    !self.engine.palette.has_focus()
                                        && self.engine.file_picker.is_none()
                                        && self.engine.buffer_picker.is_none()
                                        && current_pane == pane,
                                    self.engine.branch_watcher.current_branch(),
                                    self.engine.spinner.current(),
                                ),
                                ferrite_to_tui_rect(pane_rect),
                                &mut self.engine.workspace.buffers[buffer_id],
                            );

                            if self.engine.config.editor.show_splash
                                && self.engine.workspace.panes.num_panes() == 1
                            {
                                let buffer = &mut self.engine.workspace.buffers[buffer_id];
                                if buffer.len_bytes() == 0
                                    && !buffer.is_dirty()
                                    && buffer.file().is_none()
                                    && self.engine.workspace.buffers.len() == 1
                                {
                                    f.render_widget(
                                        SplashWidget::new(theme),
                                        ferrite_to_tui_rect(pane_rect),
                                    );
                                }
                            }
                        }
                        PaneKind::FileExplorer(file_explorer_id) => {
                            let has_focus = !self.engine.palette.has_focus()
                                && self.engine.file_picker.is_none()
                                && self.engine.buffer_picker.is_none()
                                && current_pane == pane;
                            f.render_stateful_widget(
                                FileExplorerWidget::new(theme, has_focus),
                                ferrite_to_tui_rect(pane_rect),
                                &mut self.engine.workspace.file_explorers[file_explorer_id],
                            );
                        }
                        PaneKind::Logger => {
                            let has_focus = !self.engine.palette.has_focus()
                                && self.engine.file_picker.is_none()
                                && self.engine.buffer_picker.is_none()
                                && current_pane == pane;
                            f.render_stateful_widget(
                                LoggerWidget::new(theme, self.engine.last_render_time, has_focus),
                                ferrite_to_tui_rect(pane_rect),
                                &mut self.engine.logger_state,
                            );
                        }
                    }
                }

                if let Some(file_picker) = &mut self.engine.file_picker {
                    let size = size.inner(Margin {
                        horizontal: 5,
                        vertical: 2,
                    });
                    f.render_stateful_widget(
                        PickerWidget::new(theme, &self.engine.config.editor, "Open file"),
                        size,
                        file_picker,
                    );
                }

                if let Some(buffer_picker) = &mut self.engine.buffer_picker {
                    let size = size.inner(Margin {
                        horizontal: 5,
                        vertical: 2,
                    });
                    f.render_stateful_widget(
                        PickerWidget::<BufferItem>::new(
                            theme,
                            &self.engine.config.editor,
                            "Open buffer",
                        ),
                        size,
                        buffer_picker,
                    );
                }

                if let Some(global_search_picker) = &mut self.engine.global_search_picker {
                    let size = size.inner(Margin {
                        horizontal: 5,
                        vertical: 2,
                    });
                    f.render_stateful_widget(
                        PickerWidget::<GlobalSearchMatch>::new(
                            theme,
                            &self.engine.config.editor,
                            "Matches",
                        )
                        .set_text_align(widgets::picker_widget::TextAlign::Left),
                        size,
                        global_search_picker,
                    );
                }

                let palette_size = Rect::new(
                    size.left(),
                    size.bottom()
                        .saturating_sub(self.engine.palette.height() as u16),
                    size.width,
                    (self.engine.palette.height() as u16).min(size.height),
                );
                f.render_stateful_widget(
                    CmdPaletteWidget::new(theme, self.engine.palette.has_focus(), size),
                    palette_size,
                    &mut self.engine.palette,
                );

                if self.engine.choord.is_some() {
                    let choord_widget =
                        ChoordWidget::new(theme, self.engine.get_current_keymappings());
                    f.render_widget(choord_widget, size);
                }
            })
            .unwrap();
    }
}
