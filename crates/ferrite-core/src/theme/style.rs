use csscolorparser::Color;

#[derive(Debug, Default, Clone)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}
