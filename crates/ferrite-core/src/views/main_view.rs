use std::sync::Arc;

use ferrite_geom::rect::Rect;
use ferrite_runtime::{Bounds, MouseInterction, View};

use crate::{
    engine::Engine,
    views::{palette_view::PaletteView, pane_view::PaneView, splash_view::SplashView},
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
        if mouse_interaction.is_drag() || engine.get_focus().is_pane() {
            return self.panes.handle_mouse(engine, bounds, mouse_interaction);
        }

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

        if engine.config.editor.show_splash && engine.workspace.panes.num_panes() == 1 {
            let Some((buffer_id, _)) = engine.get_current_buffer_id() else {
                return;
            };
            let buffer = &mut engine.workspace.buffers[buffer_id];
            if buffer.len_bytes() == 0
                && !buffer.is_dirty()
                && buffer.file().is_none()
                && engine.workspace.buffers.len() == 1
            {
                let theme: Arc<_> = engine.themes[&engine.config.editor.theme].clone();
                let splash = SplashView::new(theme);
                splash.render(&mut (), bounds, painter);
            }
        }
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
