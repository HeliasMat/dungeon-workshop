//! The beats of the nodes: a property set, a node of the film's own made or
//! taken away.

use engine::props::{Given, PropId};
use engine::{Beat, NodeBeat, NodeRef, Point, Sort};

/// A property of a node set, as written (`prop a angle 45`).
pub fn set_prop(node: NodeRef, prop: PropId, value: Given<NodeRef>) -> Beat {
    Beat::Node(NodeBeat::Set { node, prop, value })
}

/// A node of the film's own, at a point (`node moon image moon.png 40 30`).
pub fn make(name: impl Into<String>, sort: Sort, at: Point) -> Beat {
    Beat::Node(NodeBeat::Make { name: name.into(), sort, at })
}

/// A node of the film's own taken away.
pub fn unmake(name: impl Into<String>) -> Beat {
    Beat::Node(NodeBeat::Unmake(name.into()))
}

/// `attach a b [dx dy] [inherit ...]`.
pub fn attach(node: NodeRef, to: NodeRef, offset: Option<glam::Vec2>, inherit: engine::node::Inherit) -> Beat {
    Beat::Node(NodeBeat::Attach { node, to, offset, inherit })
}

/// A track of a node's property (`anim`, `keys`).
pub fn animate(node: NodeRef, prop: PropId, keys: Vec<engine::Key>, mode: engine::TrackMode, relative: bool, exact: bool) -> Beat {
    Beat::Node(NodeBeat::Animate { node, prop, keys, mode, relative, exact })
}

/// A node along a path (`follow_route`).
#[allow(clippy::too_many_arguments)]
pub fn follow_path(
    node: NodeRef,
    points: Vec<glam::Vec2>,
    shape: engine::PathShape,
    frames: f32,
    ease: engine::Ease,
    mode: engine::TrackMode,
    orient: bool,
    relative: bool,
) -> Beat {
    Beat::Node(NodeBeat::Path { node, points, shape, frames: engine::Frames(frames), ease, mode, orient, relative })
}

/// A node's tracks stopped (`cease anim a [prop]`).
pub fn stop_tracks(node: NodeRef, prop: Option<PropId>) -> Beat {
    Beat::Node(NodeBeat::Stop { node, prop })
}

/// The film waits for a node's tracks (`anim` without `&`).
pub fn wait_track(node: NodeRef) -> Beat {
    Beat::Flow(engine::FlowBeat::WaitTrack(node))
}

/// `detach a`.
pub fn detach(node: NodeRef) -> Beat {
    Beat::Node(NodeBeat::Detach(node))
}
