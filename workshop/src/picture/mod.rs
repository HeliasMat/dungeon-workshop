//! Pictures and animations, made in the Resources space, painted as in a pixel-art studio:
//! - layers (seen or not, how opaque, moved, doubled, merged down), each with
//!   a picture for each step of the animation (a cel);
//! - tools: pencil (its size, pixel-perfect strokes), eraser, fill (what
//!   touches, or all of a colour), picker, line, rectangle and ellipse
//!   (outlined or filled), selection, move, shade (darker, lighter), dither;
//!   mirrors on both axes; a primary and a secondary colour (left and right
//!   buttons, X swaps them), the colours used last, a palette of one's own,
//!   the game's;
//! - a selection: cut, copied, pasted (a picture dropped on the window too),
//!   moved, flipped, turned, cleared; painting stays inside it;
//! - the canvas: zoomed around the pointer, moved (middle button, or Space
//!   held), its grid, the steps before and after seen through (onion skin),
//!   the brush shown where it would paint;
//! - the steps: a strip of them under the canvas, each its time, played
//!   round or back and forth, faster or slower; seen at their true size;
//! - every change undone and redone by its name;
//! - written as the things drawn are (`name.png`, `name.4.png`: four steps
//!   side by side), its layers kept beside to be painted again.
//!
//! Here: the tools,
//! the game's palette, the layers, and the picture being painted with its
//! first workings (its steps, its cels, its changes undone and redone).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use eframe::egui::{self, Color32, Vec2};
use image::{Rgba, RgbaImage};

use files::{picture_expert_file, read_layers, read_ticks};
use pixels::composite;

mod canvas;
mod editor;
mod effects;
mod files;
mod panels;
mod pixels;
mod pointer;
mod selection;
mod simple;
mod steps;
mod strokes;
#[cfg(test)]
mod tests;

pub(crate) use canvas::fitted;
pub use editor::paint;

/// What paints.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tool {
    Pencil,
    Eraser,
    Fill,
    Picker,
    Line,
    Rectangle,
    Ellipse,
    Select,
    Move,
    Shade,
    Dither,
}

impl Tool {
    const ALL: [(Tool, &'static str, egui::Key, &'static str); 11] = [
        (Tool::Pencil, "Pencil", egui::Key::B, "B"),
        (Tool::Eraser, "Eraser", egui::Key::E, "E"),
        (Tool::Fill, "Bucket", egui::Key::G, "G"),
        (Tool::Picker, "Eyedropper", egui::Key::I, "I"),
        (Tool::Line, "Line", egui::Key::L, "L"),
        (Tool::Rectangle, "Rectangle", egui::Key::R, "R"),
        (Tool::Ellipse, "Ellipse", egui::Key::O, "O"),
        (Tool::Select, "Selection", egui::Key::M, "M"),
        (Tool::Move, "Move", egui::Key::V, "V"),
        (Tool::Shade, "Shade", egui::Key::S, "S"),
        (Tool::Dither, "Dither", egui::Key::D, "D"),
    ];

    fn about(self) -> &'static str {
        match self {
            Tool::Pencil => "click: the primary colour; right-click: the secondary",
            Tool::Eraser => "erases (makes transparent)",
            Tool::Fill => "fills what touches of the same colour (or all of that colour: \"Everywhere\")",
            Tool::Picker => "takes the colour under the pointer (Alt+click with another tool too)",
            Tool::Line => "a line from one point to another",
            Tool::Rectangle => "a rectangle (filled if \"Filled\")",
            Tool::Ellipse => "an ellipse (filled if \"Filled\")",
            Tool::Select => "chooses a rectangle: painting stays inside; Ctrl+C, Ctrl+X, Delete",
            Tool::Move => "moves the selection (or the whole layer); arrows: by a pixel (Shift: by 8)",
            Tool::Shade => "click: darker; right-click: lighter (once per stroke)",
            Tool::Dither => "a checkerboard of the primary and secondary colours",
        }
    }
}

/// The game's colours, as its pictures have them: its outline, its skins,
/// its earths, its leaves, its waters, its flames.
const PALETTE: [[u8; 3]; 32] = [
    [0x18, 0x10, 0x10],
    [0x40, 0x38, 0x38],
    [0x78, 0x70, 0x70],
    [0xB0, 0xA8, 0xA8],
    [0xE8, 0xE0, 0xD8],
    [0xFF, 0xFF, 0xFF],
    [0x58, 0x30, 0x18],
    [0x90, 0x58, 0x28],
    [0xC8, 0x90, 0x50],
    [0xF0, 0xC8, 0x88],
    [0xF8, 0xE8, 0xB8],
    [0xA0, 0x18, 0x18],
    [0xE0, 0x38, 0x30],
    [0xF8, 0x80, 0x58],
    [0xF8, 0xB0, 0x30],
    [0xF8, 0xE0, 0x38],
    [0x20, 0x50, 0x28],
    [0x38, 0x88, 0x38],
    [0x70, 0xC0, 0x48],
    [0xB0, 0xE0, 0x78],
    [0x18, 0x38, 0x78],
    [0x28, 0x68, 0xC8],
    [0x58, 0xA8, 0xF0],
    [0xA0, 0xD8, 0xF8],
    [0x48, 0x28, 0x78],
    [0x80, 0x50, 0xC0],
    [0xC0, 0x90, 0xE8],
    [0xD8, 0x58, 0x98],
    [0xF8, 0xA0, 0xC8],
    [0x60, 0x48, 0x38],
    [0x98, 0x80, 0x68],
    [0xD0, 0xC0, 0xA0],
];

/// A layer: its name, seen or not, how opaque, its picture for each step.
#[derive(Clone)]
struct Layer {
    name: String,
    visible: bool,
    opacity: u8,
    /// How it is laid on those under it: as it is, multiplied, added, screened.
    blend: u8,
    cels: Vec<RgbaImage>,
}

/// How a layer may be laid on those under it.
const BLENDS: [&str; 4] = ["Normal", "Multiply", "Add", "Screen"];

/// What a change leaves to be undone: its name, the layers and the steps' times as they were.
#[derive(Clone)]
struct Kept {
    said: String,
    layers: Vec<Layer>,
    ticks: Vec<u8>,
}

/// A picture (or an animation) being painted.
pub struct Painting {
    /// What the Workshop works with (`session`).
    pub session: std::sync::Arc<crate::session::Session>,
    pub word: String,
    layers: Vec<Layer>,
    layer: usize,
    at: usize,
    ticks: Vec<u8>,
    tool: Tool,
    brush: u32,
    perfect: bool,
    filled: bool,
    everywhere: bool,
    primary: Color32,
    secondary: Color32,
    recent: Vec<Color32>,
    palette: Vec<Color32>,
    zoom: f32,
    /// The zoom made to fit the room it has (the window grown, it grows), until zoomed by hand.
    zoom_fits: bool,
    pan: Vec2,
    grid: bool,
    onion: bool,
    mirror_x: bool,
    mirror_y: bool,
    /// A rectangle chosen (its corners, both in); what is lifted from it, and where it is.
    selection: Option<[i32; 4]>,
    floating: Option<(RgbaImage, (i32, i32))>,
    clipboard: Option<RgbaImage>,
    undone: Vec<Kept>,
    redone: Vec<Kept>,
    playing: bool,
    back_and_forth: bool,
    speed: f32,
    began: Instant,
    textures: HashMap<(usize, usize), egui::TextureHandle>,
    composites: HashMap<usize, egui::TextureHandle>,
    floating_texture: Option<egui::TextureHandle>,
    file: Option<PathBuf>,
    size: (u32, u32),
    pub(crate) saved: bool,
    said: Option<String>,
    /// A stroke: where the pointer was, where a shape began, the pixels laid (to be pixel-perfect,
    /// to shade once), whether with the right button; a move's start.
    last: Option<(i32, i32)>,
    from: Option<(i32, i32)>,
    trail: Vec<((i32, i32), Rgba<u8>)>,
    shaded: HashSet<(i32, i32)>,
    secondary_down: bool,
    moving: Option<(i32, i32)>,
    naming_layer: Option<usize>,
    /// Painted as an expert, or simply (kept from one time to the next).
    expert: bool,
    /// An expert's: the picture tiled round it; how many steps before and after are seen through, how much;
    /// its colours adjusted (hue, saturation, light, contrast), on every step or this one; turned and scaled; so many colours at most.
    tiled: bool,
    onion_range: usize,
    onion_strength: f32,
    adjust: [f32; 4],
    adjust_all: bool,
    turn: f32,
    scale: f32,
    colours_at_most: usize,
    /// A beginner's: how fast the steps go (from slow to fast).
    pace: f32,
}

impl Painting {
    /// A new one: its word, its size, how many steps.
    pub fn new(session: &std::sync::Arc<crate::session::Session>, word: &str, wide: u32, high: u32, count: usize) -> Painting {
        let layer = Layer { name: "Layer 1".to_string(), visible: true, opacity: 255, blend: 0, cels: vec![RgbaImage::new(wide, high); count.max(1)] };
        Painting::made(session, word, vec![layer], None)
    }

    /// One of the things drawn, read to be painted again (its layers, if they were kept).
    pub fn open(session: &std::sync::Arc<crate::session::Session>, word: &str, file: &std::path::Path, count: usize) -> Option<Painting> {
        let whole = image::open(file).ok()?.to_rgba8();
        let count = count.max(1) as u32;
        let wide = whole.width() / count;
        let cut = |strip: &RgbaImage| -> Vec<RgbaImage> {
            (0..count).map(|at| image::imageops::crop_imm(strip, at * wide, 0, wide, strip.height()).to_image()).collect()
        };
        let layers = read_layers(session.root(), word, whole.width(), whole.height()).map(|kept| {
            kept.into_iter().map(|(name, visible, opacity, blend, strip)| Layer { name, visible, opacity, blend, cels: cut(&strip) }).collect::<Vec<_>>()
        });
        let layers = layers.unwrap_or_else(|| vec![Layer { name: "Layer 1".to_string(), visible: true, opacity: 255, blend: 0, cels: cut(&whole) }]);
        let mut painting = Painting::made(session, word, layers, Some(file.to_path_buf()));
        if let Some(ticks) = read_ticks(session.root(), word).filter(|ticks| ticks.len() == painting.count()) {
            painting.ticks = ticks;
        }
        painting.saved = true;
        Some(painting)
    }

    fn made(session: &std::sync::Arc<crate::session::Session>, word: &str, layers: Vec<Layer>, file: Option<PathBuf>) -> Painting {
        let count = layers[0].cels.len();
        let size = (layers[0].cels[0].width(), layers[0].cels[0].height());
        let zoom = (360.0 / size.0.max(size.1) as f32).floor().clamp(2.0, 32.0);
        Painting {
            session: session.clone(),
            word: word.to_string(),
            layers,
            layer: 0,
            at: 0,
            ticks: vec![8; count],
            tool: Tool::Pencil,
            brush: 1,
            perfect: true,
            filled: false,
            everywhere: false,
            primary: Color32::from_rgb(0x18, 0x10, 0x10),
            secondary: Color32::WHITE,
            recent: Vec::new(),
            palette: PALETTE.iter().map(|[r, g, b]| Color32::from_rgb(*r, *g, *b)).collect(),
            zoom,
            zoom_fits: true,
            pan: Vec2::ZERO,
            grid: true,
            onion: count > 1,
            mirror_x: false,
            mirror_y: false,
            selection: None,
            floating: None,
            clipboard: None,
            undone: Vec::new(),
            redone: Vec::new(),
            playing: false,
            back_and_forth: false,
            speed: 1.0,
            began: Instant::now(),
            textures: HashMap::new(),
            composites: HashMap::new(),
            floating_texture: None,
            file,
            size,
            saved: false,
            said: None,
            last: None,
            from: None,
            trail: Vec::new(),
            shaded: HashSet::new(),
            secondary_down: false,
            moving: None,
            naming_layer: None,
            expert: std::fs::read_to_string(picture_expert_file(session.root())).is_ok_and(|said| said.trim() == "1"),
            tiled: false,
            onion_range: 1,
            onion_strength: 0.3,
            adjust: [0.0; 4],
            adjust_all: true,
            turn: 0.0,
            scale: 1.0,
            colours_at_most: 8,
            pace: 0.5,
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.layers[0].cels.len()
    }

    fn wide(&self) -> i32 {
        self.layers[0].cels[0].width() as i32
    }

    fn high(&self) -> i32 {
        self.layers[0].cels[0].height() as i32
    }

    fn cel(&mut self) -> &mut RgbaImage {
        let (layer, at) = (self.layer, self.at);
        &mut self.layers[layer].cels[at]
    }

    /// Kept before a change, by its name (undone later).
    fn keep(&mut self, said: &str) {
        self.undone.push(Kept { said: said.to_string(), layers: self.layers.clone(), ticks: self.ticks.clone() });
        if self.undone.len() > 100 {
            self.undone.remove(0);
        }
        self.redone.clear();
        self.saved = false;
    }

    fn changed(&mut self, layer: usize, at: usize) {
        self.textures.remove(&(layer, at));
        self.composites.remove(&at);
        self.saved = false;
    }

    fn all_changed(&mut self) {
        self.textures.clear();
        self.composites.clear();
        self.saved = false;
    }

    fn undo(&mut self) {
        if let Some(kept) = self.undone.pop() {
            self.redone.push(Kept {
                said: kept.said.clone(),
                layers: std::mem::replace(&mut self.layers, kept.layers),
                ticks: std::mem::replace(&mut self.ticks, kept.ticks),
            });
            self.settle();
            self.said = Some(format!("undone: {}", kept.said));
        }
    }

    fn redo(&mut self) {
        if let Some(kept) = self.redone.pop() {
            self.undone.push(Kept {
                said: kept.said.clone(),
                layers: std::mem::replace(&mut self.layers, kept.layers),
                ticks: std::mem::replace(&mut self.ticks, kept.ticks),
            });
            self.settle();
            self.said = Some(format!("redone: {}", kept.said));
        }
    }

    /// The layer and the step shown kept within what there is.
    fn settle(&mut self) {
        self.layer = self.layer.min(self.layers.len() - 1);
        self.at = self.at.min(self.count() - 1);
        self.ticks.resize(self.count(), 8);
        self.size = (self.wide() as u32, self.high() as u32);
        self.all_changed();
    }

    fn texture(&mut self, context: &egui::Context, layer: usize, at: usize) -> egui::TextureHandle {
        if let Some(texture) = self.textures.get(&(layer, at)) {
            return texture.clone();
        }
        let cel = &self.layers[layer].cels[at];
        let image = egui::ColorImage::from_rgba_unmultiplied([cel.width() as usize, cel.height() as usize], cel.as_raw());
        let texture = context.load_texture(format!("cel {layer} {at}"), image, egui::TextureOptions::NEAREST);
        self.textures.insert((layer, at), texture.clone());
        texture
    }

    fn composite_texture(&mut self, context: &egui::Context, at: usize) -> egui::TextureHandle {
        if let Some(texture) = self.composites.get(&at) {
            return texture.clone();
        }
        let made = composite(&self.layers, at);
        let image = egui::ColorImage::from_rgba_unmultiplied([made.width() as usize, made.height() as usize], made.as_raw());
        let texture = context.load_texture(format!("image {at}"), image, egui::TextureOptions::NEAREST);
        self.composites.insert(at, texture.clone());
        texture
    }
}
