//! The pictures and sounds the ROM holds, decoded: a monster's or a thing's
//! animations, the effects, the title's, the items'.

use std::sync::Arc;

use rom_formats::mapbg::MapBackground;
use rom_formats::{self, wan::Wan};
use shell::audio;
use shell::images::Frame;
use stage_view::ui;

use super::{LANGUAGE, RomData, SOUND_SECONDS};

/// Animations of a monster or a thing, `[animation][way faced][step]`.
#[derive(Default)]
pub(crate) struct SpriteArt {
    pub animations: Vec<Vec<Vec<Frame>>>,
    /// For a monster: where its head, its two hands and its centre are,
    /// from its sprite's centre (y downwards), standing facing down.
    pub points: [(i32, i32); 4],
}

/// An effect of moves and scenes.
#[derive(Clone)]
pub(crate) struct EffectArt {
    /// Its pictures, laid out as a character's.
    pub sprite: Arc<SpriteArt>,
    /// How long it lasts, in 60ths of a second.
    pub ticks: f32,
    /// It goes on until it is removed, instead of playing once.
    pub repeats: bool,
    /// It covers the screen, whatever the view shows.
    pub screen: bool,
    pub sound: u16,
    /// The point of a monster it is shown on.
    pub point: u8,
}

impl RomData {
    /// The title screen's logo, a map of the title scene in the game's
    /// language: its first 256x192 pixels, as RGBA.
    /// The title scene's sprites in the game's language (the files' letter:
    /// a English, b French, c German, d Italian, e Spanish): the subtitle,
    /// then "Press Start".
    pub(crate) fn title_sprites(&self) -> Vec<ui::Picture> {
        let letter = title_letter();
        (1..=2)
            .filter_map(|n| {
                let wan = rom_formats::wan::Wan::parse(self.rom.file(&format!("GROUND/s05p02{letter}{n}.wan"))?).ok()?;
                let picture = wan.render(0)?;
                Some((picture.width, picture.height, picture.rgba))
            })
            .collect()
    }

    pub(crate) fn title_logo(&self) -> Option<Vec<u8>> {
        // "Donjon Mystère" (the French "Mystery Dungeon") is the second of the localised logos.
        let name = if LANGUAGE == 'f' { "s05p02c2" } else { "s05p02c3" };
        let entry = self.maps.iter().find(|entry| entry.bma == name)?;
        let map = MapBackground::load(&self.rom, entry).ok()?;
        let frame = &map.flat(0)?;
        let mut rgba = Vec::with_capacity(ui::SCREEN_WIDTH * ui::SCREEN_HEIGHT * 4);
        // The title scene shows the map 24 pixels lower (matched against
        // the game).
        // Known gap: what the game shows above it is not known; the map's first
        // rows are mirrored there.
        const SCROLL: usize = 24;
        for y in 0..ui::SCREEN_HEIGHT {
            // (Only its topmost rows, which hold no part of the logo.)
            let row = if y >= SCROLL { y - SCROLL } else { (SCROLL - 1 - y) % 12 };
            let at = row * map.width * 4;
            rgba.extend_from_slice(frame.get(at..at + ui::SCREEN_WIDTH * 4)?);
        }
        Some(rgba)
    }

    /// How an object of scenes looks (`GROUND/<name>.wan`), laid out as a
    /// character's sprite so that scenes play its animations the same way:
    /// its first animation where a character walks, then all of them by
    /// group. Also whether it lies on the ground: a good part of its
    /// picture is then below the point it is placed by (stone discs,
    /// marks), where things standing up have that point at their foot.
    pub(crate) fn prop(&self, object: u16) -> Option<(Arc<SpriteArt>, bool)> {
        let mut props = self.props.lock().ok()?;
        props
            .entry(object)
            .or_insert_with(|| {
                let name = self.objects.get(object as usize).filter(|name| !name.is_empty())?;
                let name = localised(name);
                let wan = Wan::parse(self.rom.file(&format!("GROUND/{name}.wan"))?).ok()?;
                let frame = |meta_frame: usize, duration: u8| {
                    let picture = wan.render(meta_frame)?;
                    Some(Frame {
                        ticks: duration as u32,
                        wide: picture.width as usize,
                        high: picture.height as usize,
                        origin: picture.origin,
                        shadow: (0, 0),
                        rgba: picture.rgba,
                    })
                };
                // Whatever way it is turned, a thing with one sequence
                // shows that one.
                let sequence = |group: usize, direction: usize| -> Vec<Frame> {
                    let shown = match wan.animation(group, direction) {
                        [] => wan.animation(group, 0),
                        found => found,
                    };
                    shown.iter().filter_map(|shown| frame(shown.meta_frame, shown.duration)).collect()
                };
                let group = |group: usize| -> Vec<Vec<Frame>> { (0..8).map(|direction| sequence(group, direction)).collect() };
                let mut first = group(0);
                if first.iter().all(|frames| frames.is_empty()) {
                    first = (0..8).map(|_| frame(0, u8::MAX).into_iter().collect()).collect();
                }
                let still = first.first()?.first()?;
                let below = still.high as i32 - still.origin.1;
                let flat = below * 5 > still.high as i32 * 2;
                // In the order of `shell::images::Anim` (a thing only "walks"),
                // then the animations of scenes.
                let mut animations = vec![first, Vec::new(), Vec::new(), Vec::new(), Vec::new()];
                animations.extend((0..wan.group_sizes().len()).map(group));
                Some((Arc::new(SpriteArt { animations, ..Default::default() }), flat))
            })
            .clone()
    }

    /// How long a sound (or, `jingle`, a jingle) lasts, in 60ths of a
    /// second; one that goes on, a few seconds.
    pub(crate) fn sound_frames(&self, id: u16, jingle: bool) -> f32 {
        const LONGEST: f32 = 60.0 * 4.0;
        let Ok(mut lengths) = self.sound_lengths.lock() else {
            return 0.0;
        };
        *lengths.entry((id, jingle)).or_insert_with(|| {
            let samples = self.render_sound(id, jingle).map_or(0, |wav| wav.len().saturating_sub(44) / 2);
            (samples as f32 / audio::OUTPUT_RATE as f32 * 60.0).min(LONGEST)
        })
    }

    /// A scene animation by its number in the game's table: the group of
    /// the sprite's animations it plays, whether it goes round, whether it
    /// holds its first picture.
    pub(crate) fn scene_animation(&self, number: u16) -> Option<(usize, bool, bool)> {
        let animations = self.scene_animations.get_or_init(|| rom_formats::script::load_scene_animations(&self.rom).unwrap_or_default());
        let animation = animations.get((number & 0xFF) as usize)?;
        Some((animation.group as usize, animation.looped, animation.frozen))
    }

    /// The rain: its pictures side by side over twice the screen each way
    /// (so that it is covered wherever the view is held back by the edge
    /// of a map), one after the other.
    // Known gap: how long each picture stays and how much the scene shows
    // through are by eye; the original was not looked at.
    pub(crate) fn rain(&self) -> Option<EffectArt> {
        const TICKS: u8 = 2;
        const SEEN_THROUGH: u8 = 176;
        let (wide, high) = (ui::SCREEN_WIDTH * 2, ui::SCREEN_HEIGHT * 2);
        let pictures = rom_formats::script::load_rain(&self.rom)?;
        let frames: Vec<Frame> = pictures
            .iter()
            .filter(|(width, height, _)| *width > 0 && *height > 0)
            .map(|(width, height, picture)| {
                let mut rgba = vec![0u8; wide * high * 4];
                for (index, pixel) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                    let from = (index / wide % height * width + index % wide % width) * 4;
                    if picture[from + 3] != 0 {
                        pixel[..3].copy_from_slice(&picture[from..from + 3]);
                        pixel[3] = SEEN_THROUGH;
                    }
                }
                Frame { ticks: TICKS as u32, wide, high, origin: (wide as i32 / 2, high as i32 / 2), shadow: (0, 0), rgba }
            })
            .collect();
        let ticks = frames.len() as f32 * TICKS as f32;
        let sprite = Arc::new(SpriteArt { animations: vec![vec![frames]], ..Default::default() });
        (ticks > 0.0).then_some(EffectArt { sprite, ticks, repeats: true, screen: true, sound: 0, point: 255 })
    }

    /// Animations of a species. Both sprite packs are indexed by entity id
    /// (checked by eye on a few species of the EU ROM). `monster.bin` holds
    /// the walk (group 0), sleep (5), hurt (6) and idle (7) animations,
    /// `m_attack.bin` the attacks (group 1 is used for every attack for now).
    /// With `scenes`, the animations scripts play in the overworld follow
    /// the five above, in the order of their groups: those of
    /// `m_ground.bin`, or of `monster.bin` for the groups it lacks.
    pub(crate) fn sprite_with(&self, md_index: usize, scenes: bool) -> Option<Arc<SpriteArt>> {
        // The species says which sprite it uses; it is not its entity id.
        let sprite_index = self.monsters.species(md_index)?.sprite_index as usize;
        let mut sprites = self.sprites.lock().ok()?;
        sprites
            .entry((sprite_index, scenes))
            .or_insert_with(|| {
                let frames = |wan: &Wan, group: usize| -> Vec<Vec<Frame>> {
                    (0..8)
                        .map(|direction| {
                            wan.animation(group, direction)
                                .iter()
                                .filter_map(|frame| {
                                    let picture = wan.render(frame.meta_frame)?;
                                    // (A frame's offset moves the whole picture: an
                                    // attack's lunge towards its target.)
                                    let origin = (picture.origin.0 - frame.offset.0 as i32, picture.origin.1 - frame.offset.1 as i32);
                                    Some(Frame {
                                        ticks: frame.duration as u32,
                                        wide: picture.width,
                                        high: picture.height,
                                        origin,
                                        shadow: (frame.shadow.0 as i32, frame.shadow.1 as i32),
                                        rgba: picture.rgba,
                                    })
                                })
                                .collect()
                        })
                        .collect()
                };
                let wan = Wan::from_pack(self.rom.file("MONSTER/monster.bin")?, sprite_index).ok()?;
                // In the order of `shell::images::Anim`: walk, idle, hurt, sleep, attack.
                let mut animations: Vec<_> = [0, 7, 6, 5].iter().map(|&g| frames(&wan, g)).collect();
                let attack = self.rom.file("MONSTER/m_attack.bin").and_then(|pack| Wan::from_pack(pack, sprite_index).ok());
                animations.push(attack.as_ref().map_or(Vec::new(), |attack| frames(attack, 1)));
                // Then, by group: a scene's own (`m_ground`, for who stands
                // in scenes), else a move's (`m_attack`), else the plain
                // sprite's (for scenes again).
                let ground = self.rom.file("MONSTER/m_ground.bin").filter(|_| scenes).and_then(|pack| Wan::from_pack(pack, sprite_index).ok());
                let sizes = |wan: Option<&Wan>| wan.map_or(0, |wan| wan.group_sizes().len());
                let groups = sizes(ground.as_ref()).max(sizes(attack.as_ref())).max(if scenes { sizes(Some(&wan)) } else { 0 });
                for group in 0..groups {
                    let pick = |from: Option<&Wan>| from.map(|from| frames(from, group)).filter(|own| own.iter().any(|frames| !frames.is_empty()));
                    let plain = scenes.then_some(&wan);
                    animations.push(pick(ground.as_ref()).or_else(|| pick(attack.as_ref())).or_else(|| pick(plain)).unwrap_or_default());
                }
                // Its head, hands and centre, standing facing down.
                const IDLE_GROUP: usize = 7;
                let first = wan.animation(IDLE_GROUP, 0).first().or_else(|| wan.animation(0, 0).first());
                let points = first.and_then(|frame| wan.offsets(frame.meta_frame)).map_or([(0, 0); 4], |points| points.map(|(x, y)| (x as i32, y as i32)));
                Some(Arc::new(SpriteArt { animations, points }))
            })
            .clone()
    }

    /// Renders a sound of the game, or a jingle of `SOUND/ME`, to an
    /// in-memory WAV file.
    pub(crate) fn render_sound(&self, id: u16, jingle: bool) -> Option<Vec<u8>> {
        let pcm = match jingle {
            true => self.library().jingle(id, SOUND_SECONDS),
            false => self.library().sound(id, SOUND_SECONDS),
        };
        Some(audio::wav_bytes(&pcm?))
    }

    /// 16x16 RGBA picture of every item, by item id.
    pub(crate) fn item_art(&self) -> Vec<Vec<u8>> {
        let graphics = self.rom.file("DUNGEON/dungeon.bin").ok_or(rom_formats::RomError::Truncated).and_then(rom_formats::items::ItemGraphics::load);
        match graphics {
            Ok(graphics) => (0..self.items.item_count())
                .map(|id| {
                    let (picture, palette) = self.items.sprite(id).unwrap_or_default();
                    graphics.render(picture, palette).unwrap_or_default()
                })
                .collect(),
            Err(e) => {
                eprintln!("item graphics unavailable: {e}");
                Vec::new()
            }
        }
    }

    pub(crate) fn effect(&self, id: u16) -> Option<EffectArt> {
        let mut effects = self.effects.lock().ok()?;
        effects
            .entry(id)
            .or_insert_with(|| {
                if rom_formats::script::effect_is_rain(&self.rom, id) {
                    return self.rain();
                }
                let effect = rom_formats::script::load_effect(&self.rom, id)?;
                let pack = self.rom.file("EFFECT/effect.bin")?;
                let base = Wan::from_pack(pack, rom_formats::wan::EFFECT_BASE).ok()?;
                let wan = Wan::from_pack(pack, effect.file).ok()?.with_base(&base);
                let shown = wan.animation(0, effect.animation);
                let frames = |shown: &[rom_formats::wan::AnimFrame]| -> Vec<Frame> {
                    shown
                        .iter()
                        .filter_map(|frame| {
                            let picture = wan.render(frame.meta_frame)?;
                            Some(Frame {
                                ticks: frame.duration as u32,
                                wide: picture.width as usize,
                                high: picture.height as usize,
                                origin: picture.origin,
                                shadow: (0, 0),
                                rgba: picture.rgba,
                            })
                        })
                        .collect()
                };
                let ticks: f32 = shown.iter().map(|frame| frame.duration as f32).sum();
                // Drawn as a character's sprite: one direction, or eight
                // when the file's animations go by eights (the original's
                // effects then play the one the move faces, after the
                // effect's own: overlay 10, as a live effect starts).
                let directions = match wan.group_sizes().first().is_some_and(|&size| size > 0 && size % 8 == 0) {
                    true => (0..8).map(|direction| frames(wan.animation(0, effect.animation + direction))).collect(),
                    false => vec![frames(shown)],
                };
                let sprite = Arc::new(SpriteArt { animations: vec![directions], ..Default::default() });
                let sound = rom_formats::script::effect_sound(&self.rom, id).unwrap_or_default();
                (ticks > 0.0).then_some(EffectArt { sprite, ticks, repeats: effect.repeats, screen: false, sound, point: effect.point })
            })
            .clone()
    }
}

/// The letter of the title scene's files in the game's language (a English,
/// b French, c German, d Italian, e Spanish).
fn title_letter() -> char {
    match LANGUAGE {
        'f' => 'b',
        'g' => 'c',
        'i' => 'd',
        's' => 'e',
        _ => 'a',
    }
}

/// An object of the title scene in the game's language: its scripts name the
/// English one (`s05p02a1`, the subtitle; `s05p02a2`, "Press Start") and the
/// game takes its language's.
fn localised(name: &str) -> String {
    match name.strip_prefix("s05p02a") {
        Some(rest) => format!("s05p02{}{rest}", title_letter()),
        None => name.to_string(),
    }
}
