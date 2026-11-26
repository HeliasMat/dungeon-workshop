//! The writer on its own, with a game of a few species (no ROM, no data):
//! the species the ROM lacks, a pose a sprite lacks, a crowd too big.

use std::collections::HashMap;

use engine::{AnimId, Beat, BodyBeat, Play, Who};

use super::{Actor, Writer};
use crate::sky::Game;

/// A data folder of the tests' own: Pichu, Unown, and one form of
/// SpriteCollab's (`pichu_spiky`, 5257).
fn data() -> String {
    let folder = std::env::temp_dir().join(format!("ds_export_writer_{}", std::process::id()));
    std::fs::create_dir_all(folder.join("rom")).expect("a folder");
    std::fs::create_dir_all(folder.join("extra")).expect("a folder");
    std::fs::write(folder.join("rom").join("monsters.txt"), "172\tPichu\n228\tUnown\n25\tPikachu\n").expect("written");
    std::fs::write(folder.join("extra").join("variants.txt"), "5257\t172\tpichu_spiky\n").expect("written");
    folder.to_string_lossy().into_owned()
}

fn game(data: &str) -> Game {
    let names = [(172, "Pichu"), (228, "Unown"), (25, "Pikachu"), (143, "Snorlax")].into_iter().map(|(at, name)| (at, name.to_string())).collect();
    Game { levels: HashMap::new(), actors: Vec::new(), data: data.to_string(), move_types: Vec::new(), names, decors: HashMap::new() }
}

/// A form SpriteCollab adds is its base species on DS, a Pokémon Sky
/// lacks the stand-in: never the species number passed on to alias
/// another, and told.
#[test]
fn a_species_the_rom_lacks_is_its_base_or_the_stand_in_told() {
    let data = data();
    let game = game(&data);
    let mut writer = Writer::new(&game);
    writer.film_names.insert(Who::Monster(5257), "pichu_spiky".to_string());
    writer.film_names.insert(Who::Monster(2570), "zorua".to_string());
    let spiky = writer.actor(Who::Monster(5257)).expect("an actor");
    let zorua = writer.actor(Who::Monster(2570)).expect("an actor");
    let pichu = writer.actor(Who::Monster(172)).expect("an actor");
    assert_ne!(spiky, pichu, "two actors");
    let species = |who: Who| writer.actors[&who].species;
    assert_eq!(species(Who::Monster(5257)), Some(172));
    assert_eq!(species(Who::Monster(2570)), Some(::film::ds::STAND_IN));
    assert!(writer.told.iter().any(|told| told.starts_with("pichu_spiky (species 5257") && told.contains("drawn as Pichu")), "{:?}", writer.told);
    assert!(writer.told.iter().any(|told| told.starts_with("zorua (species 2570") && told.contains("stand-in, Unown")), "{:?}", writer.told);
    // (The stand-in has no portrait: its own name said in its line.)
    writer.say(4, None, Some(Who::Monster(2570)), Some(1), "Hi.");
    assert!(writer.body.contains("message_ResetActor();\n") && writer.body.contains("message_Talk(\" Zorua: Hi.\");"), "{}", writer.body);
    assert!(!zorua.is_empty());
}

/// A pose no sprite of Sky's plays (Rotate) is the nearest every sprite
/// has, a hop, told by species: never left out.
#[test]
fn a_pose_the_sprite_lacks_is_the_nearest_every_sprite_has() {
    let data = data();
    let game = game(&data);
    let mut writer = Writer::new(&game);
    let pikachu = Who::Monster(25);
    writer.beat(7, Some("pose"), &Beat::Body(BodyBeat::Anim { who: pikachu, anim: AnimId(12), play: Play::Times(1) }));
    assert!(writer.body.contains("SetAnimation<actor ACTOR_NPC_TEST001>(17);"), "{}", writer.body);
    assert_eq!(writer.instead_poses.get(&("Pikachu".to_string(), "Rotate".to_string(), "Hop")), Some(&vec![7]));
    assert!(writer.notes.is_empty(), "{:?}", writer.notes);
}

/// 25 from the start, every one named by a line: the 21 kept are those
/// who speak, then who move; the others left out with their lines, named.
#[test]
fn a_crowd_with_no_one_idle_keeps_its_speakers_and_movers() {
    let data = data();
    let game = game(&data);
    let mut writer = Writer::new(&game);
    for at in 0..25u16 {
        let who = Who::Another { species: 25, copy: at };
        let name = format!("FREE{at:02}");
        writer.actors.insert(
            who,
            Actor { name: name.clone(), species: Some(25), kind: 25, first: Some((100.0 + f32::from(at) * 10.0, 100.0, "DIR_DOWN")), hidden: false, layer: 0 },
        );
        writer.film_names.insert(who, format!("pikachu{at}"));
        let line = match at {
            0 => format!("message_SetFace(ACTOR_{name}, FACE_NORMAL, FACE_POS_STANDARD);"),
            1..=10 => format!("MovePositionMark<actor ACTOR_{name}>(1, Position<'m{at}', 10, 10>);"),
            _ => format!("SetEffect<actor ACTOR_{name}>(EFFECT_JOYOUS, 3);"),
        };
        writer.body.push_str(&format!("    {line}\n"));
    }
    writer.fit();
    assert_eq!(writer.actors.len(), 21);
    assert!(writer.actors.values().any(|actor| actor.name == "FREE00"), "the speaker kept");
    assert!((1..=10).all(|at| writer.actors.values().any(|actor| actor.name == format!("FREE{at:02}"))), "the movers kept");
    let told = writer.told.join("\n");
    assert!(told.contains("25 Pokémon from the start") && told.contains("(only acting where they stand)") && !told.contains("left out none"), "{told}");
    // (The farthest from the crowd's middle first: the last ones, at its end.)
    assert!(told.contains("pikachu24"), "{told}");
    assert_eq!(writer.body.lines().count(), 21, "their lines left out with them:\n{}", writer.body);
}
