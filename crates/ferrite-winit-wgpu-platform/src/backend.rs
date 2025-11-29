use std::{borrow::Cow, mem};

use ferrite_geom::rect::{Rect, Vec2};
use ferrite_runtime::Bounds;
use ferrite_style::Color;
use glyphon::{
    Attrs, AttrsList, Buffer, BufferLine, Family, FontSystem, Metrics, Shaping, TextArea,
    TextBounds, Weight, cosmic_text::Scroll,
};
use tui::{
    backend::WindowSize,
    buffer::Cell,
    layout::{Position, Size},
    prelude::Backend,
};
use unicode_width::UnicodeWidthStr;

use crate::renderer::{
    Bundle,
    geometry_renderer::{Geometry, Quad},
};

const LINE_SCALE: f32 = 1.3;
const FONT_SIZE: f32 = 14.0;

pub fn calculate_char_size(
    font_system: &mut FontSystem,
    metrics: Metrics,
    font_weight: Weight,
    shaping: Shaping,
    text: &str,
) -> (f32, f32) {
    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_wrap(font_system, glyphon::Wrap::None);

    // Use size of space to determine cell size
    buffer.set_text(
        font_system,
        text,
        &Attrs::new().weight(font_weight).family(Family::Monospace),
        shaping,
    );
    let layout = buffer.line_layout(font_system, 0).unwrap();
    let w = layout[0].w;
    (w, metrics.line_height)
}

pub fn calculate_cell_size(
    font_system: &mut FontSystem,
    metrics: Metrics,
    font_weight: Weight,
) -> (f32, f32) {
    calculate_char_size(font_system, metrics, font_weight, Shaping::Basic, " ")
}

pub fn get_metrics(scale: f32) -> Metrics {
    Metrics::relative(FONT_SIZE * scale, LINE_SCALE)
}

pub struct WgpuBackend {
    bounds: Bounds,
    pub redraw: bool,
    reshape: bool,
    buffer: Buffer,
    top_geometry: Geometry,
    bottom_geometry: Geometry,
    cells: Vec<Vec<Cell>>,
    scale: f32,
    default_fg: glyphon::Color,
    default_bg: glyphon::Color,
    // font config
    font_family: String,
    font_weight: Weight,
    // gemoetry from Painter2d
    pub overlay_gemoetry: Vec<(Rect<f32>, Color)>,
}

#[profiling::all_functions]
impl WgpuBackend {
    pub fn new(
        font_system: &mut FontSystem,
        bounds: Bounds,
        font_family: String,
        font_weight: Weight,
    ) -> Self {
        font_system.db_mut().set_monospace_family(&font_family);
        let metrics = get_metrics(1.0);
        let mut buffer = Buffer::new(font_system, metrics);
        // borrowed from cosmic term
        let (cell_width, cell_height) = calculate_cell_size(font_system, metrics, font_weight);
        let bounds = Bounds::new(
            bounds.view_bounds(),
            Vec2::new(cell_width, cell_height),
            bounds.rounding,
        );
        buffer.set_monospace_width(font_system, Some(cell_width));
        buffer.set_wrap(font_system, glyphon::Wrap::None);

        let grid_bounds = bounds.grid_bounds();
        let columns = grid_bounds.width as u16;
        let lines = grid_bounds.height as u16;

        let mut cells = Vec::new();
        for _ in 0..lines {
            let mut line = Vec::with_capacity(columns.into());
            for _ in 0..columns {
                line.push(Cell::default());
            }
            cells.push(line);
        }

        Self {
            bounds,
            buffer,
            top_geometry: Default::default(),
            bottom_geometry: Default::default(),
            cells,
            redraw: true,
            reshape: true,
            scale: 1.0,
            default_fg: glyphon::Color::rgb(0, 0, 0),
            default_bg: glyphon::Color::rgb(255, 255, 255),
            font_family,
            font_weight,
            overlay_gemoetry: Vec::new(),
        }
    }

    pub fn resize(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        let grid_bounds = bounds.grid_bounds();
        let columns = grid_bounds.width as u16;
        let lines = grid_bounds.height as u16;
        self.cells.clear();
        for _ in 0..lines {
            let mut line = Vec::with_capacity(columns.into());
            for _ in 0..columns {
                line.push(Cell::default());
            }
            self.cells.push(line);
        }
        let _ = self.clear();
        self.reshape = true;
    }

    pub fn prepare(&mut self, font_system: &mut FontSystem) -> Bundle<'_> {
        let view_bounds = self.bounds.view_bounds();
        self.buffer.set_size(
            font_system,
            Some(view_bounds.width as f32),
            Some(view_bounds.height as f32),
        );
        let cell_width = self.bounds.cell_size().x;
        let cell_height = self.bounds.cell_size().y;
        let x = view_bounds.x as f32;
        let y = view_bounds.y as f32;

        let default_attrs = Attrs::new()
            .weight(self.font_weight)
            .color(self.default_fg)
            .family(Family::Monospace);
        self.buffer.lines.resize(
            self.cells.len(),
            BufferLine::new(
                "",
                glyphon::cosmic_text::LineEnding::Lf,
                AttrsList::new(&default_attrs),
                Shaping::Basic,
            ),
        );

        if self.reshape {
            self.top_geometry.clear();
            self.bottom_geometry.clear();
            profiling::scope!("update buffer");
            for (line_idx, line) in self.cells.iter_mut().enumerate() {
                let mut skip_next = false;
                let mut attr_list = AttrsList::new(&default_attrs.clone());
                let mut line_text = String::new();
                let mut idx = 0;
                for (col_idx, cell) in line.iter().enumerate() {
                    if skip_next {
                        skip_next = false;
                        continue;
                    }
                    let mut attrs = default_attrs.clone();
                    let mut fg = self.default_fg;
                    let mut bg = None;
                    if let tui::style::Color::Rgb(r, g, b) = cell.fg {
                        fg = glyphon::Color::rgb(r, g, b);
                    }

                    if let tui::style::Color::Rgb(r, g, b) = cell.bg {
                        bg = Some(glyphon::Color::rgb(r, g, b));
                    }

                    if cell.modifier.contains(tui::style::Modifier::REVERSED) {
                        let mut tmp = bg.unwrap_or(self.default_bg);
                        mem::swap(&mut fg, &mut tmp);
                        bg = Some(tmp);
                    }

                    attrs = attrs.color(fg);
                    let symbol = cell.symbol();

                    let symbol_width = symbol.width();
                    let mut cow_symbol: Cow<str> = symbol.into();
                    if symbol_width > 1 {
                        let (ch_w, _) = calculate_char_size(
                            font_system,
                            get_metrics(self.scale),
                            self.font_weight,
                            Shaping::Advanced,
                            symbol,
                        );
                        let width = (ch_w / cell_width).round() as usize;
                        if width < symbol_width {
                            let mut owned_symbol = symbol.to_string();
                            for _ in 0..(symbol_width - width) {
                                owned_symbol.push(' ');
                            }
                            cow_symbol = owned_symbol.into();
                        }
                        skip_next = true;
                    }

                    line_text.push_str(&cow_symbol);
                    attr_list.add_span(idx..(idx + cow_symbol.len()), &attrs);
                    idx += cow_symbol.len();

                    // TODO greedy mesh here
                    self.bottom_geometry.quads.push(Quad {
                        x: col_idx as f32 * cell_width + x,
                        y: line_idx as f32 * cell_height + y,
                        width: cell_width * symbol_width as f32,
                        height: cell_height,
                        color: bg.unwrap_or(glyphon::Color::rgba(0, 0, 0, 0)),
                    });

                    if cell.modifier.contains(tui::style::Modifier::SLOW_BLINK) {
                        let cursor_width = 2.0 * self.scale;
                        self.top_geometry.quads.push(Quad {
                            x: col_idx as f32 * cell_width + x,
                            y: line_idx as f32 * cell_height + y,
                            width: cursor_width,
                            height: cell_height,
                            color: glyphon::Color::rgb(82, 139, 255),
                        });
                    }

                    for (rect, color) in &self.overlay_gemoetry {
                        self.top_geometry.quads.push(Quad {
                            x: rect.x,
                            y: rect.y,
                            width: rect.width,
                            height: rect.height,
                            color: glyphon::Color::rgb(
                                (color.r * 255.0) as u8,
                                (color.g * 255.0) as u8,
                                (color.b * 255.0) as u8,
                            ),
                        });
                    }
                }

                self.buffer.lines[line_idx] = BufferLine::new(
                    &line_text,
                    glyphon::cosmic_text::LineEnding::Lf,
                    attr_list,
                    Shaping::Advanced,
                );
            }
        }

        self.buffer.set_scroll(Scroll {
            line: 0,
            vertical: 0.0,
            horizontal: 0.0,
        });
        {
            profiling::scope!("shape text");
            self.buffer.shape_until_scroll(font_system, true);
        }
        self.reshape = false;

        let text_area = TextArea {
            buffer: &self.buffer,
            left: x,
            top: y,
            scale: 1.0,
            bounds: TextBounds {
                left: x as i32,
                top: y as i32,
                right: view_bounds.width as i32 + x as i32,
                bottom: view_bounds.height as i32 + y as i32,
            },
            default_color: self.default_fg,
            custom_glyphs: &[],
        };

        Bundle {
            text_area: Some(text_area),
            top_geometry: &self.top_geometry,
            bottom_geometry: &self.bottom_geometry,
        }
    }

    pub fn set_font_family(&mut self, font_system: &mut FontSystem, font_family: &str) {
        if font_family != self.font_family {
            self.font_family = font_family.to_string();
            font_system.db_mut().set_monospace_family(font_family);
            font_system.shape_run_cache.trim(0);
            self.update_font_metadata(font_system);
        }
    }

    pub fn set_font_weight(&mut self, font_system: &mut FontSystem, weight: Weight) {
        if self.font_weight != weight {
            self.font_weight = weight;
            self.update_font_metadata(font_system);
        }
    }

    pub fn set_scale(&mut self, font_system: &mut FontSystem, scale: f32) {
        if self.scale != scale {
            self.scale = scale;
            self.update_font_metadata(font_system);
        }
    }

    fn update_font_metadata(&mut self, font_system: &mut FontSystem) {
        let metrics = get_metrics(self.scale);
        self.buffer.set_metrics(font_system, metrics);
        let (cell_width, cell_height) = calculate_cell_size(font_system, metrics, self.font_weight);
        self.buffer
            .set_monospace_width(font_system, Some(cell_width));
        self.bounds = Bounds::new(
            self.bounds.view_bounds(),
            Vec2::new(cell_width, cell_height),
            self.bounds.rounding,
        );
        self.resize(self.bounds);
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn line_height(&self) -> f32 {
        self.bounds.cell_size().y
    }

    pub fn set_default_fg(&mut self, r: u8, g: u8, b: u8) {
        self.default_fg = glyphon::Color::rgb(r, g, b);
    }

    pub fn set_default_bg(&mut self, r: u8, g: u8, b: u8) {
        self.default_bg = glyphon::Color::rgb(r, g, b);
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds
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
            if cell_width > 1
                && let Some(next_cell) = &mut line.get_mut(column as usize + 1)
            {
                next_cell.reset();
                next_cell.set_symbol("");
            }
            self.redraw = true;
            self.reshape = true;
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
        let grid_bounds = self.bounds.grid_bounds();
        let view_bounds = self.bounds.view_bounds();
        Ok(WindowSize {
            columns_rows: Size::new(grid_bounds.width as u16, grid_bounds.height as u16),
            pixels: Size::new(view_bounds.width as u16, view_bounds.height as u16),
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
        let grid_bounds = self.bounds.grid_bounds();
        Ok(Size::new(
            grid_bounds.width as u16,
            grid_bounds.height as u16,
        ))
    }
}
