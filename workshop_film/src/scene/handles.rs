//! The handles on the film's picture, each writing its line again when let
//! go: the camera's frame (Whole Place; moved, or grown by a corner), the
//! way the one chosen faces (eight dots around their feet), the ways they
//! walk in the moment (each point of them), the spots of the film.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, vec2};

use super::super::{Films, Frame, Held};
use super::script::{Move, WAYS, moves_of, spot_lines};
use super::{Place, View};
use workshop_base::theme;

/// How far from their feet the dots of someone's way are, in points.
const FACING_RING: f32 = 26.0;
/// How near a handle the pointer takes it, in points.
const NEAR: f32 = 7.0;

/// The handles there are on the picture now, on the panel.
#[derive(Default)]
pub(super) struct Handles {
    /// The camera's frame (the whole place shown).
    pub camera: Option<Rect>,
    /// The one chosen: their name, their feet, the way they face.
    pub facing: Option<(String, Pos2, Option<&'static str>)>,
    /// Their ways in the moment: where they stood before, each move's points.
    pub before: Option<Pos2>,
    pub moves: Vec<(Move, Vec<Pos2>)>,
    /// The spots of the film: their name, line, where.
    pub spots: Vec<(String, usize, Pos2)>,
}

/// The handles of the picture as it is: the line the edits are made from.
pub(super) fn handles(films: &Films, text: &str, here: usize, drawn: &Frame, view: &View) -> Handles {
    let place = Place::of(view, drawn);
    let mut handles = Handles { camera: drawn.camera.map(|camera| view.frame_of(camera)), ..Handles::default() };
    if let Some(who) = &films.focus_who
        && let Some(feet) = drawn.feet.get(who)
    {
        let faces = drawn.state.bodies.iter().find(|body| &body.name == who).map(|body| body.faces);
        handles.facing = Some((who.clone(), place.to_screen(*feet), faces));
        let (before, moves) = moves_of(text, here, who);
        handles.before = before.map(|point| place.to_screen(point));
        handles.moves = moves
            .into_iter()
            .map(|walked| {
                let points = walked.points.iter().map(|(_, point)| place.to_screen(*point)).collect();
                (walked, points)
            })
            .collect();
    }
    if films.names || films.whole {
        handles.spots = spot_lines(text).into_iter().map(|(name, line, point)| (name, line, place.to_screen(point))).collect();
    }
    handles
}

/// The dots of the eight ways around someone's feet, in the order of `WAYS`.
pub(super) fn facing_dots(feet: Pos2) -> [Pos2; 8] {
    std::array::from_fn(|at| {
        let angle = at as f32 * std::f32::consts::FRAC_PI_4;
        feet + vec2(angle.cos(), angle.sin()) * FACING_RING
    })
}

impl Handles {
    /// The handle at that point of the panel, as what a drag holds (the
    /// place's point under it given): the dots of a way, a point of a walk,
    /// a spot, the camera's corner, the camera's frame (not where someone is).
    pub fn at(&self, at: Pos2, point: [f32; 2], on_someone: bool) -> Option<Held> {
        if let Some((who, feet, _)) = &self.facing
            && facing_dots(*feet).iter().any(|dot| dot.distance(at) <= NEAR)
        {
            return Some(Held::Facing(who.clone()));
        }
        for (walked, points) in &self.moves {
            if let Some(index) = points.iter().position(|shown| shown.distance(at) <= NEAR) {
                return Some(Held::Point(walked.line, walked.points[index].0));
            }
        }
        if let Some((_, line, _)) = self.spots.iter().find(|(_, _, shown)| shown.distance(at) <= NEAR) {
            return Some(Held::Point(*line, 2));
        }
        let camera = self.camera?;
        if camera.max.distance(at) <= NEAR + 3.0
            || [camera.min, camera.right_top(), camera.left_bottom()].iter().any(|corner| corner.distance(at) <= NEAR + 3.0)
        {
            return Some(Held::Camera { from: point, corner: true });
        }
        (camera.contains(at) && !on_someone).then_some(Held::Camera { from: point, corner: false })
    }

    /// The way the one chosen faces, from a click on one of the dots.
    pub fn way_clicked(&self, at: Pos2) -> Option<(String, &'static str)> {
        let (who, feet, _) = self.facing.as_ref()?;
        let index = facing_dots(*feet).iter().position(|dot| dot.distance(at) <= NEAR)?;
        Some((who.clone(), WAYS[index]))
    }

    /// The pointer's look over a handle.
    pub fn cursor(&self, at: Pos2, on_someone: bool) -> Option<egui::CursorIcon> {
        match self.at(at, [0.0, 0.0], on_someone)? {
            Held::Camera { corner: true, .. } => Some(egui::CursorIcon::ResizeNwSe),
            Held::Camera { .. } => Some(egui::CursorIcon::Move),
            _ => Some(egui::CursorIcon::Grab),
        }
    }

    /// The handles drawn (what is held drawn by `held`).
    pub fn draw(&self, painter: &Painter, held: Option<&Held>) {
        if let Some(camera) = self.camera.filter(|_| !matches!(held, Some(Held::Camera { .. }))) {
            for corner in [camera.min, camera.right_top(), camera.left_bottom(), camera.max] {
                painter.rect_filled(Rect::from_center_size(corner, vec2(7.0, 7.0)), 0.0, theme::FILM);
            }
        }
        // (Each walk: a dotted line from where they stood, its points round, the last larger.)
        let mut from = self.before;
        for (walked, points) in &self.moves {
            let colour = if walked.runs { Color32::from_rgb(0xFF, 0x9A, 0x4A) } else { Color32::from_rgb(0x7F, 0xD8, 0xFF) };
            let way: Vec<Pos2> = from.into_iter().chain(points.iter().copied()).collect();
            painter.extend(Shape::dashed_line(&way, Stroke::new(1.5_f32, colour), 5.0, 4.0));
            for (at, point) in points.iter().enumerate() {
                let last = at + 1 == points.len();
                painter.circle(*point, if last { 5.0 } else { 3.5 }, colour, Stroke::new(1.0_f32, Color32::BLACK));
            }
            from = points.last().copied();
        }
        if let Some((_, feet, faces)) = &self.facing {
            for (at, dot) in facing_dots(*feet).iter().enumerate() {
                let now = *faces == Some(WAYS[at]);
                if now {
                    painter.line_segment([*feet, *dot], Stroke::new(2.0_f32, theme::FILM));
                }
                painter.circle(
                    *dot,
                    if now { 4.5 } else { 3.0 },
                    if now { theme::FILM } else { Color32::from_black_alpha(160) },
                    Stroke::new(1.0_f32, theme::FILM),
                );
            }
        }
    }
}

/// What a handle held shows while it is dragged: the camera's frame where
/// it would go, the way, a point.
pub(super) fn held(painter: &Painter, handles: &Handles, held: &Held, place: &Place, at: Pos2) {
    match held {
        Held::Camera { from, corner } => {
            let Some(camera) = handles.camera else { return };
            let moved = camera_moved(camera, *from, place, at, *corner);
            painter.rect_stroke(moved, 0.0, Stroke::new(2.0_f32, Color32::WHITE), StrokeKind::Inside);
            painter.rect_stroke(camera, 0.0, Stroke::new(1.0_f32, Color32::from_white_alpha(90)), StrokeKind::Inside);
        }
        Held::Facing(_) => {
            if let Some((_, feet, _)) = &handles.facing {
                painter.arrow(*feet, at - *feet, Stroke::new(2.0_f32, Color32::WHITE));
            }
        }
        Held::Point(..) => {
            painter.circle(at, 5.0, Color32::WHITE, Stroke::new(1.0_f32, Color32::BLACK));
            let point = place.to_place(at).map(f32::round);
            workshop_decor::tag(painter, at + vec2(8.0, 4.0), &format!("{} {}", point[0], point[1]), Color32::WHITE);
        }
        _ => {}
    }
}

/// The camera's frame on the panel after a drag to that point: moved by as
/// much as the pointer from where it was taken; by a corner, grown or
/// shrunk around its middle (its shape kept) to reach the pointer.
pub(super) fn camera_moved(camera: Rect, from: [f32; 2], place: &Place, at: Pos2, corner: bool) -> Rect {
    if corner {
        let (half, reach) = (camera.size() / 2.0, (at - camera.center()).abs());
        let grown = (reach.x / half.x.max(1.0)).max(reach.y / half.y.max(1.0)).clamp(0.2, 8.0);
        Rect::from_center_size(camera.center(), camera.size() * grown)
    } else {
        camera.translate(at - place.to_screen(from))
    }
}
