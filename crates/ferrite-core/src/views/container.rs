use std::marker::PhantomData;

use ferrite_geom::rect::Rect;
use ferrite_runtime::{Bounds, View};

pub struct Container<V, S> {
    inner: V,
    margin_x: usize,
    margin_y: usize,
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
            phantom: PhantomData,
        }
    }

    pub fn margin(mut self, margin_x: usize, margin_y: usize) -> Self {
        self.margin_x = margin_x;
        self.margin_y = margin_y;
        self
    }
}

impl<V, S> View<S> for Container<V, S>
where
    V: View<S> + 'static,
{
    fn render(&self, state: &mut S, bounds: Bounds, painter: &mut ferrite_runtime::Painter) {
        let cell_size = bounds.cell_size();
        let view_bounds = bounds.view_bounds();
        let inner_bounds = Bounds::new(
            Rect::new(
                (view_bounds.x as f32 + self.margin_x as f32 * cell_size.x) as usize,
                (view_bounds.y as f32 + self.margin_y as f32 * cell_size.y) as usize,
                (view_bounds.width as f32 - self.margin_x as f32 * cell_size.x as f32 * 2.0)
                    as usize,
                (view_bounds.height as f32 - self.margin_y as f32 * cell_size.y as f32 * 2.0)
                    as usize,
            ),
            cell_size,
            bounds.rounding,
        );
        self.inner.render(state, inner_bounds, painter);
    }
}
