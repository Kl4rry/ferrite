use ferrite_linalg::*;

use crate::Rounding;

pub fn rect_to_tui(rect: Rect<i32>) -> tui_core::layout::Rect {
    tui_core::layout::Rect {
        x: rect.x.try_into().unwrap(),
        y: rect.y.try_into().unwrap(),
        width: rect.width.try_into().unwrap(),
        height: rect.height.try_into().unwrap(),
    }
}

pub fn calculate_grid_size(
    area: Rect<f32>,
    cell_size: impl Into<Vec2<f32>>,
    rounding: Rounding,
) -> Rect<i32> {
    let cell_size = cell_size.into();
    match rounding {
        Rounding::Round => Rect::new(
            (area.x as f32 / cell_size.x).round() as i32,
            (area.y as f32 / cell_size.y).round() as i32,
            (area.width as f32 / cell_size.x).round() as i32,
            (area.height as f32 / cell_size.y).round() as i32,
        ),
        Rounding::Ceil => Rect::new(
            (area.x as f32 / cell_size.x).ceil() as i32,
            (area.y as f32 / cell_size.y).ceil() as i32,
            (area.width as f32 / cell_size.x).ceil() as i32,
            (area.height as f32 / cell_size.y).ceil() as i32,
        ),
        Rounding::Floor => Rect::new(
            (area.x as f32 / cell_size.x).floor() as i32,
            (area.y as f32 / cell_size.y).floor() as i32,
            (area.width as f32 / cell_size.x).floor() as i32,
            (area.height as f32 / cell_size.y).floor() as i32,
        ),
    }
}
