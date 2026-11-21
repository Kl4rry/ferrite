use ferrite_linalg::*;
use ferrite_style::Style;
use unicode_width::UnicodeWidthStr;

use crate::{Response, Ui, Widget};

pub struct TextButton {
    text: String, // TODO: rm temp alloc
    style: Style,
}

impl TextButton {
    pub fn new(text: String) -> Self {
        Self {
            text,
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
        let rect = ui.allocate_cell_space((
            self.text.width_cjk() as i32,
            self.text.lines().count() as i32,
        ));
        // TODO: add separate id for text that changes
        ui.push_widget(&self.text, rect.map(|v| v as f32));
        let layer = ui.layer();
        layer
            .grid
            .draw_string(rect.x, rect.y, self.text, rect, self.style);
        ui.pop_widget();
        // TODO: create hover zone if hover style is set
        Response {}
    }
}
