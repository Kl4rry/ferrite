use ropey::RopeSlice;
use tui_core::style::Style;

use crate::graphemes::RopeGraphemeExt;

pub trait TuiBufExt {
    fn draw_string<T, S>(
        &mut self,
        x: u16,
        y: u16,
        string: T,
        area: tui_core::layout::Rect,
        style: S,
    ) where
        T: AsRef<str>,
        S: Into<Style>;
}

// TODO: switch coord type to i32
impl TuiBufExt for tui_core::buffer::Buffer {
    fn draw_string<T, S>(
        &mut self,
        mut x: u16,
        y: u16,
        string: T,
        area: tui_core::layout::Rect,
        style: S,
    ) where
        T: AsRef<str>,
        S: Into<Style>,
    {
        let string = string.as_ref();
        if y < area.y || y >= area.y + area.height {
            return;
        }
        let mut graphemes_to_skip = if x < area.x {
            x.saturating_sub(area.x)
        } else {
            0
        };
        let mut remaining_width = area.width.saturating_sub(x.saturating_sub(area.x)) as usize;
        let rope = RopeSlice::from(string);

        let graphemes = rope
            .graphemes()
            .filter(|grapheme| {
                !grapheme
                    .as_str()
                    .expect("rope slice should be contiguous")
                    .contains(char::is_control)
            })
            .map(|symbol| (symbol, symbol.width(0) as u16))
            .filter(|(_symbol, width)| *width > 0);

        let style = style.into();
        for (symbol, width) in graphemes {
            if graphemes_to_skip > width {
                graphemes_to_skip -= width;
                continue;
            } else {
                // Add some padding if the last grapheme is larger then 1 cell
                x += graphemes_to_skip;
            }

            remaining_width = match remaining_width.checked_sub(width.into()) {
                Some(remaining_width) => remaining_width,
                None => break,
            };

            self[(x, y)]
                .set_symbol(symbol.as_str().expect("rope slice should be contiguous"))
                .set_style(style);
            let next_symbol = x + width;
            x += 1;
            // Reset following cells if multi-width (they would be hidden by the grapheme)
            while x < next_symbol {
                self[(x, y)].reset();
                x += 1;
            }
        }
    }
}
