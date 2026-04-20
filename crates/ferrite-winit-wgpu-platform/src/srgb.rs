const GAMMA: f32 = 2.2;

pub fn srgb_to_linear(c: f32) -> f32 {
    c.powf(GAMMA)
}
