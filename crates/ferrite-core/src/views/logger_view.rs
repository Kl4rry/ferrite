use std::{sync::Arc, time::Duration};

use ferrite_geom::rect::Rect;
use ferrite_runtime::{Bounds, Painter, View};

use crate::{logger::LoggerState, theme::EditorTheme};

pub struct LoggerView {
    theme: Arc<EditorTheme>,
    render_time: Duration,
    has_focus: bool,
}

impl LoggerView {
    pub fn new(theme: Arc<EditorTheme>, render_time: Duration, has_focus: bool) -> Self {
        Self {
            theme,
            render_time,
            has_focus,
        }
    }
}

impl View<LoggerState> for LoggerView {
    fn render(&self, state: &mut LoggerState, bounds: Bounds, painter: &mut Painter) {
        let layer = painter.create_layer("logger", bounds);
        let mut layer = layer.lock().unwrap();
        let buf = &mut layer.buf;
        let area = bounds.grid_bounds();

        if area.area() == 0 {
            return;
        }

        buf.set_style(area.into(), self.theme.background);
        for y in 0..area.height.saturating_sub(1) {
            match state
                .messages
                .get(y as usize + state.lines_scrolled_up.floor() as usize)
            {
                Some(msg) => {
                    let string = format!("{:>5} {} {}", msg.level, msg.target, msg.fields.message);
                    buf.set_stringn(
                        area.x as u16,
                        (area.top() + area.height - y - 2) as u16, // TODO fix this - 2
                        string,
                        area.width.into(),
                        self.theme.text,
                    );
                }
                None => break,
            }
        }

        {
            let view_bounds = bounds.view_bounds();
            let cell_size = bounds.cell_size();
            let bottom_line_bounds = Bounds::new(
                Rect::new(
                    view_bounds.x,
                    (view_bounds.y as f32 + view_bounds.height as f32 - cell_size.y).round()
                        as usize,
                    view_bounds.width,
                    (1.0 * cell_size.y).round() as usize,
                ),
                cell_size,
                bounds.rounding,
            );

            let layer = painter.create_layer("logger-bottom-line", bottom_line_bounds);
            let mut layer = layer.lock().unwrap();
            let buf = &mut layer.buf;
            let line_area = bottom_line_bounds.grid_bounds();

            let style = if self.has_focus {
                self.theme.info_line
            } else {
                self.theme.info_line_unfocused
            };

            buf.set_style(line_area.into(), style);
            //#[cfg(not(feature = "talloc"))]
            let line = format!(" Frame time: {:?}", self.render_time);
            //#[cfg(feature = "talloc")]
            //let line = format!(
            //    " Frame time: {:?} Heap memory usage: {} Heap allocations: {}, Frame allocations: {}",
            //    self.render_time,
            //    ferrite_core::byte_size::format_byte_size(
            //        ferrite_talloc::Talloc::total_memory_allocated()
            //    ),
            //    ferrite_talloc::Talloc::num_allocations(),
            //    ferrite_talloc::Talloc::phase_allocations()
            //);

            buf.set_stringn(
                line_area.x as u16,
                line_area.y as u16,
                line,
                line_area.width.into(),
                style,
            );
        }
    }
}
