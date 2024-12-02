use std::time::Instant;

use ferrite_core::theme::EditorTheme;
use glyphon::{
    cosmic_text::Scroll, Attrs, AttrsList, Buffer, BufferLine, Cache, Family, FontSystem, Metrics,
    Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
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
    pub columns: u16,
    pub lines: u16,
    // buffer
    buffer: Buffer,
    cells: Vec<(Vec<Cell>, bool)>,
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
            cells.push((line, true));
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
            self.cells.push((line, true));
        }
        let _ = self.clear();
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, theme: &EditorTheme) {
        let mut text_areas = Vec::new();
        let start = Instant::now();
        self.buffer
            .set_size(&mut self.font_system, Some(self.width), Some(self.height));

        let fg = convert_style(&theme.text)
            .0
            .unwrap_or(glyphon::Color::rgb(0, 0, 0));
        /*et bg = convert_style(&theme.background)
        .1
        .unwrap_or(glyphon::Color::rgb(1, 1, 1));*/

        let default_attrs = Attrs::new().color(fg).family(Family::Monospace);
        self.buffer.lines.resize(
            self.cells.len(),
            BufferLine::new(
                "",
                glyphon::cosmic_text::LineEnding::Lf,
                AttrsList::new(default_attrs),
                Shaping::Basic,
            ),
        );
        for (i, (line, dirty)) in self.cells.iter_mut().enumerate() {
            if !*dirty {
                continue;
            }
            let mut attr_list = AttrsList::new(default_attrs);
            attr_list.add_span(1..3, default_attrs.color(glyphon::Color::rgb(255, 0, 255)));
            attr_list.add_span(2..9, default_attrs.color(glyphon::Color::rgb(255, 0, 0)));
            eprintln!("{:#?}", attr_list.spans());
            let mut line_text = String::new();
            for cell in line {
                /*let mut attrs = default_attrs;
                if let tui::style::Color::Rgb(r, g, b) = cell.fg {
                    let color = glyphon::Color::rgb(r, g, b);
                }*/
                line_text.push_str(cell.symbol());
            }
            //self.buffer.set_rich_text(font_system, spans, default_attrs, shaping);

            self.buffer.lines[i].set_text(
                &line_text,
                glyphon::cosmic_text::LineEnding::Lf,
                attr_list,
            );
            *dirty = false;
        }
        eprintln!("text: {:?}", Instant::now().duration_since(start));

        let start = Instant::now();
        self.buffer.set_scroll(Scroll {
            line: 0,
            vertical: 0.0,
            horizontal: 0.0,
        });
        self.buffer.shape_until_scroll(&mut self.font_system, false);
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
            let (line, dirty) = &mut self.cells[line as usize];
            line[column as usize] = cell.clone();
            *dirty = true;
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
        for (line, dirty) in &mut self.cells {
            for cell in line {
                cell.reset();
            }
            *dirty = true;
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
