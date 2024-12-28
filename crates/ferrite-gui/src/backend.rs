use std::mem;

use ferrite_core::{config::editor::FontWeight, theme::EditorTheme};
use glyphon::{
    cosmic_text::Scroll, Attrs, AttrsList, Buffer, BufferLine, Cache, Color, Family, FontSystem,
    Metrics, Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer,
    Viewport, Weight,
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

const LINE_SCALE: f32 = 1.2;
const FONT_SIZE: f32 = 15.0;

fn calculate_cell_size(
    font_system: &mut FontSystem,
    metrics: Metrics,
    font_weight: FontWeight,
) -> (f32, f32) {
    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_wrap(font_system, glyphon::Wrap::None);

    // Use size of space to determine cell size
    buffer.set_text(
        font_system,
        " ",
        Attrs::new()
            .weight(Weight(font_weight as u16))
            .family(Family::Monospace),
        Shaping::Basic,
    );
    let layout = buffer.line_layout(font_system, 0).unwrap();
    let w = layout[0].w;
    (w, metrics.line_height)
}

pub struct WgpuBackend {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    bottom_quad_renderer: QuadRenderer,
    top_quad_renderer: QuadRenderer,
    width: f32,
    height: f32,
    pub cell_width: f32,
    pub cell_height: f32,
    pub columns: u16,
    pub lines: u16,
    pub redraw: bool,
    buffer: Buffer,
    cells: Vec<Vec<Cell>>,
    scale: f32,
    // font config
    font_family: String,
    font_weight: FontWeight,
}

impl WgpuBackend {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        width: f32,
        height: f32,
        font_family: String,
        font_weight: FontWeight,
    ) -> Self {
        let mut font_system = FontSystem::new();
        font_system
            .db_mut()
            .load_font_data(include_bytes!("../../../fonts/FiraCode-Regular.ttf").to_vec());
        font_system.db_mut().set_monospace_family(&font_family);
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

        let metrics = Metrics::relative(FONT_SIZE, LINE_SCALE);
        let mut buffer = Buffer::new(&mut font_system, metrics);
        // borrowed from cosmic term
        let (cell_width, cell_height) = calculate_cell_size(&mut font_system, metrics, font_weight);
        buffer.set_monospace_width(&mut font_system, Some(cell_width));
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

        let bottom_quad_renderer = QuadRenderer::new(device, config);
        let top_quad_renderer = QuadRenderer::new(device, config);

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            bottom_quad_renderer,
            top_quad_renderer,
            width,
            height,
            cell_width,
            cell_height,
            columns,
            lines,
            buffer,
            cells,
            redraw: true,
            scale: 1.0,
            font_family,
            font_weight,
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.bottom_quad_renderer.resize(width, height);
        self.top_quad_renderer.resize(width, height);
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
        self.bottom_quad_renderer.clear();
        self.top_quad_renderer.clear();
        let mut text_areas = Vec::new();
        self.buffer
            .set_size(&mut self.font_system, Some(self.width), Some(self.height));

        let default_fg = convert_style(&theme.text)
            .0
            .unwrap_or(glyphon::Color::rgb(0, 0, 0));
        let default_bg = convert_style(&theme.background)
            .1
            .unwrap_or(glyphon::Color::rgb(255, 255, 255));

        let default_attrs = Attrs::new()
            .weight(Weight(self.font_weight as u16))
            .color(default_fg)
            .family(Family::Monospace);
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
                let symbol_width = symbol.width();
                line_text.push_str(symbol);
                attr_list.add_span(idx..(idx + symbol.len()), attrs);
                idx += symbol.len();
                // TODO greedy mesh here
                self.bottom_quad_renderer.push_quad(
                    Quad {
                        x: col_idx as f32 * self.cell_width,
                        y: line_idx as f32 * self.cell_height,
                        width: self.cell_width * symbol_width as f32,
                        height: self.cell_height * symbol_width as f32,
                    },
                    bg,
                );

                if cell.modifier.contains(tui::style::Modifier::SLOW_BLINK) {
                    let cursor_width = 2.0 * self.scale;
                    self.top_quad_renderer.push_quad(
                        Quad {
                            x: col_idx as f32 * self.cell_width,
                            y: line_idx as f32 * self.cell_height,
                            width: cursor_width,
                            height: self.cell_height,
                        },
                        Color::rgb(82, 139, 255),
                    );
                }
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
        self.buffer.shape_until_scroll(&mut self.font_system, true);
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

        self.bottom_quad_renderer.prepare(device, queue);
        self.top_quad_renderer.prepare(device, queue);
    }

    pub fn render<'rpass>(&'rpass mut self, rpass: &mut RenderPass<'rpass>) {
        self.bottom_quad_renderer.render(rpass);
        self.text_renderer
            .render(&self.atlas, &self.viewport, rpass)
            .unwrap();
        self.top_quad_renderer.render(rpass);
    }

    pub fn set_font_family(&mut self, font_family: &str) {
        if font_family != self.font_family {
            self.font_system.db_mut().set_monospace_family(font_family);
            self.font_family = font_family.to_string();
            self.font_system.shape_run_cache.trim(0);
            self.update_font_metadata();
        }
    }

    pub fn set_font_weight(&mut self, weight: FontWeight) {
        if self.font_weight != weight {
            self.font_weight = weight;
            self.update_font_metadata();
        }
    }

    pub fn set_scale(&mut self, scale: f32) {
        if self.scale != scale {
            self.scale = scale;
            self.update_font_metadata();
        }
    }

    fn update_font_metadata(&mut self) {
        let metrics = Metrics::relative(FONT_SIZE * self.scale, LINE_SCALE);
        self.buffer.set_metrics(&mut self.font_system, metrics);
        let (cell_width, cell_height) =
            calculate_cell_size(&mut self.font_system, metrics, self.font_weight);
        self.cell_width = cell_width;
        self.cell_height = cell_height;
        self.resize(self.width, self.height);
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn line_height(&self) -> f32 {
        self.cell_height
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
                if let Some(next_cell) = &mut line.get_mut(column as usize + 1) {
                    next_cell.reset();
                    next_cell.set_symbol("");
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
