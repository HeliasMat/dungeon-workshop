//! What part of a place the view shows on a screen: within the place's
//! edges, a place smaller than the view in its middle.

use glam::Vec2;

/// The part of a place shown on a screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// The point of the place at the middle of the screen.
    pub centre: Vec2,
    /// How many pixels of the screen a pixel of the place takes.
    pub close: f32,
    /// The screen's size.
    pub screen: Vec2,
}

impl Viewport {
    /// The view looking at `focus` (none: the middle of the place), that
    /// close, kept in a place of that size: it never shows past an edge,
    /// and a place smaller than the view stays in its middle.
    pub fn of(focus: Option<Vec2>, close: f32, place: Vec2, screen: Vec2) -> Self {
        let close = if close <= 0.0 { 1.0 } else { close };
        let half = screen / 2.0 / close;
        let along = |at: f32, half: f32, size: f32| if size > half * 2.0 { at.clamp(half, size - half) } else { size / 2.0 };
        let focus = focus.unwrap_or(place / 2.0);
        Viewport { centre: Vec2::new(along(focus.x, half.x, place.x), along(focus.y, half.y, place.y)), close, screen }
    }

    /// Where a point of the place is on the screen.
    pub fn on_screen(&self, at: Vec2) -> Vec2 {
        self.screen / 2.0 + (at - self.centre) * self.close
    }

    /// The same view on a screen so many times larger.
    pub fn scaled(&self, scale: f32) -> Self {
        Viewport { close: self.close * scale, screen: self.screen * scale, ..*self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Vec2 = Vec2::new(256.0, 192.0);

    #[test]
    fn the_view_keeps_within_the_place() {
        let place = Vec2::new(1000.0, 1000.0);
        assert_eq!(Viewport::of(Some(Vec2::ZERO), 1.0, place, SCREEN).centre, Vec2::new(128.0, 96.0));
        assert_eq!(Viewport::of(Some(Vec2::splat(500.0)), 1.0, place, SCREEN).centre, Vec2::splat(500.0));
        // (Drawn back, it sees twice as far from each edge.)
        assert_eq!(Viewport::of(Some(Vec2::ZERO), 0.5, place, SCREEN).centre, Vec2::new(256.0, 192.0));
        // (A place smaller than the view: in its middle.)
        assert_eq!(Viewport::of(Some(Vec2::ZERO), 1.0, Vec2::new(100.0, 100.0), SCREEN).centre, Vec2::new(50.0, 50.0));
    }

    #[test]
    fn a_point_comes_on_the_screen_as_near_as_the_view_is() {
        let view = Viewport::of(Some(Vec2::splat(500.0)), 2.0, Vec2::splat(1000.0), SCREEN);
        assert_eq!(view.on_screen(Vec2::splat(500.0)), SCREEN / 2.0);
        assert_eq!(view.on_screen(Vec2::new(510.0, 500.0)), SCREEN / 2.0 + Vec2::new(20.0, 0.0));
        assert_eq!(view.scaled(4.0).on_screen(Vec2::splat(500.0)), SCREEN * 2.0);
    }
}
