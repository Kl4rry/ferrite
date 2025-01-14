use std::mem;

use ferrite_core::{config::editor::FontWeight, theme::EditorTheme};
use glyphon::{
    cosmic_text::Scroll, Attrs, AttrsList, Buffer, BufferLine, Color, Family, FontSystem, Metrics,
    Shaping, TextArea, TextBounds, Weight,
};
use tui::{
    backend::WindowSize,
    buffer::Cell,
    layout::{Position, Size},
    prelude::Backend,
};
use unicode_width::UnicodeWidthStr;

use crate::{
    glue::convert_style,
    renderer::{
        geometry_renderer::{Geometry, Quad},
        Bundle,
    },
};

const LINE_SCALE: f32 = 1.3;
const FONT_SIZE: f32 = 14.0;
const REPLACED_SYMBOLS: &[&str] = &["☺️", "☹️"];
const REPLACEMENT_SYMBOLS: &[&str] = &["☺️ ", "☹️ "];

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

#[profiling::all_functions]
impl WgpuBackend {
    pub fn new(
        font_system: &mut FontSystem,
        width: f32,
        height: f32,
        font_family: String,
        font_weight: FontWeight,
    ) -> Self {
        font_system.db_mut().set_monospace_family(&font_family);
        let metrics = Metrics::relative(FONT_SIZE, LINE_SCALE);
        let mut buffer = Buffer::new(font_system, metrics);
        // borrowed from cosmic term
        let (cell_width, cell_height) = calculate_cell_size(font_system, metrics, font_weight);
        buffer.set_monospace_width(font_system, Some(cell_width));
        buffer.set_wrap(font_system, glyphon::Wrap::None);

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

        Self {
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

    pub fn prepare(&mut self, theme: &EditorTheme, font_system: &mut FontSystem) -> Bundle {
        let mut top_geometry = Geometry::default();
        let mut bottom_geometry = Geometry::default();
        self.buffer
            .set_size(font_system, Some(self.width), Some(self.height));

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
            let mut skip_next = false;
            let mut attr_list = AttrsList::new(default_attrs);
            let mut line_text = String::new();
            let mut idx = 0;
            for (col_idx, cell) in line.iter().enumerate() {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                let mut attrs = default_attrs;
                let mut fg = default_fg;
                let mut bg = None;
                if let tui::style::Color::Rgb(r, g, b) = cell.fg {
                    fg = glyphon::Color::rgb(r, g, b);
                }

                if let tui::style::Color::Rgb(r, g, b) = cell.bg {
                    bg = Some(glyphon::Color::rgb(r, g, b));
                }

                if cell.modifier.contains(tui::style::Modifier::REVERSED) {
                    let mut tmp = bg.unwrap_or(default_bg);
                    mem::swap(&mut fg, &mut tmp);
                    bg = Some(tmp);
                }

                attrs = attrs.color(fg);
                let symbol =
                    if let Some(idx) = REPLACED_SYMBOLS.iter().position(|s| *s == cell.symbol()) {
                        REPLACEMENT_SYMBOLS[idx]
                    } else {
                        cell.symbol()
                    };

                let symbol_width = symbol.width();
                if symbol_width > 1 {
                    skip_next = true;
                }
                line_text.push_str(symbol);
                attr_list.add_span(idx..(idx + symbol.len()), attrs);
                idx += symbol.len();
                // TODO greedy mesh here
                bottom_geometry.quads.push(Quad {
                    x: col_idx as f32 * self.cell_width,
                    y: line_idx as f32 * self.cell_height,
                    width: self.cell_width * symbol_width as f32,
                    height: self.cell_height * symbol_width as f32,
                    color: bg.unwrap_or(Color::rgba(0, 0, 0, 0)),
                });

                if cell.modifier.contains(tui::style::Modifier::SLOW_BLINK) {
                    let cursor_width = 2.0 * self.scale;
                    top_geometry.quads.push(Quad {
                        x: col_idx as f32 * self.cell_width,
                        y: line_idx as f32 * self.cell_height,
                        width: cursor_width,
                        height: self.cell_height,
                        color: Color::rgb(82, 139, 255),
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

        self.buffer.set_scroll(Scroll {
            line: 0,
            vertical: 0.0,
            horizontal: 0.0,
        });
        self.buffer.shape_until_scroll(font_system, true);
        font_system.shape_run_cache.trim(1024);

        let text_area = TextArea {
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
        };

        Bundle {
            text_area,
            top_geometry,
            bottom_geometry,
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

    pub fn set_font_weight(&mut self, font_system: &mut FontSystem, weight: FontWeight) {
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
        let metrics = Metrics::relative(FONT_SIZE * self.scale, LINE_SCALE);
        self.buffer.set_metrics(font_system, metrics);
        let (cell_width, cell_height) = calculate_cell_size(font_system, metrics, self.font_weight);
        self.buffer
            .set_monospace_width(font_system, Some(cell_width));
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
