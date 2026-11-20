//! What the player has to do with the place ([`PlayState`]): who it walks
//! about, where it may go, the walls.

use super::*;

/// What the player has to do with the place.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayState {
    /// Who the player walks about (see `PlayBeat::Control`), and whether it
    /// walks now as the player has it.
    pub controlled: Option<Who>,
    pub(super) steered: bool,
    /// Those the player may walk about in turn, Tab passing from one to
    /// the next (see `PlayBeat::ControlTeam`, [`Stage::swap_control`]).
    pub team: Vec<Who>,
    /// Keys counted with no one walked about (see `PlayBeat::Listen`).
    pub listening: bool,
    /// Where the one the player walks about may go: left, top, right,
    /// bottom (see `PlayBeat::Bounds`).
    pub bounds: Option<Rect>,
    /// The scene's walls: boxes (left, top, right, bottom) no one walked
    /// about goes into (see `PlayBeat::Wall`), and the name of each, if it
    /// has one (see `PlayBeat::NamedWall`): as long as each other, changed
    /// only by `put_wall`, `take_wall` and `clear_walls`.
    pub(crate) walls: Vec<Rect>,
    pub(crate) wall_names: Vec<Option<String>>,
    pub place_walls: Vec<Rect>,
}

impl PlayState {
    /// The scene's own walls (`wall`), to be shown.
    pub fn walls(&self) -> &[Rect] {
        &self.walls
    }
}

impl Stage {
    /// The walls of the place shown, as whoever shows it knows them: who
    /// steps aside keeps out of them. (Gone with the place. A scene's own,
    /// `wall`, are something else: they stop the player too.)
    pub fn place_walls(&mut self, walls: impl IntoIterator<Item = [f32; 4]>) {
        self.play.place_walls = walls.into_iter().map(Rect::from).collect();
    }

    /// A wall put up, by a name or not (see `PlayBeat::Wall`).
    pub(super) fn put_wall(&mut self, wall: Rect, name: Option<String>) {
        self.play.walls.push(wall);
        self.play.wall_names.push(name);
    }

    /// No wall of the scene's any more.
    pub(super) fn clear_walls(&mut self) {
        (self.play.walls, self.play.wall_names) = (Vec::new(), Vec::new());
    }

    /// The wall of that name taken away, if there is one.
    pub(super) fn take_wall(&mut self, name: &str) {
        let names = std::mem::take(&mut self.play.wall_names);
        let walls = std::mem::take(&mut self.play.walls);
        for (wall, named) in walls.into_iter().zip(names) {
            if named.as_deref() != Some(name) {
                self.play.walls.push(wall);
                self.play.wall_names.push(named);
            }
        }
    }

    /// Tab: the next of those the player may walk about in turn (see
    /// `PlayBeat::ControlTeam`) is walked about, the one before stopping where
    /// it is. Whether it changed.
    pub fn swap_control(&mut self) -> bool {
        let present: Vec<Who> = self.play.team.iter().copied().filter(|who| self.cast.contains_key(who)).collect();
        let Some(at) = self.play.controlled.and_then(|now| present.iter().position(|who| *who == now)).filter(|_| present.len() >= 2) else {
            return false;
        };
        if std::mem::take(&mut self.play.steered)
            && let Some(actor) = self.cast.get_mut(&present[at])
        {
            (actor.way.to, actor.way.left) = (actor.position, 0.0);
        }
        self.play.controlled = Some(present[(at + 1) % present.len()]);
        true
    }
}
