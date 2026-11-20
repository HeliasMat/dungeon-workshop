//! The place drawn, its lower layer and its upper one; a thing shown large;
//! a picture instead of the scene; a dungeon's name on a dark screen.

use engine::Cutaway;
use glam::Vec2;

use super::{BELOW, Brush, Play};
use crate::ui::{Canvas, SCREEN_HEIGHT, SCREEN_WIDTH, UiArt};
use shell::{Laid, Screen};

impl Play {
    /// The place's lower layer, or its upper one (tree crowns, roofs), at
    /// the frame its animation is on.
    pub(super) fn draw_place(&self, world: &mut Screen, brush: &Brush, upper: bool) {
        let Some(ground) = self.ground() else {
            return;
        };
        let (frames, laid) = if upper { (&ground.upper, Laid::SeeThrough) } else { (&ground.frames, Laid::Whole) };
        if frames.is_empty() {
            return;
        }
        let frame = (brush.seconds / ground.frame_seconds.max(0.01)) as usize % frames.len();
        world.layer(&frames[frame], ground.wide, ground.high, brush.view.centre, brush.large.close, laid);
    }

    /// A picture instead of the scene; the screen rippling takes it along.
    pub(super) fn draw_cutaway(&self, world: &mut Screen, sway: Vec2) {
        let Some(cutaway) = &self.sim.stage.light.cutaway else {
            return;
        };
        let scale = world.scale as f32;
        let lay = |world: &mut Screen, shown: &Cutaway, laid| {
            if let Some(Some((wide, high, rgba))) = self.pictures.get(&shown.level) {
                world.layer(rgba, *wide, *high, shown.at + sway, scale, laid);
            }
        };
        if let Some(before) = &cutaway.before {
            lay(world, before, Laid::Whole);
        }
        lay(world, cutaway, if cutaway.over { Laid::Light } else { Laid::Faded((cutaway.shown() * 255.0) as u32) });
    }

    /// The thing shown large: at the size of everything else on screen (its
    /// `_grand` picture, if it has one), over the scene, clear of the box.
    pub(super) fn draw_large(&self, world: &mut Screen) {
        let Some(who) = self.sim.stage.words.large else {
            return;
        };
        let large = self.large_sprites.get(&who).cloned().flatten();
        let frame = large.as_ref().and_then(|sprite| sprite.ways(0).first()).and_then(|steps| steps.first());
        let Some(frame) = frame.or_else(|| self.sim.shown.get(&who).and_then(|shown| shown.picture(0))) else {
            return;
        };
        let scale = world.scale as i32;
        let (x, y) = (SCREEN_WIDTH as i32 * scale / 2, SCREEN_HEIGHT as i32 * scale / 3);
        world.sprite(frame, (x, y), world.scale as f32, [255.0; 3], 1.0);
    }

    /// A dungeon's name on a dark screen, instead of a place.
    pub(super) fn draw_banner(&self, canvas: &mut Canvas, ui: &UiArt) {
        if let Some(banner) = &self.sim.banner {
            canvas.fill(0, BELOW, SCREEN_WIDTH, SCREEN_HEIGHT, [0, 0, 0, 255]);
            canvas.markup(ui, 0, BELOW + 88, SCREEN_WIDTH, &format!("[CN]{banner}"));
        }
    }
}
