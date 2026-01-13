use crate::layout::panes::PaneKind;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Palette,
    Pane(PaneKind),
    Picker,
}

impl Focus {
    pub fn is_pane(&self) -> bool {
        matches!(self, Self::Pane(_))
    }
}
