use std::marker::PhantomData;

use super::{Bounds, View};

pub struct AnyView<S> {
    inner: Box<dyn View<S>>,
    phantom: PhantomData<fn(&mut S)>,
}

impl<S> AnyView<S> {
    pub fn new<V: View<S> + 'static>(inner: V) -> Self {
        Self {
            inner: Box::new(inner),
            phantom: PhantomData,
        }
    }
}

impl<S> View<S> for AnyView<S> {
    fn render(&self, state: &mut S, bounds: Bounds, painter: &mut super::Painter) {
        self.inner.render(state, bounds, painter);
    }
}
