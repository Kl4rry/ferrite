use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use ferrite_linalg::*;
use ferrite_style::Color;
use ferrite_utility::graphemes::RopeGraphemeExt;
use ropey::RopeSlice;
use tui_core::style::Style;

pub mod control_flow;
mod event_loop_proxy;
mod id;
mod input;
mod utility;
pub mod widgets;

pub use control_flow::EventLoopControlFlow;
pub use event_loop_proxy::*;
pub use id::Id;
pub use input::{
    event::InputEvent,
    keycode::{KeyCode, KeyModifiers},
    mouse::{MouseButton, MouseInterction, MouseInterctionKind, MouseState},
};
pub use utility::{calculate_grid_size, rect_to_tui};

pub struct Response {}

pub struct Ui {
    pub rect: Rect<f32>,
    pub cell_size: Vec2<f32>,
    mouse_state: MouseState,
    /// Cache of layers from last frame
    layer_cache: HashMap<Id, Layer>,
    /// Layers that have been pop off the stack
    finshed_layers: Vec<Option<Layer>>,
    /// Stack of layer ids in usage order
    layer_stack: Vec<Layer>,
    /// Zones that affect the cursor icon
    cursor_zones: Vec<(CursorIcon, Rect<i32>)>,
    /// Widgets created this frame
    current_frames_widgets: Vec<WidgetData>,
    /// Widgets from last frame (used for mouse hit detection)
    last_frames_widgets: Vec<WidgetData>,
    /// Ui has a canvas
    canvas: bool,
}

impl Ui {
    pub fn new(rect: Rect<f32>, cell_size: Vec2<f32>, canvas: bool) -> Self {
        Self {
            rect,
            cell_size,
            mouse_state: Default::default(),
            layer_cache: HashMap::new(),
            finshed_layers: Vec::new(),
            layer_stack: Vec::new(),
            cursor_zones: Vec::new(),
            current_frames_widgets: Vec::new(),
            last_frames_widgets: Vec::new(),
            canvas,
        }
    }

    pub fn layer(&mut self) -> &mut Layer {
        self.layer_stack.last_mut().unwrap()
    }

    pub fn push_layer(
        &mut self,
        id: impl std::hash::Hash + std::fmt::Debug + 'static,
        rect: Rect<f32>,
        rounding: Rounding,
    ) {
        let parent = self.layer_stack.last();
        let last_z_index = parent.map(|layer| layer.z_index).unwrap_or(1.0);
        let parent_id = parent.map(|layer| layer.id).unwrap_or(Id::NULL);
        let id = parent_id.with(id);
        let mut layer = match self.layer_cache.remove(&id) {
            Some(mut layer) => {
                if layer.rect != rect {
                    layer.resize(rect, self.cell_size, rounding);
                }
                layer
            }
            None if self.canvas => {
                Layer::new(id, rect, self.cell_size, rounding, Some(Canvas::default()))
            }
            None => Layer::new(id, rect, self.cell_size, rounding, None),
        };
        layer.z_index = last_z_index + 1.0;
        self.finshed_layers.push(None);
        self.layer_stack.push(layer);
    }

    pub fn pop_layer(&mut self) {
        let layer = self
            .layer_stack
            .pop()
            .expect("layer list should not be empty");
        self.finshed_layers[self.layer_stack.len()] = Some(layer);
    }

    pub fn finish_frame(&mut self) {
        self.layer_cache.clear();
        self.cursor_zones.clear();
        for mut layer in self.finshed_layers.drain(..).filter_map(|l| l) {
            layer.grid.reset();
            if let Some(canvas) = &mut layer.canvas {
                canvas.quads.clear();
            }
            self.layer_cache.insert(layer.id, layer);
        }
        self.last_frames_widgets.clear();
        std::mem::swap(
            &mut self.last_frames_widgets,
            &mut self.current_frames_widgets,
        );
    }

    pub fn push_cursor_shape_zone(&mut self, icon: CursorIcon, zone: Rect<i32>) {
        self.cursor_zones.push((icon, zone));
    }

    // Creates a region that when the cursor is in a different set of regions
    // then last frame that application reruns all ui login.
    pub fn create_hover_region(&mut self, _area: Rect<f32>) {
        todo!();
    }

    // low level utility function
    pub fn create_widget(&mut self, id: impl std::hash::Hash + std::fmt::Debug, rect: Rect<f32>) {
        let z_index = self.layer().z_index;
        self.current_frames_widgets.push(WidgetData {
            // TODO: include entropy from parent widget so widget ids do not have to be globally unique
            id: Id::new(id),
            rect,
            z_index,
        });
    }

    pub fn cursor_zones(&self) -> &[(CursorIcon, Rect<i32>)] {
        &self.cursor_zones
    }

    pub fn finished_layers(&self) -> impl Iterator<Item = &Layer> {
        self.finshed_layers.iter().map(|l| l.as_ref().unwrap())
    }
}

// interal widget data
struct WidgetData {
    id: Id,
    rect: Rect<f32>,
    z_index: f32,
}

struct CreateWidgetResponse {
    // this will hold data abut clicks so we can check for that
}

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

pub struct Layer {
    pub id: Id,
    pub rect: Rect<f32>,
    pub grid: Grid,
    pub canvas: Option<Canvas>,
    pub z_index: f32,
    pub rounding: Rounding,
}

impl Layer {
    pub fn new(
        id: Id,
        rect: Rect<f32>,
        cell_size: Vec2<f32>,
        rounding: Rounding,
        canvas: Option<Canvas>,
    ) -> Self {
        let grid_size = calculate_grid_size(rect, cell_size, rounding);
        Self {
            id,
            rect,
            grid: Grid {
                buf: tui_core::buffer::Buffer::empty(rect_to_tui(grid_size)),
            },
            canvas,
            z_index: 1.0,
            rounding,
        }
    }

    pub fn resize(&mut self, rect: Rect<f32>, cell_size: Vec2<f32>, rounding: Rounding) {
        self.rect = rect;
        let grid_size = calculate_grid_size(rect, cell_size, rounding);
        self.grid.resize(grid_size);
    }
}

pub struct Grid {
    buf: tui_core::buffer::Buffer,
}

impl Grid {
    pub fn reset(&mut self) {
        self.buf.reset();
    }

    pub fn resize(&mut self, grid_size: Rect<i32>) {
        self.buf.resize(rect_to_tui(grid_size));
    }

    pub fn rect(&self) -> Rect<i32> {
        Rect::new(
            self.buf.area.x.into(),
            self.buf.area.y.into(),
            self.buf.area.width.into(),
            self.buf.area.height.into(),
        )
    }

    pub fn cells(&self) -> &[tui_core::buffer::Cell] {
        &self.buf.content
    }

    pub fn draw_string_local<T, S>(&mut self, x: i32, y: i32, string: T, rect: Rect<i32>, style: S)
    where
        T: AsRef<str>,
        S: Into<Style>,
    {
        let x = x - rect.x;
        let y = y - rect.y;
        self.draw_string(x, y, string, rect, style);
    }

    pub fn draw_string<T, S>(&mut self, mut x: i32, y: i32, string: T, rect: Rect<i32>, style: S)
    where
        T: AsRef<str>,
        S: Into<Style>,
    {
        let string = string.as_ref();
        if y < rect.y || y >= rect.y + rect.height {
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
                && rect.contains(Vec2::new(x.into(), y.into()))
                && let Some(cell) = self.buf.cell_mut((x, y))
            {
                cell.set_symbol(symbol.as_str().expect("rope slice should be contiguous"))
                    .set_style(style);
            }
            let next_symbol = x + width as i32;
            x += 1;
            // Reset following cells if multi-width (they would be hidden by the grapheme)
            while x < next_symbol {
                if let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y))
                    && rect.contains(Vec2::new(x.into(), y.into()))
                    && let Some(cell) = self.buf.cell_mut((x, y))
                {
                    cell.reset();
                }
                x += 1;
            }

            // Early exit optimization when we have a small rect and very long string
            if x > rect.x + rect.width {
                break;
            }
        }
    }

    pub fn draw_style<S>(&mut self, rect: Rect<i32>, style: S)
    where
        S: Into<Style> + Copy,
    {
        //let rect = self.rect;
        for x in rect.x..rect.width {
            if x < 0 {
                continue;
            }
            for y in rect.y..rect.height {
                if y < 0 {
                    continue;
                }
                if let Some(cell) = self.buf.cell_mut((x as u16, y as u16)) {
                    cell.set_style(style);
                }
            }
        }
    }
}

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

pub struct Runtime {
    pub scale: f32,
    pub font_weight: u16,
    pub font_family: String,
    pub default_bg: Color,
    pub start_of_events: Instant,
    pub last_render_time: Duration,
    pub force_redraw: bool,
    // about
    pub drawing_backend: String,
    pub window_backend: String,
    pub window_title: String,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            font_weight: 400,
            font_family: "FiraCode Nerd Font Mono".into(),
            start_of_events: Instant::now(),
            last_render_time: Duration::ZERO,
            force_redraw: false,
            default_bg: Color::new(0.0, 0.0, 0.0),
            // about
            drawing_backend: String::from("unknown"),
            window_backend: String::from("unknown"),
            window_title: String::from("Ferrite"),
        }
    }
}

pub trait ImTuiApp {
    type UserEvent;
    fn start_of_frame(&mut self, runtime: &mut Runtime);
    fn ui(&mut self, runtime: &mut Runtime, ui: &mut Ui);
    fn event(&mut self, event: InputEvent<Self::UserEvent>);
}

// layout
enum Sizing {
    Fill,
    Min,
}

enum Alignment {
    Start,
    End,
    Center,
}

enum Axis {
    Vertical,
    Horizontal,
}

enum AxisDirection {
    Forward,
    Backward,
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

pub trait Interpolate {
    fn interpolate(&self, other: &Self) -> Self;
}

pub trait AnimationCurve {
    // takes a time and returns
    fn sample(&self, normalized_point_in_animation: f32) -> f32;
}

struct Animation {
    start_time: f32,
    duration: f32,
    state: f32,
}
