use std::{sync::mpsc, time::Instant};

use anyhow::Result;
use ferrite_cli::Args;
use ferrite_core::{
    buffer::ViewId,
    engine::Engine,
    event_loop_proxy::EventLoopProxy,
    file_explorer::FileExplorerId,
    layout::panes::PaneKind,
    logger::{self, LogMessage},
    picker::{buffer_picker::BufferItem, global_search_picker::GlobalSearchMatch},
    workspace::BufferId,
};
use glue::{convert_style, ferrite_to_tui_rect, tui_to_ferrite_rect};
use tui::{
    layout::{Margin, Rect},
    widgets::{StatefulWidget, Widget},
};
use widgets::{
    background_widget::BackgroundWidget, chord_widget::ChordWidget, editor_widget::EditorWidget,
    file_explorer_widget::FileExplorerWidget, logger_widget::LoggerWidget,
    palette_widget::CmdPaletteWidget, picker_widget::PickerWidget, splash::SplashWidget,
};

#[rustfmt::skip]
pub mod glue;
pub mod rect_ext;
pub mod widgets;

pub struct TuiApp {
    pub engine: Engine,
    pub keyboard_enhancement: bool,
}

#[profiling::all_functions]
impl TuiApp {
    pub fn new<P: EventLoopProxy + 'static>(
        args: &Args,
        proxy: P,
        recv: mpsc::Receiver<LogMessage>,
        width: u16,
        height: u16,
    ) -> Result<Self> {
        let mut engine = Engine::new(args, Box::new(proxy), recv)?;

        let editor_size = tui::layout::Rect::new(
            0,
            0,
            width,
            height.saturating_sub(engine.palette.height() as u16),
        );
        engine.buffer_area = tui_to_ferrite_rect(editor_size);

        logger::set_proxy(engine.proxy.dup());

        Ok(Self {
            engine,
            keyboard_enhancement: false,
        })
    }

    pub fn start_of_events(&mut self) {
        self.engine.start_of_events = Instant::now();
        #[cfg(feature = "talloc")]
        ferrite_talloc::Talloc::reset_phase_allocations();
        profiling::finish_frame!();
        ferrite_ctx::Ctx::arena().reset();
    }

    pub fn draw_pane_borders(&mut self, buf: &mut tui::buffer::Buffer, size: Rect) {
        let theme = &self.engine.themes[&self.engine.config.editor.theme];
        for x in size.x..(size.x + size.width) {
            for y in size.y..(size.y + size.height) {
                let cell = buf.cell_mut((x, y)).unwrap();
                cell.set_symbol("│");
                cell.set_style(convert_style(&theme.pane_border));
            }
        }
    }

    pub fn draw_buffer(
        &mut self,
        buf: &mut tui::buffer::Buffer,
        area: Rect,
        buffer_id: BufferId,
        view_id: ViewId,
    ) {
        profiling::scope!("render tui editor");

        let mut splash = false;
        if self.engine.config.editor.show_splash && self.engine.workspace.panes.num_panes() == 1 {
            let buffer = &mut self.engine.workspace.buffers[buffer_id];
            if buffer.len_bytes() == 0
                && !buffer.is_dirty()
                && buffer.file().is_none()
                && self.engine.workspace.buffers.len() == 1
            {
                splash = true;
            }
        }

        let current_pane = self.engine.workspace.panes.get_current_pane();
        let theme = &self.engine.themes[&self.engine.config.editor.theme];
        let mut widget = EditorWidget::new(
            theme,
            &self.engine.config.editor,
            view_id,
            !self.engine.palette.has_focus()
                && self.engine.file_picker.is_none()
                && self.engine.buffer_picker.is_none()
                && current_pane == PaneKind::Buffer(buffer_id, view_id),
            self.engine.branch_watcher.current_branch(),
            self.engine.spinner.current(),
        );
        widget.draw_rulers = !splash;
        widget.render(area, buf, &mut self.engine.workspace.buffers[buffer_id]);

        if splash {
            SplashWidget::new(theme).render(area, buf);
        }
    }

    pub fn draw_file_explorer(
        &mut self,
        buf: &mut tui::buffer::Buffer,
        area: Rect,
        file_explorer_id: FileExplorerId,
    ) {
        profiling::scope!("render tui file explorer");
        let current_pane = self.engine.workspace.panes.get_current_pane();
        let has_focus = !self.engine.palette.has_focus()
            && self.engine.file_picker.is_none()
            && self.engine.buffer_picker.is_none()
            && current_pane == PaneKind::FileExplorer(file_explorer_id);
        FileExplorerWidget::new(
            &self.engine.themes[&self.engine.config.editor.theme],
            &self.engine.config.editor,
            has_focus,
        )
        .render(
            area,
            buf,
            &mut self.engine.workspace.file_explorers[file_explorer_id],
        );
    }

    pub fn draw_logger(&mut self, buf: &mut tui::buffer::Buffer, area: Rect) {
        profiling::scope!("render tui logger");
        let current_pane = self.engine.workspace.panes.get_current_pane();
        let has_focus = !self.engine.palette.has_focus()
            && self.engine.file_picker.is_none()
            && self.engine.buffer_picker.is_none()
            && current_pane == PaneKind::Logger;
        LoggerWidget::new(
            &self.engine.themes[&self.engine.config.editor.theme],
            self.engine.last_render_time,
            has_focus,
        )
        .render(area, buf, &mut self.engine.logger_state);
    }

    pub fn draw_overlays(&mut self, buf: &mut tui::buffer::Buffer, size: Rect) {
        let picker_margin = Margin {
            horizontal: 5,
            vertical: 2,
        };
        if let Some(file_picker) = &mut self.engine.file_picker {
            profiling::scope!("render tui file picker");
            let size = size.inner(picker_margin);
            PickerWidget::new(
                &self.engine.themes[&self.engine.config.editor.theme],
                &self.engine.config.editor,
                "Open file",
            )
            .render(size, buf, file_picker);
        }

        if let Some(buffer_picker) = &mut self.engine.buffer_picker {
            profiling::scope!("render tui buffer picker");
            let size = size.inner(picker_margin);
            PickerWidget::<BufferItem>::new(
                &self.engine.themes[&self.engine.config.editor.theme],
                &self.engine.config.editor,
                "Open buffer",
            )
            .render(size, buf, buffer_picker);
        }

        if let Some(global_search_picker) = &mut self.engine.global_search_picker {
            profiling::scope!("render tui search picker");
            let size = size.inner(picker_margin);
            PickerWidget::<GlobalSearchMatch>::new(
                &self.engine.themes[&self.engine.config.editor.theme],
                &self.engine.config.editor,
                "Matches",
            )
            .set_text_align(widgets::picker_widget::TextAlign::Left)
            .render(size, buf, global_search_picker);
        }

        let palette_size = Rect::new(
            size.left(),
            size.bottom()
                .saturating_sub(self.engine.palette.height() as u16),
            size.width,
            (self.engine.palette.height() as u16).min(size.height),
        );
        CmdPaletteWidget::new(
            &self.engine.themes[&self.engine.config.editor.theme],
            &self.engine.config.editor,
            self.engine.palette.has_focus(),
            size,
        )
        .render(palette_size, buf, &mut self.engine.palette);

        if self.engine.chord.is_some() {
            ChordWidget::new(
                &self.engine.themes[&self.engine.config.editor.theme],
                self.engine.get_current_keymappings(),
            )
            .render(size, buf);
        }
    }

    pub fn render(&mut self, buf: &mut tui::buffer::Buffer, size: Rect) {
        BackgroundWidget::new(&self.engine.themes[&self.engine.config.editor.theme])
            .render(size, buf);
        let editor_size = Rect::new(
            size.x,
            size.y,
            size.width,
            size.height
                .saturating_sub(self.engine.palette.height() as u16),
        );
        self.draw_pane_borders(buf, editor_size);

        self.engine.buffer_area = tui_to_ferrite_rect(editor_size);
        for (pane, pane_rect) in self
            .engine
            .workspace
            .panes
            .get_pane_bounds(tui_to_ferrite_rect(editor_size))
        {
            match pane {
                PaneKind::Buffer(buffer_id, view_id) => {
                    self.draw_buffer(buf, ferrite_to_tui_rect(pane_rect), buffer_id, view_id);
                }
                PaneKind::FileExplorer(file_explorer_id) => {
                    self.draw_file_explorer(buf, ferrite_to_tui_rect(pane_rect), file_explorer_id);
                }
                PaneKind::Logger => {
                    self.draw_logger(buf, ferrite_to_tui_rect(pane_rect));
                }
            }
        }

        self.draw_overlays(buf, size);
    }
}
