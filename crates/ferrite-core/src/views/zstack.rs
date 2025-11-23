use ferrite_runtime::{Bounds, MouseInterction, Painter, View, any_view::AnyView};

pub struct ZStack<S> {
    seq: Vec<AnyView<S>>,
}

impl<S> ZStack<S> {
    pub fn new(seq: Vec<AnyView<S>>) -> Self {
        Self { seq }
    }
}

impl<S> View<S> for ZStack<S> {
    fn handle_mouse(
        &self,
        state: &mut S,
        bounds: Bounds,
        mouse_interaction: MouseInterction,
    ) -> bool {
        for layer in self.seq.iter().rev() {
            if layer.handle_mouse(state, bounds, mouse_interaction) {
                break;
            }
        }
        true
    }

    fn render(&self, state: &mut S, bounds: Bounds, painter: &mut Painter) {
        for layer in &self.seq {
            layer.render(state, bounds, painter);
        }
    }
}
