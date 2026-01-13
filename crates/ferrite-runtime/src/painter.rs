use std::{
    any::{Any, TypeId},
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
};

use ferrite_geom::rect::{Rect, Vec2};
use ferrite_style::Color;

use crate::Id;

#[derive(Default)]
pub struct Painter2D {
    quads: Vec<(Rect<f32>, Color)>,
}

impl Painter2D {
    pub fn draw_quad(&mut self, rect: Rect<f32>, color: Color) {
        self.quads.push((rect, color))
    }

    pub fn get_overlay(&self) -> &[(Rect<f32>, Color)] {
        &self.quads
    }
}

pub struct Painter {
    layer_cache: HashMap<(TypeId, Id), Layer>,
    layers: Vec<(TypeId, Id, Arc<Mutex<Layer>>)>,
    cursor_zones: Vec<(CursorIcon, Rect)>,
    painter2d: bool,
}

impl Painter {
    pub fn new(painter2d: bool) -> Self {
        Self {
            layer_cache: HashMap::new(),
            layers: Vec::new(),
            cursor_zones: Vec::new(),
            painter2d,
        }
    }

    pub fn create_layer(&mut self, id: impl Hash + 'static, bounds: Bounds) -> Arc<Mutex<Layer>> {
        let type_id = id.type_id();
        let id = Id::new(id);
        let layer = match self.layer_cache.remove(&(type_id, id)) {
            Some(mut layer) => {
                if layer.bounds != bounds {
                    layer.resize(bounds);
                }
                layer
            }
            None if self.painter2d => Layer::new(bounds, Some(Painter2D::default())),
            None => Layer::new(bounds, None),
        };
        let layer = Arc::new(Mutex::new(layer));
        self.layers.push((type_id, id, layer.clone()));
        layer
    }

    pub fn clean_up_frame(&mut self) {
        self.layer_cache.clear();
        self.cursor_zones.clear();
        for (type_id, id, layer) in self.layers.drain(..) {
            let mut layer: Layer = Arc::into_inner(layer).unwrap().into_inner().unwrap();
            layer.buf.reset();
            if let Some(painter2d) = &mut layer.painter2d {
                painter2d.quads.clear();
            }
            self.layer_cache.insert((type_id, id), layer);
        }
    }

    pub fn layers(&self) -> &[(TypeId, Id, Arc<Mutex<Layer>>)] {
        &self.layers
    }

    pub fn has_painter2d(&self) -> bool {
        self.painter2d
    }

    pub fn push_cursor_zone(&mut self, icon: CursorIcon, zone: Rect) {
        self.cursor_zones.push((icon, zone));
    }

    pub fn cursor_zones(&self) -> &[(CursorIcon, Rect)] {
        &self.cursor_zones
    }
}

pub struct Layer {
    pub bounds: Bounds,
    pub buf: tui_core::buffer::Buffer,
    pub painter2d: Option<Painter2D>,
}

impl Layer {
    pub fn new(bounds: Bounds, painter2d: Option<Painter2D>) -> Self {
        Self {
            bounds,
            buf: tui_core::buffer::Buffer::empty(bounds.grid_bounds().into()),
            painter2d,
        }
    }

    pub fn resize(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        self.buf.resize(bounds.grid_bounds().into());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    bounds: Rect,
    cell_size: Vec2<f32>,
    pub rounding: Rounding,
}

impl Bounds {
    pub fn new(bounds: Rect, cell_size: Vec2<f32>, rounding: Rounding) -> Self {
        Self {
            bounds,
            cell_size,
            rounding,
        }
    }

    pub fn from_grid_bounds(bounds: Rect, cell_size: Vec2<f32>, rounding: Rounding) -> Self {
        Self {
            bounds: Rect::new(
                (bounds.x as f32 * cell_size.x) as usize,
                (bounds.y as f32 * cell_size.y) as usize,
                (bounds.width as f32 * cell_size.x) as usize,
                (bounds.height as f32 * cell_size.y) as usize,
            ),
            cell_size,
            rounding,
        }
    }

    pub fn view_bounds(&self) -> Rect {
        self.bounds
    }

    pub fn grid_bounds(&self) -> Rect {
        self.grid_bounds_with_rounding(self.rounding)
    }

    fn grid_bounds_with_rounding(&self, rounding: Rounding) -> Rect {
        match rounding {
            Rounding::Round => self.grid_bounds_rounded(),
            Rounding::Floor => self.grid_bounds_floored(),
            Rounding::Ceil => self.grid_bounds_ceil(),
        }
    }

    pub fn grid_bounds_rounded(&self) -> Rect {
        Rect::new(
            (self.bounds.x as f32 / self.cell_size.x).round() as usize,
            (self.bounds.y as f32 / self.cell_size.y).round() as usize,
            (self.bounds.width as f32 / self.cell_size.x).round() as usize,
            (self.bounds.height as f32 / self.cell_size.y).round() as usize,
        )
    }

    pub fn grid_bounds_floored(&self) -> Rect {
        Rect::new(
            (self.bounds.x as f32 / self.cell_size.x).floor() as usize,
            (self.bounds.y as f32 / self.cell_size.y).floor() as usize,
            (self.bounds.width as f32 / self.cell_size.x).floor() as usize,
            (self.bounds.height as f32 / self.cell_size.y).floor() as usize,
        )
    }

    pub fn grid_bounds_ceil(&self) -> Rect {
        Rect::new(
            (self.bounds.x as f32 / self.cell_size.x).ceil() as usize,
            (self.bounds.y as f32 / self.cell_size.y).ceil() as usize,
            (self.bounds.width as f32 / self.cell_size.x).ceil() as usize,
            (self.bounds.height as f32 / self.cell_size.y).ceil() as usize,
        )
    }

    pub fn cell_size(&self) -> Vec2<f32> {
        self.cell_size
    }

    pub fn contains(&self, view_position: Vec2<f32>) -> bool {
        let view_bounds = self.view_bounds();
        let view_bounds = Rect::new(
            view_bounds.x as f32,
            view_bounds.y as f32,
            view_bounds.width as f32,
            view_bounds.height as f32,
        );
        view_bounds.contains(view_position)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rounding {
    Floor,
    Ceil,
    #[default]
    Round,
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
