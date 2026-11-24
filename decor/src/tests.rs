//! The places: the language and its errors, shadows, walls, remarks; with the
//! game's data (data/rom), that a build is the same each time, the pieces
//! cut as their options say, and the sheets written.

use crate::build::{Builder, Place, ragged_zone};
use crate::catalogue::KINDS;
use crate::cut::cut;
use crate::fault::Location;
use crate::lang::{Decor, parse, points_of, read_path, zone_of};
use crate::matte::{Shade, shaded, thing_shadow};
use crate::pixels::{Grid, Mask, Rgba, grown};
use crate::random::Rng;
use crate::remarks::{grouped, joins, reach};
use crate::weave::{Workshop, rounded, turned};

fn words(said: &[&str]) -> Vec<String> {
    said.iter().map(|word| word.to_string()).collect()
}

fn wrong(text: &str) -> String {
    parse(text, "c.decor").map(|_| String::new()).unwrap_or_else(|why| why.to_string())
}

#[test]
fn reads_a_place() {
    let made = parse(
        "# a comment\ndecor: crique 64 128\nlevel 470\nname The Cove\ncenter 32 40\n\nground sable_plage\nput rocher_plage 8 16, 24 32\n",
        "crique.decor",
    )
    .expect("read");
    assert_eq!(
        (made.name.as_str(), made.width, made.height, made.level, made.shown.as_deref(), made.middle),
        ("crique", 64, 128, Some(470), Some("The Cove"), Some((32, 40)))
    );
    let lines: Vec<(usize, &str, &[String])> = made.lines.iter().map(|line| (line.number, line.verb.as_str(), line.words.as_slice())).collect();
    assert_eq!(lines, [(7, "ground", &words(&["sable_plage"])[..]), (8, "put", &words(&["rocher_plage", "8", "16,", "24", "32"])[..])]);
    assert_eq!(points_of(&Location::in_file("x", 1), &words(&["8", "16,", "24", "32"])).expect("points"), [(8, 16), (24, 32)]);
}

#[test]
fn modifiers_and_zones() {
    let made = parse("decor: c 64 64\nentrance 8 56 down\nground herbe_bourg zone 0 0 32 32\nput tente_bourg 8 8 flip shadow off layer over\nscatter touffe_bourg 3 zone 0 0 64 64 flip\n", "c.decor").expect("read");
    assert_eq!(made.entrances, [(2, 8, 56, "down".to_string())]);
    assert_eq!(zone_of(&Location::in_file("x", 1), &made.lines[0].words[1..], "").expect("zone"), (0, 0, 32, 32));
    let tent = &made.lines[1];
    assert_eq!((tent.words.clone(), tent.mirrored, tent.shadow, tent.layer.as_deref()), (words(&["tente_bourg", "8", "8"]), true, false, Some("over")));
    assert_eq!(made.lines[2].words, words(&["touffe_bourg", "3", "zone", "0", "0", "64", "64"]));
    assert!(made.lines[2].mirrored);
}

#[test]
fn three_layers_spots_and_our_own_pictures() {
    let text = "decor: c 64 64\nput a 1 2 layer ground\nput b 3 4 layer objects\nimage rowboat.png 5 6 layer over flip\nspot puits 30 40\n";
    let made = parse(text, "c.decor").expect("read");
    let layers: Vec<Option<&str>> = made.lines.iter().map(|line| line.layer.as_deref()).collect();
    assert_eq!(layers, [Some("ground"), None, Some("over")]);
    assert!(made.lines[2].mirrored && made.lines[2].verb == "image");
    assert_eq!(made.entrances, [(5, 30, 40, "puits".to_string())]);
    assert_eq!(crate::doc::layer_of_line("put a 1 2 layer over"), Some("over"));
    assert_eq!(crate::doc::spots(text), [(5, "puits".to_string(), [30.0, 40.0])]);
    if let Some(picture) = crate::build::own_picture(&project(), &data(), "rowboat.png") {
        assert!(picture.w > 0 && picture.cells.iter().any(|rgba| rgba[3] > 0));
    }
}

#[test]
fn errors_say_the_line_and_the_closest_word() {
    let unknown = wrong("decor: c 64 64\n\npux x 1 2\n");
    assert!(unknown.starts_with("c.decor:3: unknown word \"pux\"") && unknown.contains("put"), "{unknown}");
    assert!(wrong("decor: c 8 64\n").starts_with("c.decor:1: width and height from 16"));
    assert!(wrong("decor: c 60 61\n").is_empty(), "any size");
    assert!(wrong("ground herbe\n").starts_with("c.decor:1: the first line"));
    let modifier = wrong("decor: c 64 64\nput x 1 2 flp\n");
    assert!(modifier.starts_with("c.decor:2: \"flp\"?") && modifier.contains("flip"), "{modifier}");
    assert!(wrong("decor: c 64 64\nput x 1 2 layer middle\n").contains("layer ground|objects|over"));
}

#[test]
fn paths() {
    let read = |said: &str| read_path(&Location::in_file("x", 1), &said.split_whitespace().map(str::to_string).collect::<Vec<_>>());
    assert_eq!(read("from 0 0 to 10 0 to 10 10 width 16").expect("a path"), (vec![(0, 0), (10, 0), (10, 10)], 16));
    assert!(read("to 0 0 from 1 1").is_err());
    assert!(read("from 0 0").is_err());
}

#[test]
fn a_piece_laid_is_at_the_pixel_and_reads_back() {
    let line = crate::lang::laid("rocher_plage", 269.4, 213.0);
    assert_eq!(line, "put rocher_plage 269 213");
    let made = parse(
        &format!(
            "decor: c 576 384
{line}
"
        ),
        "c.decor",
    )
    .expect("read");
    assert_eq!(made.lines[0].words, words(&["rocher_plage", "269", "213"]));
    assert_eq!(crate::lang::laid("x", -5.0, 3.0), "put x -5 3");
}

#[test]
fn a_new_place_reads() {
    let made = parse(&crate::doc::made("fresh", "", [321, 240], Some(500), "herbe_bourg", None, None), "fresh.decor").expect("read");
    assert_eq!((made.name.as_str(), made.width, made.height, made.level, made.shown.as_deref()), ("fresh", 321, 240, Some(500), Some("Fresh")));
    assert!(!made.entrances.is_empty());
}

fn even(w: usize, h: usize, rgb: [u8; 3]) -> Vec<Rgba> {
    vec![[rgb[0], rgb[1], rgb[2], 255]; w * h]
}

#[test]
fn a_faint_shade_leaves_another_ground_as_it_is() {
    let faint = Shade { factor: Grid::filled(2, 2, 1.05), colour: Grid::new(2, 2), grounds: Vec::new() };
    let mut sand = even(2, 2, [200; 3]);
    shaded(&mut sand, &faint, &Default::default());
    assert_eq!(sand, even(2, 2, [200; 3]));
}

#[test]
fn things_of_the_scenes_shadow_as_dark_everywhere() {
    let mask = Grid { w: 4, h: 4, cells: (0..16).map(|at| at >= 8).collect() };
    let mut ground = even(4, 4, [247; 3]);
    shaded(&mut ground, &thing_shadow(&mask), &Default::default());
    assert!(ground[..8].iter().all(|rgba| rgba[0] == 247) && ground[8..].iter().all(|rgba| rgba[0] < 247 && (rgba[0] as i32 - 7) % 8 == 0));
}

#[test]
fn turned_tiles_and_rounded_paths() {
    let tile: Vec<usize> = (0..64).collect();
    assert_eq!(turned(&tile, "t")[8], tile[1]);
    assert_eq!(turned(&tile, "h")[0], tile[7]);
    assert_eq!(turned(&tile, "v")[0], tile[56]);
    let points = rounded(&[(0, 0), (100, 0), (100, 100)]);
    assert_eq!((points[0], points[points.len() - 1]), ((0.0, 0.0), (100.0, 100.0)));
    assert!(points.len() > 3);
}

fn place(size: usize) -> Place {
    Place::new(Decor { name: "trial".to_string(), width: size, height: size, ..Decor::default() }, 1)
}

#[test]
fn a_place_of_any_size_is_cut_to_it_its_walls_by_whole_tiles() {
    let mut made = place(32);
    made.cut_to(20, 13);
    assert_eq!((made.lower[0].w, made.lower[0].h, made.owner.w, made.path.h), (20, 13, 20, 13));
    assert_eq!((made.walls.w, made.walls.h, made.decor.width), (4, 4, 20));
}

#[test]
fn a_third_of_a_tile_is_a_wall() {
    let mut place = place(32);
    let solid = Grid { w: 16, h: 8, cells: (0..128).map(|at| at % 16 < 3 || (8..10).contains(&(at % 16))).collect() };
    place.wall(&Mask::filled(16, 8, true), &solid, (0, 0));
    assert_eq!((place.walls.get(0, 0), place.walls.get(1, 0)), (true, false));
}

#[test]
fn walls_add_up_and_whole_tiles_say_again() {
    let mut place = place(32);
    place.wall(&Mask::filled(8, 8, true), &Mask::filled(8, 8, true), (0, 0));
    let part = Grid { w: 8, h: 8, cells: (0..64).map(|at| at < 32).collect() };
    place.wall(&part, &Mask::new(8, 8), (0, 0));
    assert!(place.walls.get(0, 0));
    place.wall(&Mask::filled(8, 8, true), &Mask::new(8, 8), (0, 0));
    assert!(!place.walls.get(0, 0));
}

#[test]
fn growing_does_not_wrap_round() {
    let mut mask = Mask::new(6, 6);
    mask.set(2, 5, true);
    let out = grown(&mask, 1, 1);
    assert!((0..6).all(|x| !out.get(x, 0)));
    assert_eq!(out.count(), 4);
}

#[test]
fn a_zone_has_a_ragged_edge() {
    let wanted = ragged_zone(10, 10, (16, 16, 64, 64), &mut Rng::new(&[1]));
    assert!((3..7).all(|y| (3..7).all(|x| wanted.get(x, y) == 0)));
    assert!((0..10).all(|x| wanted.get(x, 0) == -1));
    assert!((1..9).flat_map(|y| (1..9).map(move |x| (x, y))).filter(|(x, y)| wanted.get(*x, *y) == 0).count() < 64);
}

#[test]
fn a_gap_between_trees_is_not_told() {
    let mut place = place(64);
    place.walls = Mask::filled(8, 8, true);
    place.tree_walls = Mask::filled(8, 8, true);
    for y in 0..8 {
        place.walls.set(0, y, false);
    }
    for (x, y) in [(3, 3), (4, 3), (3, 4), (4, 4)] {
        place.walls.set(x, y, false);
    }
    reach(&mut place);
    assert!(place.remarks.is_empty(), "{:?}", place.remarks);
    place.tree_walls = Mask::new(8, 8);
    reach(&mut place);
    assert_eq!(place.remarks.len(), 1);
}

// --- With the game's data ---------------------------------------------------------------------

/// The repository's shared library (the tests run from `decor/`).
fn project() -> assets::project::Project {
    assets::testing::library()
}

/// The game's data, in the repository.
fn data() -> String {
    assets::testing::data()
}

/// What places are made with, if the game's data is there.
fn workshop() -> Option<Workshop> {
    let data = data();
    assets::data::has(&data, "backdrops/11.png").then(|| crate::workshop(&project(), &data).expect("the workshop"))
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn joins_and_remarks() {
    let workshop = workshop().expect("the extracted data/");
    let mut place = place(64);
    let fill = |place: &mut Place, (x0, y0, x1, y1): (usize, usize, usize, usize), rgb: [u8; 3], origin: i32| {
        for y in y0..y1 {
            for x in x0..x1 {
                place.lower[0].set(x, y, [rgb[0], rgb[1], rgb[2], 255]);
                place.origin.set(x, y, origin);
            }
        }
    };
    fill(&mut place, (0, 0, 64, 64), [10, 120, 10], 1);
    fill(&mut place, (16, 16, 32, 32), [200, 220, 90], 2);
    let mask = joins(&workshop, &place).expect("joins");
    assert!((16..32).all(|y| mask.get(15, y) && mask.get(16, y)));
    assert_eq!(grouped(&mask).iter().map(|group| (group.0, group.1, group.2, group.3)).collect::<Vec<_>>(), [(8, 8, 40, 40)]);
    // (A thing off the grid, small, is no join.)
    let mut place = self::place(64);
    fill(&mut place, (0, 0, 64, 64), [10, 120, 10], 1);
    fill(&mut place, (13, 19, 20, 23), [200, 40, 40], 2);
    assert!(!joins(&workshop, &place).expect("joins").any());
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_catalogue_cuts_every_named_piece() {
    let workshop = workshop().expect("the extracted data/");
    assert!(workshop.catalogue.pieces.len() > 50 && workshop.catalogue.pieces.iter().all(|piece| KINDS.contains(&piece.kind.as_str())));
    let named = crate::catalogue::Catalogue::read_named(&crate::pieces_folder(&project())).expect("pieces.txt");
    for piece in named {
        cut(&workshop, &piece).unwrap_or_else(|why| panic!("{}: {}", piece.name, why));
    }
}

// --- Dungeon floors ---------------------------------------------------------------------------

/// A floor's plan, made for that seed: its rows.
fn floor_plan(dungeon: &str, floor: i64, seed: i64) -> Vec<Vec<char>> {
    let text = crate::room::generate::make("a", dungeon, floor, seed, 32, 20).expect("made");
    text.lines().skip_while(|line| *line != "layout").skip(1).filter(|line| !line.is_empty()).map(|line| line.chars().collect()).collect()
}

/// The tiles one walks to from there, the water not walked on.
fn reached(plan: &[Vec<char>], start: (usize, usize)) -> std::collections::BTreeSet<(usize, usize)> {
    let mut seen = std::collections::BTreeSet::from([start]);
    let mut waiting = vec![start];
    while let Some((x, y)) = waiting.pop() {
        for (nx, ny) in [(x + 1, y), (x.wrapping_sub(1), y), (x, y + 1), (x, y.wrapping_sub(1))] {
            let free = plan.get(ny).and_then(|row| row.get(nx)).is_some_and(|letter| !"#~".contains(*letter));
            if free && seen.insert((nx, ny)) {
                waiting.push((nx, ny));
            }
        }
    }
    seen
}

#[test]
fn the_same_seed_the_same_floor() {
    assert_eq!(floor_plan("Beach Cave", 2, 5), floor_plan("Beach Cave", 2, 5));
    assert_ne!(floor_plan("Beach Cave", 2, 5), floor_plan("Beach Cave", 2, 6));
}

#[test]
fn every_room_and_letter_is_reached_from_the_arrival() {
    let mut letters = std::collections::BTreeSet::new();
    for seed in 1..40 {
        let plan = floor_plan("Beach Cave", 2, seed);
        let cells = || plan.iter().enumerate().flat_map(|(y, row)| row.iter().enumerate().map(move |(x, letter)| ((x, y), *letter)));
        let start = cells().find(|(_, letter)| *letter == 's').map(|(at, _)| at).expect("an arrival");
        let ground: std::collections::BTreeSet<(usize, usize)> = cells().filter(|(_, letter)| !"#~".contains(*letter)).map(|(at, _)| at).collect();
        assert_eq!(reached(&plan, start), ground, "seed {seed}");
        assert!(cells().any(|(_, letter)| letter == 'e'), "seed {seed}");
        letters.extend(cells().map(|(_, letter)| letter));
    }
    // (Some floors have a shop, some a monster house.)
    assert!(letters.contains(&'k') && letters.contains(&'m'));
}

#[test]
fn a_floor_s_head_says_it() {
    let text = crate::room::generate::make("bois_etage", "Apple Woods", 6, 13, 32, 20).expect("made");
    assert!(text.contains("dungeon: bois_etage Apple Woods 6"));
    assert!(floor_plan("Apple Woods", 6, 13).iter().all(|row| row.len() == 32));
}

#[test]
fn things_named_hidden_locked_grouped_and_walls_said_by_hand_are_read() {
    let made = parse(
        "decor: c 64 64\ngroup rochers\nput rocher_plage 8 8 name gros_rocher locked\nput rocher_plage 24 8 hidden\nend\nwall 0 0 16 16\nfree 32 32 48 48\nground sable_plage tiles 1,2 3,4\n",
        "c.decor",
    )
    .expect("read");
    let laid: Vec<(&str, Option<&str>, bool)> = made.lines.iter().map(|line| (line.verb.as_str(), line.name.as_deref(), line.locked)).collect();
    assert_eq!(laid, [("put", Some("gros_rocher"), true), ("wall", None, false), ("free", None, false), ("ground", None, false)], "the hidden one is not laid");
    assert_eq!(made.groups, [("rochers".to_string(), 2, 5)]);
    assert!(wrong("decor: c 64 64\ngroup a\nput rocher_plage 8 8\n").contains("end"));
    assert!(wrong("decor: c 64 64\nend\n").contains("group"));
    assert!(wrong("decor: c 64 64\nput rocher_plage 8 8 name gros-rocher\n").contains("name"));
    assert!(wrong("decor: c 64 64\nwall 0 0 16\n").contains("numbers"));
}

/// `decor help` says every word of the language: it is made of the very
/// tables the reader reads by.
#[test]
fn the_help_has_every_word_of_the_language() {
    let help = crate::lang::help();
    for (word, said) in crate::lang::COMMANDS.iter().chain(crate::lang::MODIFIERS.iter()) {
        assert!(help.contains(said) && help.contains(word), "\"{word}\": {said}");
    }
    for word in crate::lang::LAYERS.iter().chain(crate::lang::LIFE.iter()) {
        assert!(help.contains(word), "\"{word}\"");
    }
    for (verb, said) in crate::lang::VERBS {
        assert!(help.contains(&format!("decor {said}")), "decor {verb}");
    }
    assert!(!help.contains(".room:") && !help.contains("layer top|bottom"), "{help}");
}

/// A place of the game made again of pieces only (its rest and its
/// elements, each where the game has it) is the game's to the pixel, in
/// every frame, both layers: its pieces are right.
#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_place_of_the_game_remade_simply_is_the_same() {
    // (The simple builder, as the decors made of the game's own pieces are built.)
    let workshop = workshop().expect("the extracted data/");
    for level in [224, 323] {
        let text = crate::recreated(&project(), &data(), level, &format!("trial_{level}"), "trial").expect("its text");
        let decor = parse(&text, "trial.decor").expect("read");
        assert!(crate::build::compose::simple(&decor), "of the game's pieces alone");
        let place = crate::build::compose::composed(&workshop, decor).expect("built");
        let source = workshop.source(level).expect("the place");
        for (frame, (lower, upper)) in source.frames.iter().zip(&source.upper).enumerate() {
            let seen = |picture: &crate::pixels::Picture, over: &crate::pixels::Picture, at: usize| {
                if over.cells[at][3] > 0 { over.cells[at] } else { picture.cells[at] }
            };
            let made = (&place.lower[frame % place.lower.len()], &place.upper[frame % place.upper.len()]);
            let differ = (0..lower.cells.len()).filter(|at| seen(lower, upper, *at)[..3] != seen(made.0, made.1, *at)[..3]).count();
            assert_eq!(differ, 0, "place {level}, image {frame}: {differ} different pixels");
        }
    }
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_place_of_the_game_remade_of_its_pieces_is_the_same() {
    let workshop = workshop().expect("the extracted data/");
    for level in [224, 323] {
        let text = crate::recreated(&project(), &data(), level, &format!("trial_{level}"), "trial").expect("its text");
        assert!(!text.contains("base"), "pieces alone: {text}");
        let decor = parse(&text, "trial.decor").expect("read");
        let place = Builder::new(&workshop, decor).build().expect("built");
        let source = workshop.source(level).expect("the place");
        for (frame, (lower, upper)) in source.frames.iter().zip(&source.upper).enumerate() {
            let seen = |picture: &crate::pixels::Picture, over: &crate::pixels::Picture, at: usize| {
                if over.cells[at][3] > 0 { over.cells[at] } else { picture.cells[at] }
            };
            let made = (&place.lower[frame % place.lower.len()], &place.upper[frame % place.upper.len()]);
            let differ = (0..lower.cells.len()).filter(|at| seen(lower, upper, *at)[..3] != seen(made.0, made.1, *at)[..3]).count();
            assert_eq!(differ, 0, "place {level}, image {frame}: {differ} different pixels");
        }
    }
}

#[test]
fn a_place_s_life_says_who_is_there_when_what_they_say_and_what_the_game_does() {
    let text = "decor: a 256 192\nlevel 1\nground b\nentrance 8 96 west\n\n[life]\nresident kecleon 120 80 left if chapter >= 2\nsays if not met: Welcome!\nsays Back again?\ntalk shop\nresident elder 60 60\ntalk film the_elder\nexit 240 80 16 32 to test_harbour left\nexit 0 0 8 8 to 224\nzone pier 100 150 40 20\non talk kecleon if not met\n  set met\n  if chapter = 2\n    chapter 3\n  end\nend\non enter\n  play hello\nend\n";
    let decor = crate::lang::parse(text, "a.decor").expect("it reads");
    let kecleon = &decor.residents[0];
    assert_eq!((kecleon.condition.as_deref(), kecleon.talk.as_deref()), (Some("chapter >= 2"), Some("shop")));
    assert_eq!(kecleon.lines, ["Welcome!", "Back again?"]);
    assert_eq!(kecleon.tests, [Some("not met".to_string()), None]);
    assert_eq!(decor.residents[1].talk.as_deref(), Some("film the_elder"));
    assert_eq!(decor.ways_out.len(), 1);
    assert_eq!((decor.ways_out[0].to.as_str(), decor.ways_out[0].entrance.as_deref()), ("test_harbour", Some("left")));
    assert_eq!(decor.exits, [(0, 0, 8, 8, 224)]);
    assert_eq!(decor.zones[0].name, "pier");
    assert_eq!(decor.rules.len(), 2);
    assert_eq!(decor.rules[0].head, "on talk kecleon if not met");
    assert_eq!(decor.rules[0].body, "  set met\n  if chapter = 2\n    chapter 3\n  end\n");
    assert!(crate::lang::parse(&text.replace("on enter\n  play hello\nend\n", "on enter\n"), "a.decor").is_err(), "an `on` without its end");
}

/// Every element offered and not almost like another has its line in
/// `piece_tags.txt` (those almost like one go by its line), and every line
/// names an element offered; every place of the game (menus and visions
/// left out) and every decor has its kind in `place_kinds.txt`, and every
/// line there names one.
#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn every_element_and_place_is_labelled() {
    let workshop = workshop().expect("the extracted data/");
    let tables = crate::pieces_folder(&project());
    let text = std::fs::read_to_string(tables.join(crate::tags::PIECE_TAGS)).expect("piece_tags.txt");
    let labels = crate::tags::read_labels(&text).expect("piece_tags.txt read");
    let aside = crate::sort::set_aside(&tables);
    let offered: Vec<&str> = crate::offered(&workshop.catalogue, &aside).into_iter().map(|piece| piece.name.as_str()).collect();
    let lacking: Vec<&&str> = offered.iter().filter(|name| crate::tags::label_of(name, &labels, &aside).is_none()).collect();
    assert!(lacking.is_empty(), "{} element(s) with no line: {lacking:?}", lacking.len());
    let unknown: Vec<&String> = labels.keys().filter(|name| !offered.contains(&name.as_str())).collect();
    assert!(unknown.is_empty(), "lines of no element offered: {unknown:?}");
    let kinds = crate::tags::PlaceKinds::read(&std::fs::read_to_string(tables.join(crate::tags::PLACE_KINDS)).expect("place_kinds.txt")).expect("read");
    // (The game's own places: the ROM's list, not the decors built over it.)
    let rom = std::fs::read_to_string(std::path::Path::new(&data()).join("rom").join("backdrops.txt")).expect("backdrops.txt");
    let places: Vec<String> = rom
        .lines()
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .filter(|row| row.get(1).is_some_and(|code| !crate::sort::no_place(code)))
        .filter_map(|row| row.first().map(|level| level.to_string()))
        .collect();
    // Only the decors the repository ships (tracked by git): the user's own, made in
    // the Workshop, are theirs to label or not.
    let tracked = std::process::Command::new("git")
        .args(["ls-files", "*.decor"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default();
    let shipped = |name: &String| tracked.lines().any(|path| path.ends_with(&format!("/{name}.decor")));
    let mut decors: Vec<String> = crate::names(&project()).into_iter().filter(shipped).collect();
    decors.extend(crate::games_own_places(&project()).into_iter().map(|(_, name)| name).filter(shipped));
    let lacking: Vec<&String> = places.iter().chain(&decors).filter(|key| !kinds.by_key.contains_key(key.as_str())).collect();
    assert!(lacking.is_empty(), "places with no kind: {lacking:?}");
    let unknown: Vec<&String> = kinds.by_key.keys().filter(|key| !places.contains(key) && !decors.contains(key)).collect();
    assert!(unknown.is_empty(), "kinds of no place: {unknown:?}");
}
