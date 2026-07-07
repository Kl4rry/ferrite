use ferrite_geom::rect::Rect;
use ferrite_style::Style;
use ferrite_utility::tui_buf_ext::TuiBufExt;

use crate::{Response, Ui, Widget};

pub struct TextButton {
    text: String, // TODO: rm temp alloc
    area: Rect<i32>,
    style: Style,
}

impl TextButton {
    pub fn new(text: String, area: Rect<i32>) -> Self {
        Self {
            text,
            area,
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for TextButton {
    type State = ();
    fn ui(self, ui: &mut Ui, _: &mut Self::State) -> Response {
        let layer = ui.layer();
        layer.buf.draw_string_i32(
            self.area.x,
            self.area.y,
            self.text,
            self.area,
            self.style,
        );
        // TODO: create hover zone if hover style is set
        Response {}
    }
}
