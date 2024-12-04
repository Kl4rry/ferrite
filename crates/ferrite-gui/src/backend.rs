use std::mem;

use ferrite_core::theme::EditorTheme;
use glyphon::{
    cosmic_text::Scroll, Attrs, AttrsList, Buffer, BufferLine, Cache, Family, FontSystem, Metrics,
    Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use quad_renderer::{Quad, QuadRenderer};
use tui::{
    backend::WindowSize,
    buffer::Cell,
    layout::{Position, Size},
    prelude::Backend,
};
use unicode_width::UnicodeWidthStr;
use wgpu::RenderPass;

use crate::glue::convert_style;

mod quad_renderer;

pub struct WgpuBackend {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    quad_renderer: QuadRenderer,
    width: f32,
    height: f32,
    cell_width: f32,
    cell_height: f32,
    pub columns: u16,
    pub lines: u16,
    pub redraw: bool,
    buffer: Buffer,
    cells: Vec<Vec<Cell>>,
}

impl WgpuBackend {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        width: f32,
        height: f32,
    ) -> Self {
        let mut font_system = FontSystem::new();
        font_system.db_mut().set_monospace_family("Fira Code");
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, config.format);
        let text_renderer = TextRenderer::new(
            &mut atlas,
            device,
            wgpu::MultisampleState {
                count: 1,
                ..Default::default()
            },
            None,
        );

        let viewport = Viewport::new(device, &cache);

        let metrics = Metrics::relative(15.0, 1.20);
        let mut buffer = Buffer::new(&mut font_system, metrics);
        // borrowed from cosmic term
        let (cell_width, cell_height) = {
            buffer.set_wrap(&mut font_system, glyphon::Wrap::None);

            // Use size of space to determine cell size
            buffer.set_text(
                &mut font_system,
                " ",
                Attrs::new().family(Family::Monospace),
                Shaping::Basic,
            );
            let layout = buffer.line_layout(&mut font_system, 0).unwrap();
            let w = layout[0].w;
            buffer.set_monospace_width(&mut font_system, Some(w));
            (w, metrics.line_height)
        };
        buffer.set_wrap(&mut font_system, glyphon::Wrap::None);

        let columns = (width / cell_width) as u16;
        let lines = (height / cell_height) as u16;

        let mut cells = Vec::new();
        for _ in 0..lines {
            let mut line = Vec::with_capacity(columns.into());
            for _ in 0..columns {
                line.push(Cell::default());
            }
            cells.push(line);
        }

        let quad_renderer = QuadRenderer::new(device, config);

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            quad_renderer,
            width,
            height,
            cell_width,
            cell_height,
            columns,
            lines,
            buffer,
            cells,
            redraw: true,
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.quad_renderer.resize(width, height);
        self.width = width;
        self.height = height;
        self.columns = (width / self.cell_width) as u16;
        self.lines = (height / self.cell_height) as u16;
        self.cells.clear();
        for _ in 0..self.lines {
            let mut line = Vec::with_capacity(self.columns.into());
            for _ in 0..self.columns {
                line.push(Cell::default());
            }
            self.cells.push(line);
        }
        let _ = self.clear();
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, theme: &EditorTheme) {
        self.quad_renderer.clear();
        let mut text_areas = Vec::new();
        self.buffer
            .set_size(&mut self.font_system, Some(self.width), Some(self.height));

        let default_fg = convert_style(&theme.text)
            .0
            .unwrap_or(glyphon::Color::rgb(0, 0, 0));
        let default_bg = convert_style(&theme.background)
            .1
            .unwrap_or(glyphon::Color::rgb(255, 255, 255));

        let default_attrs = Attrs::new().color(default_fg).family(Family::Monospace);
        self.buffer.lines.resize(
            self.cells.len(),
            BufferLine::new(
                "",
                glyphon::cosmic_text::LineEnding::Lf,
                AttrsList::new(default_attrs),
                Shaping::Basic,
            ),
        );
        for (line_idx, line) in self.cells.iter_mut().enumerate() {
            let mut attr_list = AttrsList::new(default_attrs);
            let mut line_text = String::new();
            let mut idx = 0;
            // TODO handle cells that are wider then 1
            for (col_idx, cell) in line.iter().enumerate() {
                let mut attrs = default_attrs;
                let mut fg = default_fg;
                let mut bg = default_bg;
                if let tui::style::Color::Rgb(r, g, b) = cell.fg {
                    fg = glyphon::Color::rgb(r, g, b);
                }

                if let tui::style::Color::Rgb(r, g, b) = cell.bg {
                    bg = glyphon::Color::rgb(r, g, b);
                }

                if cell.modifier.contains(tui::style::Modifier::REVERSED) {
                    mem::swap(&mut fg, &mut bg);
                }

                attrs = attrs.color(fg);
                let symbol = cell.symbol();
                line_text.push_str(symbol);
                attr_list.add_span(idx..(idx + symbol.len()), attrs);
                idx += symbol.len();
                //if bg != default_bg {
                self.quad_renderer.push_quad(
                    Quad {
                        x: col_idx as f32 * self.cell_width,
                        y: line_idx as f32 * self.cell_height,
                        width: self.cell_width,
                        height: self.cell_height,
                    },
                    bg,
                );
                //}
            }

            self.buffer.lines[line_idx] = BufferLine::new(
                &line_text,
                glyphon::cosmic_text::LineEnding::Lf,
                attr_list,
                Shaping::Advanced,
            );
        }

        self.buffer.set_scroll(Scroll {
            line: 0,
            vertical: 0.0,
            horizontal: 0.0,
        });
        self.buffer.shape_until_scroll(&mut self.font_system, false);
        self.font_system.shape_run_cache.trim(1024);

        self.viewport.update(
            queue,
            Resolution {
                width: self.width as u32,
                height: self.height as u32,
            },
        );

        text_areas.push(TextArea {
            buffer: &self.buffer,
            left: 0.0,
            top: 0.0,
            scale: 1.0,
            bounds: TextBounds {
                left: 0,
                top: 0,
                right: self.width as i32,
                bottom: self.height as i32,
            },
            default_color: default_fg,
            custom_glyphs: &[],
        });

        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();

        self.quad_renderer.prepare(device, queue);
    }

    pub fn render<'rpass>(&'rpass mut self, rpass: &mut RenderPass<'rpass>) {
        self.quad_renderer.render(rpass);
        self.text_renderer
            .render(&self.atlas, &self.viewport, rpass)
            .unwrap();
    }
}

impl Backend for WgpuBackend {
    fn draw<'a, I>(&mut self, content: I) -> std::io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a tui::buffer::Cell)>,
    {
        for (column, line, cell) in content {
            let line = &mut self.cells[line as usize];
            line[column as usize] = cell.clone();
            let cell_width = cell.symbol().width();
            if cell_width > 1 {
                if let Some(next_cell) = &mut line.get_mut(column as usize) {
                    next_cell.set_symbol("");
                }
            } else if cell_width == 1 {
                if let Some(next_cell) = &mut line.get_mut(column as usize + 1) {
                    if next_cell.symbol() == "" {
                        next_cell.set_char(' ');
                    }
                }
            }
            self.redraw = true;
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn show_cursor(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn clear(&mut self) -> std::io::Result<()> {
        self.buffer.lines.clear();
        for line in &mut self.cells {
            for cell in line {
                cell.reset();
            }
        }
        Ok(())
    }

    fn window_size(&mut self) -> std::io::Result<tui::backend::WindowSize> {
        Ok(WindowSize {
            columns_rows: Size::new(self.columns, self.lines),
            pixels: Size::new(self.width as u16, self.height as u16),
        })
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn get_cursor_position(&mut self) -> std::io::Result<Position> {
        Ok(Position::new(0, 0))
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, _position: P) -> std::io::Result<()> {
        Ok(())
    }

    fn size(&self) -> std::io::Result<tui::prelude::Size> {
        Ok(Size::new(self.columns, self.lines))
    }
}
