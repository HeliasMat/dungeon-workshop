//! What is on screen of each node, as an agent reads it back without seeing
//! the picture ([`Framed`]): its rectangle in the view, whether it is seen,
//! how much of it the nodes over it and the dialogue box cover, its size in
//! pixels of the screen, how far it is from each edge.
//!
//! The rectangles are those that hold what is drawn (a body's: its size as
//! told, `ViewBeat::Body`, on its feet), turned, sized and slanted as its
//! node is, not its very pixels: a share covered is that of the rectangle.
//! Over what: the order it is drawn in ([`Stage::drawing_order`]), every
//! `z` told counted, the dialogue box at its own rank among them.

use super::*;
use crate::beat::NodeRef;
use crate::props::NodeKind;

/// A rectangle of the screen, in its pixels: from its top left, so wide and
/// high.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl ScreenRect {
    fn between(low: Vec2, high: Vec2) -> ScreenRect {
        ScreenRect { x: low.x, y: low.y, w: (high.x - low.x).max(0.0), h: (high.y - low.y).max(0.0) }
    }

    fn holds(&self, at: Vec2) -> bool {
        at.x >= self.x && at.x <= self.x + self.w && at.y >= self.y && at.y <= self.y + self.h
    }

    /// The part of it in another, if any (a point: itself, if it is in).
    fn within(&self, other: &ScreenRect) -> Option<ScreenRect> {
        let (low, high) = (
            Vec2::new(self.x.max(other.x), self.y.max(other.y)),
            Vec2::new((self.x + self.w).min(other.x + other.w), (self.y + self.h).min(other.y + other.h)),
        );
        (low.x <= high.x && low.y <= high.y).then(|| ScreenRect::between(low, high))
    }
}

/// How far a rectangle is from each edge of the screen (less than 0: past it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edges {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// A node as it is on screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Framed {
    pub node: NodeRef,
    pub kind: NodeKind,
    /// Its rectangle on the screen (none: it has no place of its own there,
    /// as the caption).
    pub rect: Option<ScreenRect>,
    /// Some of it is in the frame.
    pub in_frame: bool,
    /// In the frame, not hidden, and more than nothing of it shows.
    pub visible: bool,
    /// The share of what of it is in the frame that is covered: by the
    /// nodes over it, by the dialogue box; both (each point counted once).
    pub covered_by_nodes: f32,
    pub covered_by_box: f32,
    pub covered: f32,
    /// Its size on the screen, in pixels.
    pub size: Vec2,
    pub edges: Option<Edges>,
}

/// How many points a side a share covered is counted on.
const SAMPLES: usize = 12;

/// Where a node is in the order things are drawn in: its place in
/// `order`, if drawn there by itself; else just under the first drawn at a
/// higher rank than its own (its `z`, else its sheet's).
fn rank(stage: &Stage, order: &[(f32, Drawn)], node: &NodeRef, shown: &Node) -> f32 {
    let at = |drawn: Drawn| order.iter().position(|(_, one)| *one == drawn).map(|at| at as f32);
    let own = match node {
        NodeRef::Who(who) => at(Drawn::Body(*who)),
        NodeRef::Named(name) => stage.words.pictures.iter().position(|picture| picture.name == *name).and_then(|picture| at(Drawn::Picture(picture))),
        NodeRef::Halo(halo) => at(Drawn::Halo(usize::from(*halo))),
        NodeRef::Emitter(_) => at(Drawn::Sheet(Sheet::Particles)),
        NodeRef::Caption => at(Drawn::Sheet(Sheet::Caption)),
    };
    own.unwrap_or_else(|| {
        let z = shown.z.unwrap_or(match shown.kind {
            NodeKind::Body | NodeKind::Thing | NodeKind::Group => Layer::Among.z(),
            NodeKind::Picture => Sheet::Pictures.z(),
            NodeKind::Halo => Sheet::Halos.z(),
            NodeKind::Emitter => Sheet::Particles.z(),
            NodeKind::Caption => Sheet::Caption.z(),
        });
        order.iter().filter(|(rank, _)| *rank <= z).count() as f32 - 0.5
    })
}

impl Stage {
    /// Where a node is in the place, as a rectangle: its low and high
    /// corners (a point: both the same); none if it has no place of its own.
    /// `sizes`: a picture's size, if known (16 a side if not).
    fn held(&self, node: &NodeRef, shown: &Node, sizes: &dyn Fn(&NodeRef) -> Option<Vec2>) -> Option<(Vec2, Vec2)> {
        let pos = shown.transform.pos;
        // (Turned, sized and slanted about its pivot, from that point.)
        let placed = |from: Vec2, low: Vec2, high: Vec2| {
            let (low, high) = shown.transform.bounds(low, high);
            (from + low, from + high)
        };
        Some(match (node, shown.kind) {
            (NodeRef::Who(who), _) => {
                let actor = self.cast.get(who)?;
                let size = self.view.body(*who);
                // (From its feet, as high as it is off the ground; sunk, less of it.)
                let feet = pos - Vec2::new(0.0, actor.shown_height());
                let sunk = (actor.sunk.max(0.0) / shown.transform.scale.y.abs().max(f32::MIN_POSITIVE)).min(size.y);
                placed(feet, Vec2::new(-size.x / 2.0, sunk - size.y), Vec2::new(size.x / 2.0, 0.0))
            }
            (_, NodeKind::Picture) => {
                let size = sizes(node).unwrap_or(Vec2::splat(16.0));
                placed(pos, -size / 2.0, size / 2.0)
            }
            (NodeRef::Halo(rank), _) => {
                let radius = self.light.halos.get(usize::from(*rank))?.radius;
                (pos - Vec2::splat(radius), pos + Vec2::splat(radius))
            }
            (_, NodeKind::Caption) => return None,
            _ => (pos, pos),
        })
    }

    /// Every node as it is on screen in that view; `box_shown`: the dialogue
    /// box is there (it covers what is under it); `sizes`: a picture's
    /// size, if known.
    pub fn on_screen(&self, view: &Viewport, box_shown: bool, sizes: &dyn Fn(&NodeRef) -> Option<Vec2>) -> Vec<Framed> {
        let screen = ScreenRect { x: 0.0, y: 0.0, w: view.screen.x, h: view.screen.y };
        let high = BOX_HIGH * view.screen.y / SCREEN.y;
        let dialogue = match self.words.box_top {
            true => ScreenRect { x: 0.0, y: 0.0, w: screen.w, h: high },
            false => ScreenRect { x: 0.0, y: screen.h - high, w: screen.w, h: high },
        };
        let nodes: Vec<(NodeRef, Node)> = self.node_refs().into_iter().filter_map(|node| self.node(&node).map(|shown| (node, shown))).collect();
        let order = self.drawing_order(|_| false);
        let box_rank = order.iter().position(|(_, drawn)| *drawn == Drawn::Sheet(Sheet::Box)).map_or(f32::INFINITY, |at| at as f32);
        let placed: Vec<(Option<ScreenRect>, f32, bool)> = nodes
            .iter()
            .map(|(node, shown)| {
                let rect = self.held(node, shown, sizes).map(|(low, high)| ScreenRect::between(view.on_screen(low), view.on_screen(high)));
                // (What hides what is under it: a body, a thing, a picture, well seen.)
                let hides = matches!(shown.kind, NodeKind::Body | NodeKind::Thing | NodeKind::Picture) && shown.visible && shown.looks.opacity >= 0.5;
                (rect, rank(self, &order, node, shown), hides)
            })
            .collect();
        nodes
            .iter()
            .zip(&placed)
            .enumerate()
            .map(|(at, ((node, shown), (rect, rank, _)))| {
                let seen = rect.and_then(|rect| rect.within(&screen));
                let in_frame = seen.is_some() || (rect.is_none() && shown.kind == NodeKind::Caption);
                let visible = in_frame && shown.visible && shown.looks.opacity > 0.0;
                let over: Vec<ScreenRect> = placed
                    .iter()
                    .enumerate()
                    .filter(|(other_at, (_, other_rank, hides))| *other_at != at && *hides && other_rank > rank)
                    .filter_map(|(_, (other, ..))| *other)
                    .collect();
                // (The box covers only what is drawn under it.)
                let boxed = box_shown && *rank < box_rank;
                let (mut by_nodes, mut by_box, mut either, mut counted) = (0, 0, 0, 0);
                if let Some(seen) = seen {
                    for row in 0..SAMPLES {
                        for column in 0..SAMPLES {
                            let at = Vec2::new(seen.x + seen.w * (column as f32 + 0.5) / SAMPLES as f32, seen.y + seen.h * (row as f32 + 0.5) / SAMPLES as f32);
                            let (under_node, under_box) = (over.iter().any(|other| other.holds(at)), boxed && dialogue.holds(at));
                            counted += 1;
                            by_nodes += usize::from(under_node);
                            by_box += usize::from(under_box);
                            either += usize::from(under_node || under_box);
                        }
                    }
                }
                let share = |count: usize| if counted == 0 { 0.0 } else { count as f32 / counted as f32 };
                Framed {
                    node: node.clone(),
                    kind: shown.kind,
                    rect: *rect,
                    in_frame,
                    visible,
                    covered_by_nodes: share(by_nodes),
                    covered_by_box: share(by_box),
                    covered: share(either),
                    size: rect.map_or(Vec2::ZERO, |rect| Vec2::new(rect.w, rect.h)),
                    edges: rect.map(|rect| Edges { left: rect.x, top: rect.y, right: screen.w - (rect.x + rect.w), bottom: screen.h - (rect.y + rect.h) }),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> Viewport {
        Viewport { centre: Vec2::new(128.0, 96.0), close: 1.0, screen: SCREEN }
    }

    fn of(framed: &[Framed], who: Who) -> &Framed {
        framed.iter().find(|one| one.node == NodeRef::Who(who)).expect("framed")
    }

    #[test]
    fn a_body_is_its_size_on_its_feet_and_far_from_the_edges() {
        let mut stage = Stage::default();
        stage.put(Who::Monster(25), Vec2::new(128.0, 100.0), 1);
        stage.view.bodies.insert(Who::Monster(25), Vec2::new(24.0, 32.0));
        let framed = stage.on_screen(&view(), false, &|_| None);
        let pika = of(&framed, Who::Monster(25));
        assert_eq!(pika.rect, Some(ScreenRect { x: 116.0, y: 68.0, w: 24.0, h: 32.0 }));
        assert!(pika.in_frame && pika.visible && pika.covered == 0.0);
        assert_eq!(pika.size, Vec2::new(24.0, 32.0));
        assert_eq!(pika.edges, Some(Edges { left: 116.0, top: 68.0, right: 116.0, bottom: 92.0 }));
        // (Twice as close: twice as big on the screen.)
        let near = Viewport { close: 2.0, ..view() };
        assert_eq!(of(&stage.on_screen(&near, false, &|_| None), Who::Monster(25)).size, Vec2::new(48.0, 64.0));
    }

    #[test]
    fn off_the_frame_or_hidden_is_not_visible() {
        let mut stage = Stage::default();
        stage.put(Who::Monster(25), Vec2::new(600.0, 100.0), 1);
        stage.put(Who::Monster(133), Vec2::new(100.0, 100.0), 1);
        stage.cast.get_mut(&Who::Monster(133)).expect("there").attributes |= attribute::HIDDEN;
        let framed = stage.on_screen(&view(), false, &|_| None);
        let (far, hidden) = (of(&framed, Who::Monster(25)), of(&framed, Who::Monster(133)));
        assert!(!far.in_frame && !far.visible && far.edges.is_some_and(|edges| edges.right < 0.0));
        assert!(hidden.in_frame && !hidden.visible);
    }

    #[test]
    fn the_box_and_who_stands_in_front_cover() {
        let mut stage = Stage::default();
        // (Its feet at the screen's bottom: the box, 56 high, hides most of it.)
        stage.put(Who::Monster(25), Vec2::new(60.0, 192.0), 1);
        stage.put(Who::Monster(133), Vec2::new(200.0, 100.0), 1);
        stage.put(Who::Monster(7), Vec2::new(200.0, 101.0), 1);
        let framed = stage.on_screen(&view(), true, &|_| None);
        let low = of(&framed, Who::Monster(25));
        assert!(low.covered_by_box == 1.0 && low.covered_by_nodes == 0.0, "{low:?}");
        // (The one whose feet are lower is in front: the other wholly behind it.)
        let (behind, front) = (of(&framed, Who::Monster(133)), of(&framed, Who::Monster(7)));
        assert!(behind.covered_by_nodes > 0.9 && front.covered_by_nodes == 0.0, "{behind:?} {front:?}");
        assert_eq!(of(&stage.on_screen(&view(), false, &|_| None), Who::Monster(25)).covered, 0.0, "no box, nothing covers");
    }

    #[test]
    fn a_rank_told_counts_over_the_box_and_over_who_stands_in_front() {
        use crate::props::{Given, PropId, Value};
        let mut stage = Stage::default();
        stage.put(Who::Monster(25), Vec2::new(60.0, 192.0), 1);
        stage.put(Who::Monster(133), Vec2::new(200.0, 100.0), 1);
        stage.put(Who::Monster(7), Vec2::new(200.0, 101.0), 1);
        let z = |stage: &mut Stage, who: Who, z: f32| stage.set(&NodeRef::Who(who), PropId::Z, &Given::Is(Value::Number(z))).expect("set");
        // (Over the box: not covered by it; the one behind told in front.)
        z(&mut stage, Who::Monster(25), 400.0);
        z(&mut stage, Who::Monster(133), 5.0);
        let framed = stage.on_screen(&view(), true, &|_| None);
        assert_eq!(of(&framed, Who::Monster(25)).covered_by_box, 0.0);
        let (front, behind) = (of(&framed, Who::Monster(133)), of(&framed, Who::Monster(7)));
        assert!(front.covered_by_nodes == 0.0 && behind.covered_by_nodes > 0.9, "{front:?} {behind:?}");
    }

    #[test]
    fn turned_its_rectangle_is_the_box_that_holds_it() {
        use crate::props::{Given, PropId, Value};
        let mut stage = Stage::default();
        stage.put(Who::Monster(25), Vec2::new(128.0, 100.0), 1);
        stage.view.bodies.insert(Who::Monster(25), Vec2::new(24.0, 32.0));
        // (A quarter turn about its feet: lying to the left of them, as wide as it was high.)
        stage.set(&NodeRef::Who(Who::Monster(25)), PropId::Angle, &Given::Is(Value::Number(-90.0))).expect("set");
        let framed = stage.on_screen(&view(), false, &|_| None);
        let rect = of(&framed, Who::Monster(25)).rect.expect("a rect");
        assert!((rect.w - 32.0).abs() < 0.01 && (rect.h - 24.0).abs() < 0.01, "{rect:?}");
        assert!((rect.x - 96.0).abs() < 0.01 && (rect.y - 88.0).abs() < 0.01, "{rect:?}");
        // (Twice as wide one way.)
        stage.set(&NodeRef::Who(Who::Monster(25)), PropId::Angle, &Given::Is(Value::Number(0.0))).expect("set");
        stage.set(&NodeRef::Who(Who::Monster(25)), PropId::ScaleX, &Given::Is(Value::Number(2.0))).expect("set");
        let wide = of(&stage.on_screen(&view(), false, &|_| None), Who::Monster(25)).size;
        assert!((wide - Vec2::new(48.0, 32.0)).length() < 0.01, "{wide}");
    }

    #[test]
    fn a_picture_is_its_size_about_its_point() {
        let mut stage = Stage::default();
        stage.words.pictures.push(PlacedPicture { name: "moon".to_string(), file: "moon.png".to_string(), at: Vec2::new(50.0, 40.0) });
        let framed = stage.on_screen(&view(), false, &|_| Some(Vec2::new(20.0, 10.0)));
        let moon = framed.iter().find(|one| one.node == NodeRef::Named("moon".to_string())).expect("the picture");
        assert_eq!((moon.kind, moon.rect), (NodeKind::Picture, Some(ScreenRect { x: 40.0, y: 35.0, w: 20.0, h: 10.0 })));
    }
}
