use tui_core::layout::Rect;

pub trait RectExt {
    fn clamp_within(&self, outer: Rect) -> Rect;
}

impl RectExt for Rect {
    fn clamp_within(&self, outer: Rect) -> Rect {
        let left = self.left().max(outer.left());
        let right = self.right().min(outer.right());
        let top = self.top().max(outer.top());
        let bottom = self.bottom().min(outer.bottom());
        Rect {
            x: left,
            y: top,
            width: right.saturating_sub(left),
            height: bottom.saturating_sub(top),
        }
    }
}
