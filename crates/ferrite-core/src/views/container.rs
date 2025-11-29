use std::marker::PhantomData;

use ferrite_geom::rect::Rect;
use ferrite_runtime::{Bounds, View};

pub struct Container<V, S> {
    inner: V,
    margin_x: usize,
    margin_y: usize,
    grid_alinged: bool,
    phantom: PhantomData<fn(&mut S)>,
}

impl<V, S> Container<V, S>
where
    V: View<S> + 'static,
{
    pub fn new(inner: V) -> Self {
        Self {
            inner,
            margin_x: 0,
            margin_y: 0,
            grid_alinged: false,
            phantom: PhantomData,
        }
    }

    pub fn margin(mut self, margin_x: usize, margin_y: usize) -> Self {
        self.margin_x = margin_x;
        self.margin_y = margin_y;
        self
    }

    pub fn grid_alinged(mut self, grid_alinged: bool) -> Self {
        self.grid_alinged = grid_alinged;
        self
    }
}

impl<V, S> View<S> for Container<V, S>
where
    V: View<S> + 'static,
{
    fn render(&self, state: &mut S, bounds: Bounds, painter: &mut ferrite_runtime::Painter) {
        let cell_size = bounds.cell_size();

        let (x, y, width, height) = if self.grid_alinged {
            let grid_bounds = bounds.grid_bounds_floored();
            (
                grid_bounds.x as f32 * cell_size.x,
                grid_bounds.y as f32 * cell_size.y,
                grid_bounds.width as f32 * cell_size.x,
                grid_bounds.height as f32 * cell_size.y,
            )
        } else {
            let view_bounds = bounds.view_bounds();
            (
                view_bounds.x as f32,
                view_bounds.y as f32,
                view_bounds.width as f32,
                view_bounds.height as f32,
            )
        };

        let inner_bounds = Bounds::new(
            Rect::new(
                (x + self.margin_x as f32 * cell_size.x) as usize,
                (y + self.margin_y as f32 * cell_size.y) as usize,
                (width - self.margin_x as f32 * cell_size.x * 2.0) as usize,
                (height - self.margin_y as f32 * cell_size.y * 2.0) as usize,
            ),
            cell_size,
            bounds.rounding,
        );
        self.inner.render(state, inner_bounds, painter);
    }
}
