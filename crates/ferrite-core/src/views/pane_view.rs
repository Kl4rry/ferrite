use ferrite_geom::rect::Vec2;
use ferrite_runtime::{Bounds, MouseInterction, View, any_view::AnyView};

use crate::{
    engine::Engine,
    layout::panes::PaneKind,
    views::{
        editor_view::EditorView, file_explorer_view::FileExplorerView, lens::Lens,
        logger_view::LoggerView,
    },
};

pub struct PaneView {
    views: Vec<(PaneKind, AnyView<Engine>)>,
}

impl PaneView {
    pub fn new(engine: &mut Engine) -> Self {
        let current_pane = engine.workspace.panes.get_current_pane();
        let iter = engine.workspace.panes.get_list_of_panes().into_iter().map(
            |pane_kind| -> (PaneKind, AnyView<Engine>) {
                (
                    pane_kind,
                    match pane_kind {
                        PaneKind::Buffer(buffer_id, view_id) => {
                            AnyView::new(Lens::new(
                                EditorView::new(
                                    view_id,
                                    engine.config.editor.clone(),
                                    engine.themes[&engine.config.editor.theme].clone(),
                                    // TODO Move focus checking into engine
                                    !engine.palette.has_focus()
                                        && engine.file_picker.is_none()
                                        && engine.buffer_picker.is_none()
                                        && engine.global_search_picker.is_none()
                                        && current_pane == PaneKind::Buffer(buffer_id, view_id),
                                    engine.branch_watcher.current_branch(),
                                    engine.spinner.current(),
                                )
                                .set_ceil_surface_size(true),
                                move |engine: &mut Engine| &mut engine.workspace.buffers[buffer_id],
                            ))
                        }
                        PaneKind::FileExplorer(file_explorer_id) => {
                            AnyView::new(Lens::new(
                                FileExplorerView::new(
                                    engine.config.editor.clone(),
                                    engine.themes[&engine.config.editor.theme].clone(),
                                    // TODO Move focus checking into engine
                                    !engine.palette.has_focus()
                                        && engine.file_picker.is_none()
                                        && engine.buffer_picker.is_none()
                                        && engine.global_search_picker.is_none()
                                        && current_pane == PaneKind::FileExplorer(file_explorer_id),
                                ),
                                move |engine: &mut Engine| {
                                    &mut engine.workspace.file_explorers[file_explorer_id]
                                },
                            ))
                        }
                        PaneKind::Logger => AnyView::new(Lens::new(
                            LoggerView::new(
                                engine.themes[&engine.config.editor.theme].clone(),
                                engine.last_render_time,
                                !engine.palette.has_focus()
                                    && engine.file_picker.is_none()
                                    && engine.buffer_picker.is_none()
                                    && engine.global_search_picker.is_none()
                                    && current_pane == PaneKind::Logger,
                            ),
                            move |engine: &mut Engine| &mut engine.logger_state,
                        )),
                    },
                )
            },
        );
        Self {
            views: iter.collect(),
        }
    }
}

impl View<Engine> for PaneView {
    fn handle_mouse(
        &self,
        engine: &mut Engine,
        bounds: Bounds,
        mouse_interaction: MouseInterction,
    ) -> bool {
        let pane_bounds = engine.workspace.panes.get_pane_bounds(bounds.view_bounds());
        for (pane_kind, pane_bound) in pane_bounds {
            let (_, view) = self
                .views
                .iter()
                .find(|(view_pane_kind, _)| pane_kind == *view_pane_kind)
                .unwrap();
            if pane_bound.contains(Vec2::new(
                mouse_interaction.position.x as usize,
                mouse_interaction.position.y as usize,
            )) {
                engine.workspace.panes.make_current(pane_kind);
                view.handle_mouse(
                    engine,
                    Bounds::new(pane_bound, bounds.cell_size(), bounds.rounding),
                    mouse_interaction,
                );
                return true;
            }
        }
        false
    }

    fn render(&self, engine: &mut Engine, bounds: Bounds, painter: &mut ferrite_runtime::Painter) {
        // TODO: draw separating lines
        let pane_bounds = engine.workspace.panes.get_pane_bounds(bounds.view_bounds());
        for (pane_kind, pane_bound) in pane_bounds {
            let (_, view) = self
                .views
                .iter()
                .find(|(view_pane_kind, _)| pane_kind == *view_pane_kind)
                .unwrap();
            view.render(
                engine,
                Bounds::new(pane_bound, bounds.cell_size(), bounds.rounding),
                painter,
            );
        }
    }
}
