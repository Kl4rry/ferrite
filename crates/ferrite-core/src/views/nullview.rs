use ferrite_runtime::{Bounds, MouseInterction, Painter, View, any_view::AnyView};

pub struct NullView<S>(std::marker::PhantomData<S>);

impl<S: 'static> NullView<S> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }

    pub fn any() -> AnyView<S> {
        AnyView::new(Self(std::marker::PhantomData))
    }
}

impl<S: 'static> Default for NullView<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> View<S> for NullView<S> {
    fn handle_mouse(&self, _: &mut S, _: Bounds, _: MouseInterction) -> bool {
        false
    }

    fn render(&self, _: &mut S, _: Bounds, _: &mut Painter) {}
}
