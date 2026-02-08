use std::sync::Arc;

use ferrite_geom::rect::Rect;
use ferrite_runtime::{Bounds, Painter, View};
use ferrite_utility::tui_buf_ext::TuiBufExt;
use unicode_width::UnicodeWidthStr;

use crate::{palette::completer::Completer, theme::EditorTheme};

pub struct CompleterView {
    theme: Arc<EditorTheme>,
}

impl CompleterView {
    pub fn new(theme: Arc<EditorTheme>) -> Self {
        Self { theme }
    }
}

impl View<Completer> for CompleterView {
    fn render(&self, completer: &mut Completer, bounds: Bounds, painter: &mut Painter) {
        let cell_size = bounds.cell_size();
        let view_bounds = bounds.view_bounds();
        let bounds = Bounds::new(
            Rect::new(
                view_bounds.x,
                (view_bounds.bottom() as f32
                    - bounds.grid_bounds_floored().height as f32 * cell_size.y)
                    as usize,
                view_bounds.width,
                (bounds.grid_bounds_floored().height as f32 * cell_size.y) as usize,
            ),
            bounds.cell_size(),
            bounds.rounding,
        );

        let layer = painter.create_layer("palette completer", bounds);
        let mut layer = layer.lock().unwrap();

        let buf = &mut layer.buf;
        let area = bounds.grid_bounds();

        if completer.options().is_empty() {
            return;
        }

        let widest = completer
            .options()
            .iter()
            .map(|option| option.display().width())
            .max()
            .unwrap()
            + 8;

        let columns = (area.width / widest).max(1);
        let rows = completer
            .options()
            .len()
            .div_ceil(columns)
            .clamp(1, 10)
            .min(area.height);

        let completer_area = Rect::new(area.x, area.bottom() - rows, area.width, rows);

        buf.set_style(completer_area.into(), self.theme.completer);

        // TODO show correct page of completion alternatives

        for row in 0..rows {
            for col in 0..columns {
                let index = col * rows + row;
                let Some(option) = completer.options().get(index) else {
                    break;
                };
                let y = area.bottom() - rows + row;
                let x = area.left() + widest * col;
                let style = if Some(index) == completer.current() {
                    self.theme.completer_selected
                } else {
                    self.theme.completer
                };
                buf.draw_string(
                    x as u16,
                    y as u16,
                    option.display(),
                    completer_area.into(),
                    style,
                );
            }
        }
    }
}
