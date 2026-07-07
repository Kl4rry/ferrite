use std::{collections::HashMap, hash::Hash};

use ferrite_geom::rect::{Rect, Vec2};
use ferrite_style::Color;

use crate::{id::Id, input::mouse::MouseState};
pub mod id;
pub mod input;
pub mod widgets;

#[derive(Default)]
pub struct Canvas {
    quads: Vec<(Rect<f32>, Color)>,
}

impl Canvas {
    pub fn draw_quad(&mut self, rect: Rect<f32>, color: Color) {
        self.quads.push((rect, color))
    }

    pub fn get_overlay(&self) -> &[(Rect<f32>, Color)] {
        &self.quads
    }
}

pub struct Layer {
    pub area: Rect<f32>,
    pub buf: tui_core::buffer::Buffer,
    pub canvas: Option<Canvas>,
}

impl Layer {
    pub fn new(
        area: Rect<f32>,
        cell_size: Vec2<f32>,
        rounding: Rounding,
        canvas: Option<Canvas>,
    ) -> Self {
        let grid_size = calculate_grid_size(area, cell_size, rounding);
        Self {
            area,
            buf: tui_core::buffer::Buffer::empty(grid_size.into()),
            canvas,
        }
    }

    pub fn resize(&mut self, area: Rect<f32>, cell_size: Vec2<f32>, rounding: Rounding) {
        self.area = area;
        let grid_size = calculate_grid_size(area, cell_size, rounding);
        self.buf.resize(grid_size.into());
    }
}

pub struct Response {}

pub struct Ui {
    pub cell_size: Vec2<f32>,
    pub area: Rect<f32>,
    pub mouse_state: MouseState,
    /// Cache of layers from last frame
    layer_cache: HashMap<Id, Layer>,
    /// Layers that have been pop off the stack
    finshed_layers: Vec<Option<(Id, Layer)>>,
    /// Stack of layer ids in usage order
    layer_stack: Vec<(Id, Layer)>,
    cursor_zones: Vec<(CursorIcon, Rect)>,
    canvas: bool,
}

impl Ui {
    pub fn layer(&mut self) -> &mut Layer {
        self.layer_stack
            .last_mut()
            .map(|(_id, layer)| layer)
            .unwrap()
    }

    pub fn push_layer(&mut self, id: impl Hash + 'static, area: Rect<f32>, rounding: Rounding) {
        let id = Id::new(id);
        let layer = match self.layer_cache.remove(&id) {
            Some(mut layer) => {
                if layer.area != area {
                    layer.resize(area, self.cell_size, rounding);
                }
                layer
            }
            None if self.canvas => {
                Layer::new(area, self.cell_size, rounding, Some(Canvas::default()))
            }
            None => Layer::new(area, self.cell_size, rounding, None),
        };
        self.finshed_layers.push(None);
        self.layer_stack.push((id, layer));
    }

    pub fn pop_layer(&mut self) {
        let layer = self.layer_stack.pop();
        self.finshed_layers[self.layer_stack.len()] = layer;
    }

    pub fn finish_frame(&mut self) {
        self.layer_cache.clear();
        // self.cursor_zones.clear();
        for (id, mut layer) in self.finshed_layers.drain(..).filter_map(|l| l) {
            layer.buf.reset();
            if let Some(canvas) = &mut layer.canvas {
                canvas.quads.clear();
            }
            self.layer_cache.insert(id, layer);
        }
    }

    pub fn push_cursor_shape_zone(&mut self, icon: CursorIcon, zone: Rect) {
        self.cursor_zones.push((icon, zone));
    }

    // Creates a region that when the cursor is in a different set of regions
    // then last frame that application reruns all ui login.
    pub fn create_hover_region(&mut self, _area: Rect<f32>) {
        todo!();
    }
}

impl Ui {}

trait Widget {
    type State;
    fn ui(self, ui: &mut Ui, state: &mut Self::State) -> Response;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rounding {
    Floor,
    Ceil,
    #[default]
    Round,
}

pub fn calculate_grid_size(
    area: Rect<f32>,
    cell_size: Vec2<f32>,
    rounding: Rounding,
) -> Rect<usize> {
    match rounding {
        Rounding::Round => Rect::new(
            (area.x as f32 / cell_size.x).round() as usize,
            (area.y as f32 / cell_size.y).round() as usize,
            (area.width as f32 / cell_size.x).round() as usize,
            (area.height as f32 / cell_size.y).round() as usize,
        ),
        Rounding::Ceil => Rect::new(
            (area.x as f32 / cell_size.x).ceil() as usize,
            (area.y as f32 / cell_size.y).ceil() as usize,
            (area.width as f32 / cell_size.x).ceil() as usize,
            (area.height as f32 / cell_size.y).ceil() as usize,
        ),
        Rounding::Floor => Rect::new(
            (area.x as f32 / cell_size.x).floor() as usize,
            (area.y as f32 / cell_size.y).floor() as usize,
            (area.width as f32 / cell_size.x).floor() as usize,
            (area.height as f32 / cell_size.y).floor() as usize,
        ),
    }
}

#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy)]
pub enum CursorIcon {
    #[default]
    Default,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    EResize,
    NResize,
    NeResize,
    NwResize,
    SResize,
    SeResize,
    SwResize,
    WResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
    AllScroll,
    ZoomIn,
    ZoomOut,
}
