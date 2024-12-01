use std::time::Instant;

use ferrite_core::theme::EditorTheme;
use glyphon::{
    Attrs, Buffer, Cache, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
};
use tui::{
    backend::WindowSize,
    buffer::Cell,
    layout::{Position, Size},
    prelude::Backend,
};
use wgpu::RenderPass;

use crate::glue::convert_style;

pub struct WgpuBackend {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    width: f32,
    height: f32,
    cell_width: f32,
    cell_height: f32,
    columns: u16,
    lines: u16,
    // buffer
    buffer: Buffer,
    cells: Vec<Vec<Cell>>,
}

impl WgpuBackend {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        width: f32,
        height: f32,
    ) -> Self {
        let mut font_system = FontSystem::new();
        font_system.db_mut().set_monospace_family("Fira Code");
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, surface_format);
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
                Shaping::Advanced,
            );
            let layout = buffer.line_layout(&mut font_system, 0).unwrap();
            let w = layout[0].w;
            buffer.set_monospace_width(&mut font_system, Some(w));
            (w, metrics.line_height)
        };
        buffer.set_wrap(&mut font_system, glyphon::Wrap::None);

        let columns = (width / cell_width) as u16;
        let lines = (height / cell_height) as u16;

        let mut cells: Vec<Vec<Cell>> = Vec::new();
        for _ in 0..lines {
            let mut line = Vec::with_capacity(columns.into());
            for _ in 0..columns {
                line.push(Cell::default());
            }
            cells.push(line);
        }

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            width,
            height,
            cell_width,
            cell_height,
            columns,
            lines,
            buffer,
            cells,
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
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
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, theme: &EditorTheme) {
        let mut text_areas = Vec::new();
        let start = Instant::now();
        self.buffer
            .set_size(&mut self.font_system, Some(self.width), Some(self.height));

        self.buffer.lines.clear();
        let fg = convert_style(&theme.text)
            .0
            .unwrap_or(glyphon::Color::rgb(0, 0, 0));
        /*et bg = convert_style(&theme.background)
        .1
        .unwrap_or(glyphon::Color::rgb(1, 1, 1));*/
        let default_attrs = Attrs::new().color(fg).family(Family::Monospace);
        let mut spans = Vec::new();
        for line in &self.cells {
            for cell in line {
                let mut attrs = default_attrs;
                if let tui::style::Color::Rgb(r, g, b) = cell.fg {
                    let color = glyphon::Color::rgb(r, g, b);
                    attrs = attrs.color(color);
                }
                spans.push((cell.symbol(), attrs));
            }
            spans.push(("\n", default_attrs));
        }
        eprintln!("text: {:?}", Instant::now().duration_since(start));

        let start = Instant::now();
        self.buffer.set_rich_text(
            &mut self.font_system,
            spans,
            default_attrs,
            Shaping::Advanced,
        );
        eprintln!("set: {:?}", Instant::now().duration_since(start));

        let start = Instant::now();
        self.buffer.shape_until_scroll(&mut self.font_system, true);
        eprintln!("shape: {:?}", Instant::now().duration_since(start));

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
            default_color: glyphon::Color::rgb(205, 214, 244),
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
    }

    pub fn render<'rpass>(&'rpass mut self, rpass: &mut RenderPass<'rpass>) {
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
            self.cells[line as usize][column as usize] = cell.clone();
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
        /*eprintln!("clear");
        self.buffer.lines.clear();
        for line in &mut self.cells {
            for cell in line {
                cell.reset();
            }
        }*/
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
