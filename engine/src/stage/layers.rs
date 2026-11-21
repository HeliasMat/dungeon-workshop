//! In what order a scene is drawn: the one account of it, for whoever draws
//! the stage. The picture is made of [`SHEETS`], from the furthest back;
//! among the bodies, each has its [`Layer`] and its depth.
//!
//! Each is drawn at its rank, a number, `z` ([`Sheet::z`], [`Layer::z`]):
//! [`Stage::drawing_order`] gives every sheet, every body and thing, every
//! picture and every halo, sorted by it, those of the same rank as they
//! come (the sheets' order, the bodies by their feet). Nothing told, it is
//! today's order; a node told a `z` (`prop a z 400`) goes where its number
//! puts it, over the dialogue box if it is higher ([`Sheet::Box`]).

use super::*;

/// A sheet of a scene's picture. Whoever draws goes through [`SHEETS`] in
/// their order and draws each; it keeps no order of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sheet {
    /// The place's lower layer, with its animations and palette cycles.
    Place,
    /// Its upper layer (tree crowns, roofs): over the ground, under everyone;
    /// it covers again who stands behind it (see `Bodies`).
    PlaceUpper,
    /// The light's colour, stronger on the place than on the cast.
    PlaceLight,
    /// Those of the stage by depth ([`Stage::in_drawing_order`]): each with
    /// its shadow, what it holds, a glint if it shines.
    Bodies,
    /// The fishing lines, over the cast: nobody stands on them.
    Lines,
    /// What moves throw.
    Shots,
    /// The statuses over heads, then emotes and effects (see `Emotes`).
    Statuses,
    Effects,
    /// The light's colour on the cast.
    CastLight,
    /// A thing shown large, over the scene.
    Large,
    /// A picture instead of the scene (`plan`, `scene: <image>`).
    Cutaway,
    /// A dungeon's name on a dark screen.
    Banner,
    /// What falls (rain, snow...), the spotlight or its lights, halos, the
    /// pictures put about, bolts, the ripples.
    Weather,
    Lights,
    Halos,
    Pictures,
    Bolts,
    Ripples,
    /// The particles in the game's pixels, the bars of life, the words
    /// rising, the bubbles, the film bands.
    Particles,
    Bars,
    Popups,
    Bubbles,
    Bands,
    /// The numbers of blows, in the game's digits.
    Digits,
    /// The particles that glow, in all the screen's pixels.
    Glows,
    /// The screen without colours, then the fade.
    Grey,
    Fade,
    /// Over the fade: black coming from a side, the credits, a caption, the
    /// words kept on screen, a fight's log, the gauges, a dungeon's map.
    Wipe,
    Credits,
    Caption,
    Hud,
    Log,
    Gauges,
    Minimap,
    /// The dialogue box and its choices: drawn by whoever has the words,
    /// at its rank (see `Stage::drawing_order`).
    Box,
}

/// The sheets of a scene, from the furthest back.
pub const SHEETS: [Sheet; 35] = [
    Sheet::Place,
    Sheet::PlaceUpper,
    Sheet::PlaceLight,
    Sheet::Bodies,
    Sheet::Lines,
    Sheet::Shots,
    Sheet::Statuses,
    Sheet::Effects,
    Sheet::CastLight,
    Sheet::Large,
    Sheet::Cutaway,
    Sheet::Banner,
    Sheet::Weather,
    Sheet::Lights,
    Sheet::Halos,
    Sheet::Pictures,
    Sheet::Bolts,
    Sheet::Ripples,
    Sheet::Particles,
    Sheet::Bars,
    Sheet::Popups,
    Sheet::Bubbles,
    Sheet::Bands,
    Sheet::Digits,
    Sheet::Glows,
    Sheet::Grey,
    Sheet::Fade,
    Sheet::Wipe,
    Sheet::Credits,
    Sheet::Caption,
    Sheet::Hud,
    Sheet::Log,
    Sheet::Gauges,
    Sheet::Minimap,
    Sheet::Box,
];

/// Between the ranks of two sheets one after the other.
const Z_STEP: f32 = 10.0;

impl Sheet {
    /// Drawn on the game's own grid of pixels (words, bubbles, digits), not
    /// in all the screen's: what is drawn so goes over what was drawn in the
    /// screen's pixels before it.
    pub fn in_game_pixels(self) -> bool {
        matches!(
            self,
            Sheet::Banner
                | Sheet::Weather
                | Sheet::Lights
                | Sheet::Halos
                | Sheet::Pictures
                | Sheet::Bolts
                | Sheet::Ripples
                | Sheet::Particles
                | Sheet::Bars
                | Sheet::Popups
                | Sheet::Bubbles
                | Sheet::Bands
                | Sheet::Digits
                | Sheet::Wipe
                | Sheet::Credits
                | Sheet::Caption
                | Sheet::Hud
                | Sheet::Log
                | Sheet::Gauges
                | Sheet::Minimap
                | Sheet::Box
        )
    }

    /// Its rank: ten more than the one before it, the bodies' at 0 (the
    /// place -30, the pictures 120, the fade 230, the box 310).
    pub fn z(self) -> f32 {
        let at = SHEETS.iter().position(|sheet| *sheet == self).unwrap_or(0) as f32;
        let bodies = SHEETS.iter().position(|sheet| *sheet == Sheet::Bodies).unwrap_or(0) as f32;
        (at - bodies) * Z_STEP
    }

    /// Its name, as the table of ranks says it.
    pub fn said(self) -> &'static str {
        match self {
            Sheet::Place => "the place",
            Sheet::PlaceUpper => "the top of the place (roofs, trees)",
            Sheet::PlaceLight => "the light on the place",
            Sheet::Bodies => "bodies and things, by their feet (on the ground -1, among 0, over 1)",
            Sheet::Lines => "fishing lines",
            Sheet::Shots => "what moves throw",
            Sheet::Statuses => "statuses",
            Sheet::Effects => "emotes and effects",
            Sheet::CastLight => "the light on bodies",
            Sheet::Large => "a thing shown large",
            Sheet::Cutaway => "a picture in place of the scene (shot)",
            Sheet::Banner => "a dungeon's name",
            Sheet::Weather => "the weather",
            Sheet::Lights => "the spotlight, the lights",
            Sheet::Halos => "halos",
            Sheet::Pictures => "pictures (node … picture)",
            Sheet::Bolts => "lightning bolts",
            Sheet::Ripples => "ripples",
            Sheet::Particles => "particles",
            Sheet::Bars => "health bars",
            Sheet::Popups => "words rising",
            Sheet::Bubbles => "bubbles",
            Sheet::Bands => "film bands",
            Sheet::Digits => "the digits of blows",
            Sheet::Glows => "glowing particles",
            Sheet::Grey => "the screen without colours",
            Sheet::Fade => "the fade",
            Sheet::Wipe => "the wipe",
            Sheet::Credits => "the credits",
            Sheet::Caption => "the caption",
            Sheet::Hud => "words kept on screen",
            Sheet::Log => "a fight's log",
            Sheet::Gauges => "gauges",
            Sheet::Minimap => "a dungeon's map",
            Sheet::Box => "the dialogue box and its choices",
        }
    }
}

/// What is drawn at a rank: a sheet, or one of those of the stage drawn
/// each at its own (a body or thing, a picture by its rank among them, a
/// halo by its).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drawn {
    Sheet(Sheet),
    Body(Who),
    Picture(usize),
    Halo(usize),
}

/// The ranks, as FILMS.md has them (“z ranks”, checked by
/// `film/src/tests/docs.rs`).
pub fn z_table() -> String {
    let mut out = String::from("| z | What is drawn |\n|---|---|\n");
    for sheet in SHEETS {
        out.push_str(&format!("| {} | {} |\n", sheet.z(), sheet.said()));
    }
    out
}

/// Where someone is drawn among the others, from the furthest back. A
/// place's own pictures, its upper layer too, are under all of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// Lying on the ground, or told to stay under everyone.
    Ground,
    /// Among the others: who stands lower hides who stands higher.
    Among,
    /// Over everyone.
    Over,
}

impl Layer {
    /// The layer of someone with these attributes (`flat`: its picture lies
    /// on the ground).
    pub fn of(attributes: u16, flat: bool) -> Self {
        let has = |bits: u16| attributes & bits != 0;
        match () {
            _ if has(attribute::OVER_MAP) => Layer::Over,
            _ if has(attribute::UNDER) => Layer::Ground,
            _ if has(attribute::OVER | attribute::ON_SCREEN) => Layer::Over,
            _ if flat => Layer::Ground,
            _ => Layer::Among,
        }
    }

    /// Its rank, about the bodies' sheet's (see `Sheet::z`).
    pub fn z(self) -> f32 {
        let around = Sheet::Bodies.z();
        match self {
            Layer::Ground => around - 1.0,
            Layer::Among => around,
            Layer::Over => around + 1.0,
        }
    }
}

impl Stage {
    /// Those who show, in the order to draw them (`flat`: whose picture
    /// lies on the ground).
    pub fn in_drawing_order(&self, flat: impl Fn(Who) -> bool) -> Vec<(Who, &Actor)> {
        let showing = self.cast.iter().filter(|(_, actor)| actor.attributes & attribute::HIDDEN == 0);
        let mut order: Vec<(Layer, Who, &Actor)> = showing.map(|(who, actor)| (Layer::of(actor.attributes, flat(*who)), *who, actor)).collect();
        order.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.drawn_at().total_cmp(&b.2.drawn_at())));
        // (Who was told to be in front of someone: just after them; one
        // hung on someone with no `z` of its own too.)
        let hung = |who: Who| {
            let kept = self.nodes.get(&crate::beat::NodeRef::Who(who)).filter(|kept| kept.z.is_none())?;
            match kept.parent {
                Some(crate::beat::NodeRef::Who(parent)) => Some(parent),
                _ => None,
            }
        };
        let told: Vec<(Who, Who)> = order.iter().filter_map(|(_, who, actor)| Some((*who, actor.in_front_of().or_else(|| hung(*who))?))).collect();
        for (who, other) in told {
            let at = |order: &[(Layer, Who, &Actor)], one: Who| order.iter().position(|(_, shown, _)| *shown == one);
            if let (Some(mine), Some(theirs)) = (at(&order, who), at(&order, other))
                && mine < theirs
            {
                let moved = order.remove(mine);
                order.insert(theirs, moved);
            }
        }
        order.into_iter().map(|(_, who, actor)| (who, actor)).collect()
    }

    /// A node's own rank, if it was told one.
    fn told_z(&self, node: &crate::beat::NodeRef) -> Option<f32> {
        self.nodes.get(node).and_then(|kept| kept.z)
    }

    /// Whether someone was told a rank over the place's upper layer (tree
    /// crowns, roofs: `prop a z 1`, `front_of_leaves a`): whoever draws
    /// does not cover it again with that layer when it stands behind it.
    /// Told one under it (`prop a z -25`), it is under that layer whole.
    pub fn over_place_upper(&self, who: Who) -> bool {
        self.told_z(&crate::beat::NodeRef::Who(who)).is_some_and(|z| z > Sheet::PlaceUpper.z())
    }

    /// Everything drawn, at its rank, from the furthest back (`flat`: whose
    /// picture lies on the ground). The sheets, the bodies and things (by
    /// their layer and their feet), the pictures and the halos, each at its
    /// `z` if told one, else at its sheet's (its layer's); sorted by rank,
    /// those of the same one in today's order.
    pub fn drawing_order(&self, flat: impl Fn(Who) -> bool) -> Vec<(f32, Drawn)> {
        use crate::beat::NodeRef;
        let mut drawn = Vec::new();
        for sheet in SHEETS {
            match sheet {
                Sheet::Bodies => {
                    for (who, actor) in self.in_drawing_order(&flat) {
                        let layer = Layer::of(actor.attributes, flat(who));
                        drawn.push((self.told_z(&NodeRef::Who(who)).unwrap_or(layer.z()), Drawn::Body(who)));
                    }
                }
                Sheet::Pictures => {
                    for (at, picture) in self.words.pictures.iter().enumerate() {
                        drawn.push((self.told_z(&NodeRef::Named(picture.name.clone())).unwrap_or(sheet.z()), Drawn::Picture(at)));
                    }
                }
                Sheet::Halos => {
                    for at in 0..self.light.halos.len() {
                        let told = u16::try_from(at).ok().and_then(|rank| self.told_z(&NodeRef::Halo(rank)));
                        drawn.push((told.unwrap_or(sheet.z()), Drawn::Halo(at)));
                    }
                }
                sheet => drawn.push((sheet.z(), Drawn::Sheet(sheet))),
            }
        }
        // (Stable: the same rank, as they came.)
        drawn.sort_by(|a, b| a.0.total_cmp(&b.0));
        drawn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_sheet_is_drawn_once_and_the_words_after_the_bodies() {
        for sheet in SHEETS {
            assert_eq!(SHEETS.iter().filter(|other| **other == sheet).count(), 1, "{sheet:?}");
        }
        let at = |sheet: Sheet| SHEETS.iter().position(|other| *other == sheet);
        assert!(at(Sheet::Place) < at(Sheet::PlaceUpper) && at(Sheet::PlaceUpper) < at(Sheet::Bodies));
        assert!(at(Sheet::Bodies) < at(Sheet::Bubbles) && at(Sheet::Fade) < at(Sheet::Caption));
    }

    #[test]
    fn untold_everything_is_drawn_in_today_s_order() {
        let mut stage = Stage::default();
        let (a, b, low) = (Who::Character(1), Who::Character(2), Who::Character(3));
        stage.put(a, Vec2::new(0.0, 50.0), 1);
        stage.put(b, Vec2::new(0.0, 10.0), 1);
        stage.put(low, Vec2::new(0.0, 90.0), 1);
        stage.cast.get_mut(&low).expect("put").attributes = attribute::UNDER;
        stage.words.pictures.push(PlacedPicture { name: "moon".to_string(), file: "moon.png".to_string(), at: Vec2::ZERO });
        let order: Vec<Drawn> = stage.drawing_order(|_| false).into_iter().map(|(_, drawn)| drawn).collect();
        // (The sheets as they are, the bodies in the place of theirs, the picture in that of the pictures.)
        let mut today = Vec::new();
        for sheet in SHEETS {
            match sheet {
                Sheet::Bodies => today.extend([Drawn::Body(low), Drawn::Body(b), Drawn::Body(a)]),
                Sheet::Pictures => today.push(Drawn::Picture(0)),
                Sheet::Halos => {}
                sheet => today.push(Drawn::Sheet(sheet)),
            }
        }
        assert_eq!(order, today);
        let ranks: Vec<f32> = SHEETS.iter().map(|sheet| sheet.z()).collect();
        assert!(ranks.windows(2).all(|pair| pair[0] < pair[1]), "{ranks:?}");
        assert!(Layer::Over.z() < Sheet::Lines.z() && Sheet::PlaceLight.z() < Layer::Ground.z());
    }

    #[test]
    fn a_rank_told_puts_it_where_its_number_says_ties_as_they_came() {
        use crate::beat::NodeRef;
        use crate::props::{Given, PropId, Value};
        let mut stage = Stage::default();
        let (a, b, c) = (Who::Character(1), Who::Character(2), Who::Character(3));
        stage.put(a, Vec2::new(0.0, 90.0), 1);
        stage.put(b, Vec2::new(0.0, 10.0), 1);
        stage.put(c, Vec2::new(0.0, 50.0), 1);
        let z = |stage: &mut Stage, who: Who, z: f32| stage.set(&NodeRef::Who(who), PropId::Z, &Given::Is(Value::Number(z))).expect("set");
        let at = |stage: &Stage, drawn: Drawn| stage.drawing_order(|_| false).iter().position(|(_, one)| *one == drawn).expect("drawn");
        // (Over the box; under the place's light; the same rank as another: by feet still.)
        z(&mut stage, a, 400.0);
        z(&mut stage, b, -15.0);
        assert!(at(&stage, Drawn::Body(a)) > at(&stage, Drawn::Sheet(Sheet::Box)));
        assert!(
            at(&stage, Drawn::Body(b)) < at(&stage, Drawn::Sheet(Sheet::PlaceLight))
                && at(&stage, Drawn::Body(b)) > at(&stage, Drawn::Sheet(Sheet::PlaceUpper))
        );
        z(&mut stage, a, 0.0);
        z(&mut stage, b, 0.0);
        assert!(at(&stage, Drawn::Body(b)) < at(&stage, Drawn::Body(c)) && at(&stage, Drawn::Body(c)) < at(&stage, Drawn::Body(a)));
        // (A picture told under the bodies.)
        stage.words.pictures.push(PlacedPicture { name: "moon".to_string(), file: "moon.png".to_string(), at: Vec2::ZERO });
        stage.set(&NodeRef::Named("moon".to_string()), PropId::Z, &Given::Is(Value::Number(-5.0))).expect("set");
        assert!(at(&stage, Drawn::Picture(0)) < at(&stage, Drawn::Body(b)));
    }

    #[test]
    fn one_told_over_the_place_s_upper_layer_is_not_covered_by_it() {
        use crate::beat::NodeRef;
        use crate::props::{Given, PropId, Value};
        let mut stage = Stage::default();
        let (peeks, hides, untold) = (Who::Character(1), Who::Character(2), Who::Character(3));
        for who in [peeks, hides, untold] {
            stage.put(who, Vec2::new(0.0, 40.0), 1);
        }
        stage.set(&NodeRef::Who(peeks), PropId::Z, &Given::Is(Value::Number(1.0))).expect("set");
        stage.set(&NodeRef::Who(hides), PropId::Z, &Given::Is(Value::Number(-25.0))).expect("set");
        assert!(stage.over_place_upper(peeks) && !stage.over_place_upper(hides) && !stage.over_place_upper(untold));
        // (Under it whole: drawn before the upper layer.)
        let at = |drawn: Drawn| stage.drawing_order(|_| false).iter().position(|(_, one)| *one == drawn).expect("drawn");
        assert!(at(Drawn::Body(hides)) < at(Drawn::Sheet(Sheet::PlaceUpper)) && at(Drawn::Body(peeks)) > at(Drawn::Sheet(Sheet::PlaceUpper)));
    }

    #[test]
    fn the_ranks_are_in_their_table() {
        let table = z_table();
        assert!(table.contains("| 0 | bodies") && table.contains("| 310 | the dialogue box"), "{table}");
    }

    #[test]
    fn what_a_body_is_told_outweighs_what_it_lies_on() {
        assert_eq!(Layer::of(0, false), Layer::Among);
        assert_eq!(Layer::of(0, true), Layer::Ground);
        assert_eq!(Layer::of(attribute::OVER, true), Layer::Over);
        assert_eq!(Layer::of(attribute::UNDER | attribute::OVER, false), Layer::Ground);
        assert_eq!(Layer::of(attribute::OVER_MAP | attribute::UNDER, false), Layer::Over);
    }

    #[test]
    fn who_is_told_to_be_in_front_is_drawn_after_and_depth_counts_as_standing_lower() {
        let mut stage = Stage::default();
        let (a, b) = (Who::Character(1), Who::Character(2));
        stage.put(a, Vec2::new(0.0, 10.0), 1);
        stage.put(b, Vec2::new(0.0, 50.0), 1);
        let order = |stage: &Stage| stage.in_drawing_order(|_| false).into_iter().map(|(who, _)| who).collect::<Vec<Who>>();
        assert_eq!(order(&stage), [a, b]);
        stage.direct(Beat::Body(BodyBeat::Depth { who: a, depth: 60.0 }), Vec2::ZERO);
        assert_eq!(order(&stage), [b, a]);
        stage.direct(Beat::Body(BodyBeat::Depth { who: a, depth: 0.0 }), Vec2::ZERO);
        stage.direct(Beat::Body(BodyBeat::InFront { who: a, of: Some(b) }), Vec2::ZERO);
        assert_eq!(order(&stage), [b, a]);
        stage.direct(Beat::Body(BodyBeat::InFront { who: a, of: None }), Vec2::ZERO);
        assert_eq!(order(&stage), [a, b]);
    }

    #[test]
    fn who_stands_lower_is_drawn_after_and_what_lies_first() {
        let mut stage = Stage::default();
        let (high, low, mat, ghost, flying) = (Who::Character(1), Who::Character(2), Who::Character(3), Who::Character(4), Who::Character(5));
        stage.put(low, Vec2::new(0.0, 50.0), 1);
        stage.put(high, Vec2::new(0.0, 10.0), 1);
        stage.put(mat, Vec2::new(0.0, 90.0), 1);
        stage.put(ghost, Vec2::new(0.0, 20.0), 1);
        stage.put(flying, Vec2::new(0.0, 0.0), 1);
        stage.cast.get_mut(&ghost).expect("put").attributes = attribute::HIDDEN;
        stage.cast.get_mut(&flying).expect("put").attributes = attribute::OVER;
        let order: Vec<Who> = stage.in_drawing_order(|who| who == mat).into_iter().map(|(who, _)| who).collect();
        assert_eq!(order, [mat, high, low, flying]);
    }
}
