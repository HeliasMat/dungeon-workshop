//! What this game's scenes ask of it. The scene engine knows these lines
//! only as the host's (`Beat::Host`, `Course::Host`): here they mean
//! something.

use glam::Vec2;

use engine::{Beat, Course, Cue, SHOT_FRAMES, Stage, Who};
use rules::tables::Tables;

/// What a scene does with the world map of the top screen.
#[derive(Clone, Debug, PartialEq)]
pub enum MapCommand {
    Show(u16),
    Hide,
    Mode(u16),
    Level(u16),
    Look { at: Vec2, slides: bool },
    Arrow(Option<Vec2>),
    Mark { at: Vec2, blinks: bool },
    Name(Option<String>),
}

/// A line of a scene that only the game can play.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// The team is given money (taken away when less than 0).
    Money(i32),
    /// So many of an item given to the team or taken from it.
    Items {
        item: u16,
        count: u16,
        take: bool,
    },
    /// Someone of the stage joins the team, at that level.
    Recruit {
        who: Who,
        level: u16,
    },
    /// What the top screen shows, and where it looks.
    Top(Option<u16>),
    TopCam(f32, f32),
    TopImage(Option<String>),
    Map(MapCommand),
    /// A dungeon to go through, a place to walk in, one of the game's
    /// menus.
    Dungeon(u16),
    Ground(u16),
    Menu(u16),
    /// A special episode opens; a dungeon opens to visits.
    Open(u16),
    Unlock(u16),
    /// A fight, as its line has it (see `fight::host`).
    Fight(Vec<String>),
    /// The game's team (a film one plays: `player`): the species of its
    /// leader and of its partner (0: none said).
    Team(u16, u16),
    /// So many of a thing (taken, below 0) into the bag of the game, by its
    /// name (a film one plays: `stow`, `take_out`).
    Bag(i32, String),
}

/// An item as files write it: `109`, `i109`, `i109.2`.
fn item(field: &str) -> Option<u16> {
    field.trim_start_matches('i').split('.').next()?.parse().ok()
}

fn map(fields: &[String]) -> Option<MapCommand> {
    let number = |at: usize| fields.get(at).and_then(|field| field.parse::<u16>().ok());
    let real = |at: usize| fields.get(at).and_then(|field| field.parse::<f32>().ok());
    let flag = |at: usize| fields.get(at).is_some_and(|field| field == "1");
    Some(match fields.get(1)?.as_str() {
        "show" => MapCommand::Show(number(2)?),
        "hide" => MapCommand::Hide,
        "mode" => MapCommand::Mode(number(2)?),
        "level" => MapCommand::Level(number(2)?),
        "look" => MapCommand::Look { at: Vec2::new(real(2)?, real(3)?), slides: flag(4) },
        "arrow" => MapCommand::Arrow(real(2).zip(real(3)).map(|(x, y)| Vec2::new(x, y))),
        "mark" => MapCommand::Mark { at: Vec2::new(real(2)?, real(3)?), blinks: flag(4) },
        "name" => MapCommand::Name(fields.get(2).filter(|name| *name != "-").cloned()),
        _ => return None,
    })
}

/// What a line of this game's asks of the game that plays the scene, as the
/// engine hands it over (`Vocabulary::host_asks`): the money, the items.
pub fn game_ask(fields: &[String]) -> Option<engine::GameAsk> {
    match Command::parse(fields)? {
        Command::Money(amount) => Some(engine::GameAsk::Money(amount)),
        Command::Items { item, count, take } => Some(engine::GameAsk::Item { item, count, take }),
        _ => None,
    }
}

impl Command {
    /// A line of a scene as what it asks.
    pub fn parse(fields: &[String]) -> Option<Self> {
        let number = |at: usize| fields.get(at).and_then(|field| field.parse::<u16>().ok());
        Some(match fields.first()?.as_str() {
            "getmoney" => Command::Money(fields.get(1)?.parse().ok()?),
            word @ ("getitem" | "takeitem") => {
                Command::Items { item: item(fields.get(1)?)?, count: fields.get(2).map_or(Some(1), |count| count.parse().ok())?, take: word == "takeitem" }
            }
            // (Level 5 when none is given.)
            "recruit" => Command::Recruit { who: Who::parse(fields.get(1)?)?, level: number(2).unwrap_or(5).clamp(1, 100) },
            "top" => Command::Top(number(1)),
            "topcam" => Command::TopCam(fields.get(1)?.parse().ok()?, fields.get(2)?.parse().ok()?),
            "topimage" => Command::TopImage(Some(fields.get(1)?.clone()).filter(|file| file != "off")),
            "map" => Command::Map(map(fields)?),
            "dungeon" => Command::Dungeon(number(1)?),
            "ground" => Command::Ground(number(1)?),
            "menu" => Command::Menu(number(1)?),
            "open" => Command::Open(number(1)?),
            "unlock" => Command::Unlock(number(1)?),
            "fight" => Command::Fight(fields.to_vec()),
            "team" => Command::Team(number(1)?, number(2).unwrap_or(0)),
            "bag" => Command::Bag(fields.get(1)?.parse::<f64>().ok()? as i32, Some(fields[2..].join(" ")).filter(|name| !name.is_empty())?),
            _ => return None,
        })
    }

    /// What a beat asks of the game, if it does.
    pub fn of(beat: &Beat) -> Option<Self> {
        match beat {
            Beat::Host(fields) | Beat::Course(Course::Host(fields)) => Self::parse(fields),
            _ => None,
        }
    }

    /// The story going on in a place, as a beat.
    pub fn ground(level: u16) -> Beat {
        Beat::Course(Course::Host(vec!["ground".to_string(), level.to_string()]))
    }

    /// One of the game's menus, as a beat.
    pub fn menu(menu: u16) -> Beat {
        Beat::Course(Course::Host(vec!["menu".to_string(), menu.to_string()]))
    }

    /// The place a beat sends the story to.
    pub fn ground_of(beat: &Beat) -> Option<u16> {
        match Self::of(beat)? {
            Command::Ground(level) => Some(level),
            _ => None,
        }
    }

    /// The menu a beat opens.
    pub fn menu_of(beat: &Beat) -> Option<u16> {
        match Self::of(beat)? {
            Command::Menu(menu) => Some(menu),
            _ => None,
        }
    }

    /// The dungeon a beat sends the team to.
    pub fn dungeon_of(beat: &Beat) -> Option<u16> {
        match Self::of(beat)? {
            Command::Dungeon(dungeon) => Some(dungeon),
            _ => None,
        }
    }
}

/// What shows of a move, used by a monster of some species.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveShown {
    /// The motion of its user, where on it the move shows, its sound.
    pub group: u8,
    pub point: u8,
    pub sound: u16,
    /// What shows before it, as it strikes, on the target, what flies.
    pub effects: [u16; 4],
    pub lob: bool,
    pub flight: f32,
}

impl MoveShown {
    pub fn of(tables: &Tables, id: u16, species: Option<u16>) -> Self {
        let motion = tables.move_motions.get(id as usize);
        let own_sound = tables.move_sounds.get(id as usize).copied().unwrap_or(0);
        let (group, point, sound) = motion.map_or((1, 3, own_sound), |motion| motion.of(species));
        // (Most moves have none of their motion's: theirs is the move's own.)
        let sound = if sound & 0xFF != 0 { sound } else { own_sound };
        let effects = tables.move_effects.get(id as usize).copied().unwrap_or_default();
        MoveShown {
            group,
            point,
            sound,
            effects,
            lob: motion.is_some_and(|motion| motion.lob),
            flight: motion.map_or(6.0, |motion| (motion.flight as f32).max(1.0)),
        }
    }
}

/// A move used on the stage, as in dungeons: its user moves as the move has
/// it, what shows before it, then what shows as it strikes, then what flies
/// at its own speed, and what shows on the target as the blow lands.
/// (`before`: how long what shows first lasts; `flies`: what flies can be
/// shown; `hit`: the frames to the step its user's sprite marks for the
/// blow, if it does: what it strikes with shows then, what flies leaves
/// then, a blow struck by hand lands then.)
pub fn stage_move(stage: &mut Stage, who: Who, target: Option<Who>, shown: &MoveShown, before: f32, flies: bool, hit: Option<f32>) {
    let [first, strike, on_target, flying] = shown.effects;
    if let Some(actor) = stage.cast.get_mut(&who) {
        actor.cue(Cue::Motion(shown.group));
    }
    if first != 0 {
        stage.effect(who, first, None);
    }
    if strike != 0 {
        stage.shoot_as(who, who, 0, strike, hit.map_or(before, |hit| hit.max(before)), false, false);
    }
    let Some(target) = target else {
        return;
    };
    let far = stage.cast.get(&who).zip(stage.cast.get(&target)).map_or(0.0, |(from, to)| from.position.distance(to.position));
    let landing = match (flies, hit) {
        (true, _) => (far / shown.flight).max(SHOT_FRAMES),
        (false, Some(hit)) => hit,
        (false, None) => SHOT_FRAMES,
    };
    let wait = if flies { hit.unwrap_or(0.0) } else { 0.0 };
    stage.shoot_after(who, target, if flies { flying } else { 0 }, on_target, landing, shown.lob, true, wait);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| word.to_string()).collect()
    }

    #[test]
    fn lines_are_read_as_what_they_ask() {
        assert_eq!(Command::parse(&line(&["getmoney", "-50"])), Some(Command::Money(-50)));
        assert_eq!(Command::parse(&line(&["takeitem", "i109.2", "3"])), Some(Command::Items { item: 109, count: 3, take: true }));
        assert_eq!(Command::parse(&line(&["recruit", "m25"])), Some(Command::Recruit { who: Who::Monster(25), level: 5 }));
        assert_eq!(Command::parse(&line(&["map", "level", "4"])), Some(Command::Map(MapCommand::Level(4))));
        assert_eq!(Command::parse(&line(&["walk", "0", "1", "2"])), None);
    }

    #[test]
    fn beats_say_where_they_lead() {
        assert_eq!(Command::ground_of(&Command::ground(9)), Some(9));
        assert_eq!(Command::menu_of(&Command::menu(7)), Some(7));
        assert_eq!(Command::dungeon_of(&Command::ground(9)), None);
    }
}
