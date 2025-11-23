use std::marker::PhantomData;

use ferrite_runtime::{Bounds, MouseInterction, Painter, View};

pub struct Lens<F, V, ParentState, ChildState> {
    view: V,
    f: F,
    panthom: PhantomData<fn(&mut ParentState) -> &mut ChildState>,
}

impl<F, V, ParentState, ChildState> Lens<F, V, ParentState, ChildState>
where
    F: Fn(&mut ParentState) -> &mut ChildState,
{
    pub fn new(view: V, f: F) -> Self {
        Self {
            view,
            f,
            panthom: PhantomData,
        }
    }
}

impl<F, V, ParentState, ChildState> View<ParentState> for Lens<F, V, ParentState, ChildState>
where
    ParentState: 'static,
    ChildState: 'static,
    V: View<ChildState>,
    F: Fn(&mut ParentState) -> &mut ChildState + 'static,
{
    fn handle_mouse(
        &self,
        parent_state: &mut ParentState,
        bounds: Bounds,
        mouse_interaction: MouseInterction,
    ) -> bool {
        let child_state: &mut ChildState = (self.f)(parent_state);
        self.view
            .handle_mouse(child_state, bounds, mouse_interaction)
    }

    fn render(&self, parent_state: &mut ParentState, bounds: Bounds, painter: &mut Painter) {
        let child_state: &mut ChildState = (self.f)(parent_state);
        self.view.render(child_state, bounds, painter);
    }
}
