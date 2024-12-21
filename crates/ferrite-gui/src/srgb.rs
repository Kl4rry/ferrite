pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        return c / 12.92;
    } else {
        return ((c + 0.055) / 1.055).powf(2.4);
    }
}
