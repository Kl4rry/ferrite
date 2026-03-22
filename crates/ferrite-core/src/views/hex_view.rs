use std::{fmt::Write, sync::Arc};

use ferrite_geom::rect::Vec2;
use ferrite_runtime::{Bounds, View, painter::Rounding};
use ferrite_utility::tui_buf_ext::TuiBufExt;
use unicode_width::UnicodeWidthStr;

use crate::{
    config::editor::Editor,
    hex::{Hex, HexViewId},
    theme::EditorTheme,
    views::info_line_hex_view::InfoLineView,
};

pub struct HexBufferView {
    hex_view_id: HexViewId,
    config: Arc<Editor>,
    theme: Arc<EditorTheme>,
    has_focus: bool,
    branch: Option<String>,
    spinner: Option<char>,
    pub info_line: bool,
    pub ceil_surface_size: bool,
    pub scrollbar: bool,
}

impl HexBufferView {
    pub fn new(
        hex_view_id: HexViewId,
        config: Arc<Editor>,
        theme: Arc<EditorTheme>,
        has_focus: bool,
        branch: Option<String>,
        spinner: Option<char>,
    ) -> Self {
        Self {
            hex_view_id,
            config,
            theme,
            has_focus,
            branch,
            spinner,
            ceil_surface_size: false,
            info_line: true,
            scrollbar: true,
        }
    }

    pub fn set_ceil_surface_size(mut self, ceil_surface_size: bool) -> Self {
        self.ceil_surface_size = ceil_surface_size;
        self
    }

    pub fn set_scrollbar(mut self, scrollbar: bool) -> Self {
        self.scrollbar = scrollbar;
        self
    }
}

impl View<Hex> for HexBufferView {
    fn render(&self, hex: &mut Hex, mut bounds: Bounds, painter: &mut ferrite_runtime::Painter) {
        let hex_view_id = self.hex_view_id;
        let _has_focus = self.has_focus;
        let unique_id = hex.views[hex_view_id].unique_id();
        let grid_bounds = bounds.grid_bounds();

        let rounding = if self.ceil_surface_size && bounds.cell_size() != Vec2::new(1.0, 1.0) {
            Rounding::Ceil
        } else {
            Rounding::Round
        };
        bounds.rounding = rounding;

        let layer = painter.create_layer(unique_id, bounds);
        let mut layer = layer.lock().unwrap();

        layer
            .buf
            .set_style(grid_bounds.into(), self.theme.background);

        let len_bytes = hex.bytes.len();
        let line_pos = hex.views[hex_view_id].line_pos.floor() as usize;
        let last_line = len_bytes.div_ceil(0x10);
        // TODO: rm tmp alloc
        let min_width = format!("{:x}", last_line).width().max(6);

        // TODO: rm tmp alloc
        let mut temp_string = String::with_capacity(256);
        let mut chunk_index = 0;
        loop {
            let start = ((chunk_index + line_pos) * 0x10).min(len_bytes);
            let end = (start + 0x10).min(len_bytes);

            if chunk_index >= grid_bounds.height || start == end {
                break;
            }
            temp_string.clear();

            write!(temp_string, "0x{:0width$x}:  ", start, width = min_width).unwrap();

            let slice = &hex.bytes[start..end];
            for byte in slice {
                write!(temp_string, " {:02x}", byte).unwrap();
            }

            for _ in 0..(0x10 - slice.len()) {
                write!(temp_string, "   ").unwrap();
            }

            write!(temp_string, "   ").unwrap();

            // TODO: add color to hex values
            for byte in slice {
                if *byte == 0 {
                    write!(temp_string, "0").unwrap();
                } else if *byte == b'\n' {
                    write!(temp_string, "_").unwrap();
                } else if byte.is_ascii_graphic() {
                    write!(temp_string, "{}", *byte as char).unwrap();
                } else if byte.is_ascii_whitespace() {
                    write!(temp_string, " ").unwrap();
                } else if byte.is_ascii() {
                    write!(temp_string, "•").unwrap();
                } else {
                    write!(temp_string, "x").unwrap();
                }
            }

            layer.buf.draw_string(
                grid_bounds.x as u16,
                (grid_bounds.y + chunk_index) as u16,
                &temp_string,
                grid_bounds.into(),
                self.theme.text,
            );

            chunk_index += 1;
        }

        // TODO add scrollbar

        if self.info_line {
            let path: String = if let Some(path) = hex.file() {
                path.to_string_lossy().into()
            } else {
                String::from("unnamed")
            };

            let info_line = InfoLineView {
                theme: &self.theme,
                config: &self.config.info_line,
                focus: self.has_focus,
                path,
                branch: &self.branch,
                size: hex.bytes.len(),
                spinner: self.spinner,
                parent_unique_id: unique_id,
            };
            info_line.render(&mut (), bounds.bottom_lines(1), painter);
        }
    }
}
