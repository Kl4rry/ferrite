pub fn convert_style(style: &ferrite_core::theme::style::Style) -> tui::style::Style {
    tui::style::Style {
        fg: style.fg.as_ref().map(convert_color),
        bg: style.bg.as_ref().map(convert_color),
        ..Default::default()
    }
}

pub fn convert_color(color: &ferrite_core::theme::style::Color)-> tui::style::Color {
    tui::style::Color::Rgb(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
    )
}

pub fn tui_to_ferrite_rect(rect: tui::layout::Rect) -> ferrite_core::layout::panes::Rect {
    ferrite_core::layout::panes::Rect {
        x: rect.x.into(),
        y: rect.y.into(),
        width: rect.width.into(),
        height: rect.height.into(),
    }
}

pub fn ferrite_to_tui_rect(rect: ferrite_core::layout::panes::Rect) -> tui::layout::Rect {
    tui::layout::Rect {
        x: rect.x.try_into().unwrap(),
        y: rect.y.try_into().unwrap(),
        width: rect.width.try_into().unwrap(),
        height: rect.height.try_into().unwrap(),
    }
}
