use ferrite_geom::rect::{Rect, Vec2};
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

    fn draw_string_i32<T, S>(
        &mut self,
        x: i32,
        y: i32,
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
        x: u16,
        y: u16,
        string: T,
        area: tui_core::layout::Rect,
        style: S,
    ) where
        T: AsRef<str>,
        S: Into<Style>,
    {
        self.draw_string_i32(x as i32, y as i32, string, area, style);
    }

    fn draw_string_i32<T, S>(
        &mut self,
        mut x: i32,
        y: i32,
        string: T,
        area: tui_core::layout::Rect,
        style: S,
    ) where
        T: AsRef<str>,
        S: Into<Style>,
    {
        let area = Rect::new(
            area.x as i32,
            area.y as i32,
            area.width as i32,
            area.height as i32,
        );
        let string = string.as_ref();
        if y < area.y || y >= area.y + area.height {
            return;
        }
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
            if let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y))
                && area.contains(Vec2::new(x.into(), y.into()))
                && let Some(cell) = self.cell_mut((x, y))
            {
                cell.set_symbol(symbol.as_str().expect("rope slice should be contiguous"))
                    .set_style(style);
            }
            let next_symbol = x + width as i32;
            x += 1;
            // Reset following cells if multi-width (they would be hidden by the grapheme)
            while x < next_symbol {
                if let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y))
                    && area.contains(Vec2::new(x.into(), y.into()))
                    && let Some(cell) = self.cell_mut((x, y))
                {
                    cell.reset();
                }
                x += 1;
            }

            // Early exit optimization when we have a small area and very long string
            if x > area.x + area.width {
                break;
            }
        }
    }
}
