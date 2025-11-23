use ferrite_geom::rect::Rect;
use ferrite_runtime::{Bounds, MouseInterction, View};

use crate::{
    engine::Engine,
    views::{palette_view::PaletteView, pane_view::PaneView},
};

pub struct MainView {
    panes: PaneView,
    palette: PaletteView,
}

impl MainView {
    pub fn new(panes: PaneView, palette: PaletteView) -> Self {
        Self { panes, palette }
    }
}

impl View<Engine> for MainView {
    fn handle_mouse(
        &self,
        engine: &mut Engine,
        bounds: Bounds,
        mouse_interaction: MouseInterction,
    ) -> bool {
        if self
            .palette
            .handle_mouse(&mut engine.palette, bounds, mouse_interaction)
        {
            return true;
        }
        let pane_bounds = calculate_bounds(engine, bounds);
        if pane_bounds.contains(mouse_interaction.position) {
            return self.panes.handle_mouse(engine, bounds, mouse_interaction);
        }
        false
    }

    fn render(&self, engine: &mut Engine, bounds: Bounds, painter: &mut ferrite_runtime::Painter) {
        let pane_bounds = calculate_bounds(engine, bounds);
        self.panes.render(engine, pane_bounds, painter);
        self.palette.render(&mut engine.palette, bounds, painter);
    }
}

fn calculate_bounds(engine: &mut Engine, bounds: Bounds) -> Bounds {
    let cell_size = bounds.cell_size();
    let view_bounds = bounds.view_bounds();
    // Calculate size of palette
    let palette_height = (engine.palette.height() as f32 * cell_size.y) as usize;

    let pane_pixel_area = Rect::new(
        view_bounds.left(),
        view_bounds.top(),
        view_bounds.width,
        view_bounds.height - palette_height,
    );
    Bounds::new(pane_pixel_area, cell_size, bounds.rounding)
}
