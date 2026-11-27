//! The writer on its own, with a game of a few species (no ROM, no data):
//! the species the ROM lacks, a pose a sprite lacks, a crowd too big.

use std::collections::HashMap;

use engine::{AnimId, Beat, BodyBeat, Frames, MoveId, Play, SpeechBeat, Voice, Who};

use ::film::ds::patches::{PATCHES, Patches};

use super::{Actor, Writer};
use crate::notes::{Fate, Kind, Note};
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
    Game { levels: HashMap::new(), actors: Vec::new(), data: data.to_string(), move_types: Vec::new(), move_effects: Vec::new(), names, decors: HashMap::new() }
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
    let species: Vec<&Note> = writer.notes.iter().filter(|note| note.kind == Kind::Species && note.fate == Fate::Changed).collect();
    assert!(species.iter().any(|note| note.message.starts_with("pichu_spiky (species 5257") && note.instead.contains("drawn as Pichu")), "{species:?}");
    assert!(species.iter().any(|note| note.message.starts_with("zorua (species 2570") && note.instead.contains("stand-in, Unown")), "{species:?}");
    assert_eq!(species.len(), 2, "each told once: {species:?}");
    // (The stand-in has no portrait: its own name said in its line.)
    writer.say(4, None, Some(Who::Monster(2570)), Some(1), "Hi.");
    assert!(writer.body.contains("message_EmptyActor();\n") && writer.body.contains("message_Talk(\"Zorua: Hi.\");"), "{}", writer.body);
    assert!(!zorua.is_empty());
}

/// A Pokémon SpriteCollab adds whose sheets the data has is put in the ROM:
/// itself on its actor, its portrait shown, the one drawn instead kept for
/// `patch_rom.py`, nothing told as a loss.
#[test]
fn a_species_with_its_sheets_is_put_in_the_rom() {
    let folder = std::env::temp_dir().join(format!("ds_export_added_{}", std::process::id()));
    std::fs::create_dir_all(folder.join("extra").join("portraits")).expect("a folder");
    std::fs::create_dir_all(folder.join("rom")).expect("a folder");
    // (Grookey's walk and idle, as the dungeons' sheets: 0 and 1.)
    std::fs::write(folder.join("extra").join("sprites.txt"), "2810\t0\t32\t56\t6,4\t0\t0\n2810\t1\t24\t40\t16,16\t0\t0\n").expect("written");
    std::fs::write(folder.join("extra").join("portraits").join("2810_0.png"), b"").expect("written");
    let data = folder.to_string_lossy().into_owned();
    let game = game(&data);
    let mut writer = Writer::new(&game);
    let grookey = Who::Monster(2810);
    writer.film_names.insert(grookey, "grookey".to_string());
    let actor = writer.actor(grookey).expect("an actor");
    assert_eq!(writer.actors[&grookey].species, Some(2810));
    assert_eq!(writer.added.get(&2810), Some(&::film::ds::STAND_IN));
    assert!(writer.notes.iter().all(|note| note.kind == Kind::Added && note.fate == Fate::Added), "{:?}", writer.notes);
    assert_eq!(writer.notes.len(), 1, "{:?}", writer.notes);
    writer.say(4, None, Some(grookey), Some(0), "Hi.");
    assert!(writer.body.contains(&format!("message_SetFace({actor}, FACE_NORMAL")), "{}", writer.body);
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
            Actor {
                name: name.clone(),
                species: Some(25),
                kind: 25,
                first: Some((100.0 + f32::from(at) * 10.0, 100.0, "DIR_DOWN")),
                hidden: false,
                seen: false,
                layer: 0,
            },
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
    let crowd: Vec<&Note> = writer.notes.iter().filter(|note| note.kind == Kind::Crowd).collect();
    assert!(crowd.iter().all(|note| note.fate == Fate::LeftOut), "{crowd:?}");
    let told = crowd.iter().map(|note| format!("{}: {}", note.message, note.instead)).collect::<Vec<_>>().join("\n");
    assert!(told.contains("25 Pokémon from the start") && told.contains("(only acting where they stand)") && !told.contains("left out none"), "{told}");
    // (The farthest from the crowd's middle first: the last ones, at its end.)
    assert!(told.contains("pikachu24"), "{told}");
    assert_eq!(writer.body.lines().count(), 21, "their lines left out with them:\n{}", writer.body);
}

/// A move shows its own effect on the one it reaches (the game's tables:
/// Thunderbolt's 236), its type's without them; at a place, its own as
/// it strikes, on its user.
#[test]
fn a_move_shows_its_own_effect() {
    let data = data();
    let mut tables = game(&data);
    tables.move_effects = vec![[0; 4]; 538];
    tables.move_effects[129] = [22, 0, 236, 0];
    tables.move_effects[537] = [613, 610, 612, 611];
    let (pikachu, pichu) = (Who::Monster(25), Who::Monster(172));
    let mut writer = Writer::new(&tables);
    writer.started = true;
    writer.beat(3, Some("use"), &Beat::Body(BodyBeat::Move { who: pikachu, id: MoveId(129), target: Some(pichu) }));
    let (user, target) = (writer.actors[&pikachu].name.clone(), writer.actors[&pichu].name.clone());
    assert!(writer.body.contains(&format!("SetEffect<actor ACTOR_{target}>(236, 3);")), "{}", writer.body);
    writer.beat(4, Some("use"), &Beat::Body(BodyBeat::Move { who: pikachu, id: MoveId(537), target: Some(Who::Spot(900)) }));
    assert!(writer.body.contains(&format!("SetEffect<actor ACTOR_{user}>(610, 3);")), "{}", writer.body);
    // (No tables: Ember's type, Fire.)
    let mut types = game(&data);
    types.move_types = vec![0; 263];
    types.move_types[262] = 2;
    let mut writer = Writer::new(&types);
    writer.started = true;
    writer.beat(3, Some("attack"), &Beat::Body(BodyBeat::Move { who: pikachu, id: MoveId(262), target: Some(pichu) }));
    let target = writer.actors[&pichu].name.clone();
    assert!(writer.body.contains(&format!("SetEffect<actor ACTOR_{target}>(6, 3);")), "{}", writer.body);
}

/// A data folder with Pikachu's, Ditto's and Munchlax's portraits (normal,
/// happy, sad), for the lines said with a face.
fn faces_data(name: &str) -> String {
    let folder = std::env::temp_dir().join(format!("ds_export_{name}_{}", std::process::id()));
    std::fs::create_dir_all(folder.join("rom").join("portraits")).expect("a folder");
    std::fs::write(folder.join("rom").join("monsters.txt"), "25\tPikachu\n132\tDitto\n488\tMunchlax\n").expect("written");
    for species in [25, 132, 488] {
        for face in [0, 1, 5] {
            std::fs::write(folder.join("rom").join("portraits").join(format!("{species}_{face}.png")), b"").expect("written");
        }
    }
    folder.to_string_lossy().into_owned()
}

/// A face changing in a line (`[FACE:n]`, from `[sad]`) changes in the box
/// with ChangePortraitMidText, `*words*` bold with BoldText; without the
/// mega patch, a box for each face, told, and the stars as written.
#[test]
fn a_face_changes_in_the_box_and_stars_are_bold_with_the_mega_patch() {
    let data = faces_data("faces");
    let game = game(&data);
    let pikachu = Who::Monster(25);
    let mut writer = Writer::new(&game);
    writer.patches = Patches::read(PATCHES, "", false);
    let actor = writer.actor(pikachu).expect("an actor");
    writer.say(4, None, Some(pikachu), Some(1), "We won the *cup*! [FACE:5]But...");
    assert!(writer.body.contains(&format!("message_SetFace({actor}, FACE_HAPPY, FACE_POS_STANDARD);")), "{}", writer.body);
    assert!(writer.body.contains("message_Talk(\" We won the [BS]cup[BR]! [FACE:5]But...\");"), "{}", writer.body);
    assert!(writer.notes.is_empty(), "{:?}", writer.notes);
    let mut writer = Writer::new(&game);
    writer.patches = Patches::none();
    let actor = writer.actor(pikachu).expect("an actor");
    writer.say(4, None, Some(pikachu), Some(1), "We won the *cup*! [FACE:5]But...");
    let said: Vec<&str> = writer.body.lines().map(str::trim).collect();
    assert_eq!(
        said,
        [
            format!("message_SetFace({actor}, FACE_HAPPY, FACE_POS_STANDARD);").as_str(),
            "message_Talk(\" We won the *cup*!\");",
            format!("message_SetFace({actor}, FACE_SAD, FACE_POS_STANDARD);").as_str(),
            "message_Talk(\" But...\");",
        ]
    );
    assert!(writer.notes.iter().any(|note| note.message == "a face changed in the middle of a line"), "{:?}", writer.notes);
}

/// A line said in a bubble: the box see-through while it shows
/// (set_textbox_transparency), solid again after.
#[test]
fn a_bubble_line_is_see_through_with_the_mega_patch() {
    let data = faces_data("bubble");
    let game = game(&data);
    let mut writer = Writer::new(&game);
    writer.patches = Patches::read(PATCHES, "", false);
    writer.beat(3, Some("bubble"), &Beat::Speech(SpeechBeat::Bubble { who: Who::Monster(25), frames: Frames(60.0), text: "Hm.".into(), voice: Voice::Said }));
    let said: Vec<&str> = writer.body.lines().map(str::trim).collect();
    let opened = said.iter().position(|line| *line == "ProcessSpecial(66, 1, 0);").expect("see-through");
    assert_eq!(said[opened + 1], "message_SetWaitMode(60, 0);");
    let closed = said.iter().position(|line| *line == "ProcessSpecial(66, 0, 0);").expect("solid again");
    assert_eq!(said[closed - 2..closed], ["message_Close();", "message_SetWaitMode(-1, -1);"]);
}

/// One left out of the scene (Sky's room) still says his lines, without an
/// actor: his name in them, his portrait by set-portrait-monster; every
/// other order to him left out.
#[test]
fn one_left_out_says_his_lines_with_a_portrait_alone() {
    let data = faces_data("alone");
    let game = game(&data);
    let mut writer = Writer::new(&game);
    writer.patches = Patches::read(PATCHES, "", false);
    let ditto = Who::Monster(132);
    writer.film_names.insert(ditto, "ditto".to_string());
    let actor = writer.actor(ditto).expect("an actor");
    writer.say(4, None, Some(ditto), Some(1), "Hi.");
    writer.body.push_str(&format!("    SetAnimation<actor {actor}>(2);\n"));
    writer.drop_actors(&[ditto]);
    let said: Vec<&str> = writer.body.lines().map(str::trim).collect();
    assert_eq!(said, ["message_EmptyActor();", "ProcessSpecial(64, 132, 1);", "message_Talk(\"[CS:N]Ditto[CR]: Hi.\");"]);
}

/// One who transforms is a special actor: his species set as the scene
/// loads him, then, under the white, taken away and loaded again as the
/// other (dynamic_actor) in a layer of his own; his lines then his own
/// name and face. Without dynamic_actor (a base for sharing), he keeps his
/// look, told.
#[test]
fn a_transform_changes_the_species_with_dynamic_actor() {
    let data = faces_data("transform");
    let game = game(&data);
    let (ditto, munchlax) = (Who::Monster(132), Who::Monster(488));
    let mut writer = Writer::new(&game);
    writer.patches = Patches::read(PATCHES, "", false);
    writer.shifters.push(ditto);
    writer.film_names.insert(ditto, "ditto".to_string());
    writer.body.push_str("    supervision_Acting(0);\n");
    let actor = writer.actor(ditto).expect("an actor");
    assert_eq!(actor, "ACTOR_EVENT_NPC01");
    writer.actor(munchlax).expect("an actor");
    writer.started = true;
    writer.beat(5, Some("transform"), &Beat::Body(BodyBeat::Mimic { who: ditto, like: Some(munchlax) }));
    writer.say(6, None, Some(ditto), Some(1), "Hi.");
    writer.beat(7, Some("transform"), &Beat::Body(BodyBeat::Mimic { who: ditto, like: None }));
    let block = writer.block();
    assert!(block.contains("// ds-export actor ACTOR_EVENT_NPC01 x") && block.contains(" layer 1\n"), "{block}");
    let script: Vec<&str> = block.lines().map(str::trim).collect();
    let set = script.iter().position(|line| *line == "ProcessSpecial(67, 132, 2); // ACTOR_EVENT_NPC01").expect("his species first");
    assert_eq!(script[set + 1], "supervision_Acting(0);");
    let into = script.iter().position(|line| *line == "ProcessSpecial(67, 488, 2); // ACTOR_EVENT_NPC01").expect("into munchlax");
    assert_eq!(
        script[into - 1..into + 2],
        ["Destroy<actor ACTOR_EVENT_NPC01>();", "ProcessSpecial(67, 488, 2); // ACTOR_EVENT_NPC01", "supervision_Acting(1);"]
    );
    assert!(block.contains("ProcessSpecial(64, 132, 1);\n    message_Talk(\"[CS:N]Ditto[CR]: Hi.\");"), "{block}");
    assert!(script.iter().filter(|line| **line == "ProcessSpecial(67, 132, 2); // ACTOR_EVENT_NPC01").count() == 2, "back: {block}");
    // (A base for sharing has no dynamic_actor: his look kept, told.)
    let mut writer = Writer::new(&game);
    writer.patches = Patches::read(PATCHES, "", true);
    writer.shifters.push(ditto);
    assert_ne!(writer.actor(ditto).expect("an actor"), "ACTOR_EVENT_NPC01");
    writer.beat(5, Some("transform"), &Beat::Body(BodyBeat::Mimic { who: ditto, like: Some(munchlax) }));
    assert!(!writer.body.contains("Destroy"), "{}", writer.body);
    assert!(writer.notes.iter().any(|note| note.message.contains("not in a base for sharing")), "{:?}", writer.notes);
}
