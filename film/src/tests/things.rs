//! Things: in the hands, on the stage, in a fight.

use super::*;

#[test]
fn items() {
    wrote(&scene(&["krabby hold Apple"]), &[&["hold", "i109", "krabby"]]);
    wrote(&scene(&["krabby hold Apple", "krabby drop"]), &[&["hold", "i109", "off"]]);
    has(&scene(&["krabby eat Apple"]), "anim");
    wrote(&scene(&["krabby throw Apple at piplup"]), &[&["throw", "i109", "piplup", "24", "26"]]);
    wrote(&scene(&["krabby hold Apple", "krabby give Apple to piplup"]), &[&["give", "apple", "krabby", "piplup", "30", "keep"]]);
    told(&scene(&["krabby hold Aplpe"]), "unknown thing");
    told(&scene(&["krabby drop"]), "holds nothing");
    told(&scene(&["krabby eat"]), "holds nothing");
    told(&scene(&["krabby give Apple to piplup"]), "without holding it");
}

#[test]
fn things_of_the_game_and_of_our_own() {
    // (A picture of ours, a prop: all by their words.)
    wrote(
        &scene(&["krabby hold letter", "krabby give letter to piplup"]),
        &[&["hold", "letter", "krabby"], &["give", "letter", "krabby", "piplup", "30", "keep"]],
    );
    wrote(
        &scene(&["object put chest 200 150", "open chest", "close chest"]),
        &[&["thing", "o186.1", "name:chest"], &["anim", "o186.1", "0", "0", "0"], &["pose", "o186.1", "0", "image", "1"]],
    );
    wrote(&scene(&["object put o186 center", "object put chest2 left"]), &[&["thing", "o186.1", "name:o186"]]);
    has(&scene(&["object put bag center", "bag shift to krabby in 2s"]), "go");
    // (Thrown: it hits.)
    has(&scene(&["krabby throw Apple at piplup", "wait 1s"]), "recoil");
    // (Two in the hands, and what is let go is the one said.)
    wrote(&scene(&["krabby hold letter", "krabby hold Apple", "krabby drop letter"]), &[&["hold", "d0", "off"]]);
    told(&scene(&["wait 1s", "object put chest center"]), "bursts in mid-scene");
}

#[test]
fn a_fisher_and_what_he_pulls_out() {
    // (The fisher is a variant painted by the user: without it, nothing to try.)
    if !variant("pikachu_pecheur") {
        return;
    }
    // (Fishing, with the rod drawn in a variant's pictures, from the cast to
    // the line in.)
    wrote(
        &fisher(&["pikachu_pecheur throw line to 300 100"]),
        &[&["anim", "pikachu_pecheur", "Swing"], &["fishline", "pikachu_pecheur", "d0", "300", "100"], &["float", "d0", "1", "70"]],
    );
    wrote(&fisher(&["pikachu_pecheur throw line"]), &[&["fishline", "pikachu_pecheur", "d0"]]);
    told(&scene(&["krabby throw line"]), "has no rod");
    wrote(&fisher(&["pikachu_pecheur throw line", "bite pikachu_pecheur"]), &[&["anim", "d0", "0", "1"], &["emote", "pikachu_pecheur", "exclaim", "40"]]);
    told(&fisher(&["bite pikachu_pecheur"]), "has no line in the water");
    told(&fisher(&["haul pikachu_pecheur"]), "has no line in the water");
    wrote(
        &fisher(&["pikachu_pecheur throw line", "haul pikachu_pecheur"]),
        &[&["anim", "pikachu_pecheur", "Pull"], &["gone", "d0"], &["fishline", "pikachu_pecheur", "off"]],
    );
    wrote(
        &fisher(&["pikachu_pecheur throw line", "haul pikachu_pecheur Apple"]),
        &[&["putnear", "i109", "d0", "down"], &["fishline", "pikachu_pecheur", "i109", "hold"], &["fishline", "pikachu_pecheur", "off"]],
    );
    wrote(
        &fisher(&["pikachu_pecheur throw line", "haul pikachu_pecheur piplup"]),
        &[&["putnear", "piplup", "d0", "down"], &["fishline", "pikachu_pecheur", "piplup", "hold"]],
    );
    // (Cast to a point, what comes out comes out there, where the bobber is.)
    wrote(&fisher(&["pikachu_pecheur throw line to 300 100", "haul pikachu_pecheur piplup"]), &[&["put", "piplup", "300", "100", "down"]]);
    told(&fisher(&["pikachu_pecheur throw line", "haul pikachu_pecheur zzzz"]), "what comes out of the water?");
    wrote(
        &fisher(&["pikachu_pecheur throw line", "pack_up pikachu_pecheur"]),
        &[&["fishline", "pikachu_pecheur", "off"], &["gone", "d0"], &["pose", "pikachu_pecheur", "idle"]],
    );
    told(&scene(&["pack_up krabby"]), "has no rod");
    // (What goes on at the water: rings round the bobber, a nibble, a strike,
    // a line that snaps and leaves none, something stirring under.)
    wrote(&fisher(&["pikachu_pecheur throw line to 300 100"]), &[&["emit", "rings", "d0", "0.7", "white"]]);
    wrote(
        &fisher(&["pikachu_pecheur throw line", "nibble pikachu_pecheur"]),
        &[&["jump", "bobber", "2", "6"], &["emote", "pikachu_pecheur", "question", "40"]],
    );
    wrote(&fisher(&["pikachu_pecheur throw line", "strike pikachu_pecheur"]), &[&["particles", "drops", "d0", "24", "blue"]]);
    wrote(&fisher(&["pikachu_pecheur throw line", "snap pikachu_pecheur"]), &[&["fishline", "pikachu_pecheur", "off"], &["gone", "d0"]]);
    told(&fisher(&["pikachu_pecheur throw line", "snap pikachu_pecheur", "bite pikachu_pecheur"]), "has no line in the water");
    wrote(&fisher(&["eddy 330 100"]), &[&["particles", "rings", "@", "330", "100", "3"]]);
    told(&fisher(&["eddy here"]), "“here”");
    // (The bait on, a fish coming near, its run on the line, the catch shown
    // off; by chance, or a heavy one.)
    wrote(&fisher(&["bait pikachu_pecheur Apple"]), &[&["vanish", "i109", "8"], &["gone", "i109"]]);
    told(&fisher(&["bait pikachu_pecheur"]), "bait a <item>");
    wrote(&fisher(&["pikachu_pecheur throw line to 300 100", "pikachu_pecheur approach"]), &[&["after", "1", "particles", "rings", "@", "190", "90", "1"]]);
    wrote(
        &fisher(&["pikachu_pecheur throw line to 300 100", "struggle pikachu_pecheur 2"]),
        &[&["go", "d0", "334", "103", "22", "1"], &["go", "d0", "266", "96", "22", "1"], &["go", "d0", "300", "100", "22", "1"]],
    );
    wrote(&fisher(&["pikachu_pecheur throw line", "haul pikachu_pecheur piplup heavy"]), &[&["quake", "3", "150"]]);
    told(&fisher(&["pikachu_pecheur throw line", "haul pikachu_pecheur random"]), "fishes at random");
    wrote(
        &fisher(&["bait pikachu_pecheur Apple", "pikachu_pecheur throw line", "haul pikachu_pecheur"]),
        &[&["popup", "pikachu_pecheur", "white", "The bait is gone..."]],
    );
    wrote(
        &fisher(&["pikachu_pecheur throw line", "haul pikachu_pecheur piplup", "big_catch pikachu_pecheur piplup What a catch!"]),
        &[&["jingle", "7"], &["popup", "piplup", "yellow", "What a catch!"]],
    );
    // (What goes on in the water by itself.)
    has(&fisher(&["school 100 80 180 80"]), "while");
    wrote(&fisher(&["krabby jump water 300 90"]), &[&["put", "krabby", "300", "90", "left"], &["hop", "krabby", "256", "90", "38", "40"], &["gone", "krabby"]]);
    told(&fisher(&["krabby jump water"]), "jump <pokemon> water x y");
    // (He fishes facing any way, and keeps it while his line is out: the lines
    // said around do not turn him; free again once it is in. He sleeps facing us.)
    wrote(
        &fisher(&["pikachu_pecheur throw line", "pack_up pikachu_pecheur"]),
        &[&["holdgaze", "pikachu_pecheur", "on"], &["holdgaze", "pikachu_pecheur", "off"]],
    );
    wrote(&fisher(&["pikachu_pecheur sleep", "wake pikachu_pecheur"]), &[&["views", "pikachu_pecheur", "down"], &["views", "pikachu_pecheur", "off"]]);
    // (A fish's shadow comes up under the bobber, past it, turns, waits; it
    // dives at the bite, or darts off.)
    let shadowed = ["pikachu_pecheur throw line to 300 100", "pikachu_pecheur shadow"];
    wrote(
        &fisher(&shadowed),
        &[&["put", "d1", "180", "92", "down"], &["layer", "d1", "under"], &["go", "d1", "322", "105", "90", "1"], &["go", "d2", "304", "104", "40", "1"]],
    );
    wrote(&fisher(&[shadowed[0], shadowed[1], "bite pikachu_pecheur"]), &[&["vanish", "d2", "8"]]);
    wrote(&fisher(&[shadowed[0], shadowed[1], "pikachu_pecheur shadow flee"]), &[&["gone", "d2"]]);
    told(&fisher(&["pikachu_pecheur throw line", "pikachu_pecheur shadow"]), "at a point of the water");
    // (The line's tension shown while the fish runs, gone with the line.)
    let taut = fisher(&["pikachu_pecheur throw line to 300 100", "struggle pikachu_pecheur 2 tension", "haul pikachu_pecheur piplup"]);
    wrote(
        &taut,
        &[&["gauge", "middle", "tension_pikachu_pecheur", "100", "red"], &["after", "23", "set", "tension_pikachu_pecheur", "84"], &["gauge", "middle", "off"]],
    );
    wrote(
        &fisher(&["pikachu_pecheur throw line to 300 100", "struggle pikachu_pecheur 2 tension", "snap pikachu_pecheur"]),
        &[&["set", "tension_pikachu_pecheur", "100"]],
    );
    // (Off the hook in the air: back into the water by where it came out.)
    wrote(
        &fisher(&["pikachu_pecheur throw line to 300 100", "haul pikachu_pecheur piplup unhooks"]),
        &[&["hop", "piplup", "314", "104", "40", "54"], &["gone", "piplup"]],
    );
    // (What was caught: measured against the record, let go, in the bucket,
    // the day told.)
    let caught = |more: &[&str]| {
        let mut lines = vec!["object put bucket center", "pikachu_pecheur throw line to 300 100", "haul pikachu_pecheur piplup"];
        lines.extend(more);
        fisher(&lines)
    };
    has(&caught(&["measure pikachu_pecheur"]), "popup");
    wrote(&caught(&["pikachu_pecheur release"]), &[&["hop", "piplup", "300", "100", "46", "44"], &["gone", "piplup"]]);
    wrote(&caught(&["bucket pikachu_pecheur"]), &[&["gone", "piplup"], &["popup", "d0", "white", "x1"]]);
    told(&caught(&["pikachu_pecheur release", "bucket pikachu_pecheur"]), "nothing caught");
    told(&fisher(&["pikachu_pecheur throw line", "haul pikachu_pecheur Apple", "measure pikachu_pecheur"]), "cannot be measured");
    wrote(&fisher(&["fishing_report pikachu_pecheur"]), &[&["say", "-", "-", "[CN]Pikachu caught nothing today."]]);
    let day = caught(&["pikachu_pecheur throw line to 300 100", "haul pikachu_pecheur Apple", "fishing_report pikachu_pecheur"]);
    assert!(engine_lines(&day).0.iter().any(|fields| fields[0] == "say" && fields[3].contains("Piplup and Apple")), "{day}");
    // (A bottle from the sea, uncorked: the letter in his hands.)
    wrote(
        &fisher(&["pikachu_pecheur throw line", "haul pikachu_pecheur bottle", "pick_up pikachu_pecheur bottle", "uncork pikachu_pecheur"]),
        &[&["popup", "d1", "white", "Pop!"], &["gone", "d1"], &["hold", "d2", "pikachu_pecheur"]],
    );
    told(&fisher(&["uncork pikachu_pecheur"]), "holds no bottle");
    // (Struck too early: what came near flees, the line stays out.)
    let early = fisher(&["pikachu_pecheur throw line to 300 100", "pikachu_pecheur shadow", "strike pikachu_pecheur too_soon", "bite pikachu_pecheur"]);
    wrote(&early, &[&["popup", "pikachu_pecheur", "white", "Too soon!"], &["gone", "d2"]]);
    // (A leap on the line in the middle of the run, the line on it.)
    wrote(
        &fisher(&["pikachu_pecheur throw line to 300 100", "struggle pikachu_pecheur 3 jump krabby"]),
        &[&["fishline", "pikachu_pecheur", "krabby", "hold"], &["hop", "krabby", "250", "96", "38", "40"], &["fishline", "pikachu_pecheur", "d0", "hold"]],
    );
    // (A cast that hooks a friend; a hook caught on the bottom.)
    wrote(
        &fisher(&["pikachu_pecheur throw line on piplup"]),
        &[&["fishline", "pikachu_pecheur", "piplup", "hold"], &["popup", "piplup", "red", "Ouch!"], &["fishline", "pikachu_pecheur", "off"]],
    );
    wrote(
        &fisher(&["pikachu_pecheur throw line", "snag pikachu_pecheur", "haul pikachu_pecheur boot"]),
        &[&["popup", "pikachu_pecheur", "white", "It's stuck..."], &["fishline", "pikachu_pecheur", "d1", "hold"]],
    );
    // (A good spot glinting; a bird making off with the catch; a photo.)
    has(&fisher(&["fishing_spot 300 100 2s"]), "parallel");
    let snatched = fisher(&[
        "pikachu_pecheur throw line to 300 100",
        "haul pikachu_pecheur krabby 330 180",
        "snatch piplup pikachu_pecheur",
        "fishing_report pikachu_pecheur",
    ]);
    wrote(
        &snatched,
        &[&["carry", "krabby", "piplup"], &["popup", "pikachu_pecheur", "red", "Hey! My catch!"], &["say", "-", "-", "[CN]Pikachu caught nothing today."]],
    );
    told(&fisher(&["snatch piplup pikachu_pecheur"]), "nothing caught");
    wrote(
        &fisher(&["photo pikachu_pecheur piplup The big day!"]),
        &[&["still", "on"], &["popup", "pikachu_pecheur", "yellow", "The big day!"], &["still", "off"]],
    );
    // (Those watching: glad at a catch, sorry at one lost.)
    let watched = fisher(&[
        "spectators piplup",
        "pikachu_pecheur throw line to 300 100",
        "haul pikachu_pecheur Apple",
        "pikachu_pecheur throw line to 300 100",
        "snap pikachu_pecheur",
    ]);
    wrote(&watched, &[&["after", "1", "emote", "piplup", "joy", "40"], &["after", "1", "emote", "piplup", "sweat", "40"]]);
    // (Out at sea on a Lokhlass: on its back, swimming, a catch onto the back,
    // a heavy one towing it, off again; a wave; its song.)
    let sea = |more: &[&str]| {
        let mut lines = vec!["lapras put 260 120", "ride pikachu_pecheur lapras", "lapras swim to 300 120 in 2s", "pikachu_pecheur throw line to 300 60"];
        lines.extend(more);
        fisher(&lines)
    };
    wrote(&sea(&[]), &[&["throw", "pikachu_pecheur", "lapras", "28", "30"], &["ride", "pikachu_pecheur", "lapras", "10", "9"], &["flies", "lapras"]]);
    wrote(&sea(&["haul pikachu_pecheur magikarp"]), &[&["throw", "magikarp", "lapras", "80", "54"], &["ride", "magikarp", "lapras", "8", "20"]]);
    wrote(
        &sea(&["haul pikachu_pecheur magikarp heavy"]),
        &[&["after", "37", "go", "lapras", "300", "114", "12", "1"], &["popup", "lapras", "white", "It's pulling us!"]],
    );
    wrote(&sea(&["dismount pikachu_pecheur 200 200"]), &[&["ride", "pikachu_pecheur", "off"], &["hop", "pikachu_pecheur", "200", "200", "28", "30"]]);
    told(&fisher(&["dismount pikachu_pecheur 200 200"]), "on nobody's back");
    wrote(&sea(&["sing lapras pikachu_pecheur"]), &[&["note", "c"]]);
    has(&sea(&["sing lapras pikachu_pecheur"]), "parallel");
    wrote(&fisher(&["surge pikachu_pecheur piplup"]), &[&["quake", "2", "40"], &["jump", "piplup", "10", "26"]]);
    wrote(&fisher(&["lapras swim from 820 120 to 400 130", "lapras plunge"]), &[&["put", "lapras", "820", "120", "down"], &["flies", "lapras"]]);
    // (`monte a 20 30` is the engine's rise, not ours.)
    wrote(&fisher(&["ride piplup 20 0.5s"]), &[&["ride", "piplup", "20", "30"]]);
    // (The rifle is in a variant's frames: levelled, its joke, loaded again.)
    wrote(
        &hunter(&["aim pikachu_chasseur piplup"]),
        &[
            &["anim", "pikachu_chasseur", "Vise", "1"],
            &["holdgaze", "pikachu_chasseur"],
            &["putnear", "d0", "piplup", "down"],
            &["face", "pikachu_chasseur", "piplup"],
        ],
    );
    wrote(&hunter(&["aim pikachu_chasseur", "aim pikachu_chasseur off"]), &[&["pose", "pikachu_chasseur", "idle"], &["holdgaze", "pikachu_chasseur", "off"]]);
    wrote(&hunter(&["fire_rifle pikachu_chasseur"]), &[&["anim", "pikachu_chasseur", "FireRifle"], &["after", "6", "sound", "hop"]]);
    // (Fired with its flag out: only a click. `tire` stays the fishing word.)
    wrote(&hunter(&["fire_rifle pikachu_chasseur", "fire_rifle pikachu_chasseur"]), &[&["emote", "pikachu_chasseur", "sweat", "30"]]);
    told(&hunter(&["haul pikachu_chasseur"]), "has no line in the water");
    wrote(
        &hunter(&["fire_rifle pikachu_chasseur", "reload pikachu_chasseur"]),
        &[&["after", "14", "pose", "pikachu_chasseur", "idle"], &["after", "14", "sound", "click"]],
    );
    told(&scene(&["aim krabby piplup"]), "has no rifle");
    // (A dungeon floor: its log, the team coming onto it, the stairs laid,
    // found and taken, a trap stepped on, an item found on the ground.)
    wrote(&scene(&["message Beach Cave, B3F."]), &[&["battlelog", "150", "Beach Cave, B3F."]]);
    wrote(&scene(&["arrival krabby piplup center"]), &[&["opacity", "krabby", "0", "0"], &["after", "14", "opacity", "piplup", "1", "16"]]);
    wrote(
        &scene(&["stairs center", "find_stairs krabby", "take_stairs krabby piplup floor 4"]),
        &[
            &["layer", "d0", "under"],
            &["face", "krabby", "d0"],
            &["vanish", "piplup", "10"],
            &["gone", "krabby"],
            &["gone", "piplup"],
            &["sound", "4873"],
            &["fadeout", "30"],
        ],
    );
    told(&scene(&["find_stairs krabby"]), "no stairs on the floor");
    wrote(&scene(&["trap krabby slumber"]), &[&["battlelog", "150", "Krabby stepped on a Slumber Trap!"], &["pose", "krabby", "sleep"]]);
    wrote(&scene(&["trap krabby trapdoor"]), &[&["vanish", "krabby", "20"]]);
    told(&scene(&["trap krabby zzz"]), "trap a <kind>");
    wrote(
        &scene(&["object put Apple center", "discover krabby Apple"]),
        &[&["hold", "i109", "krabby"], &["jingle", "7"], &["battlelog", "150", "Krabby picked up the Apple!"]],
    );
    told(&scene(&["discover krabby Apple"]), "discover a <item>");
}

#[test]
fn a_shop_on_a_floor() {
    // (A shop on the floor: its goods laid out, one bought, one stolen.)
    let film = |lines: &[&str]| format!("uses dungeon\n{}", scene(lines));
    wrote(
        &film(&["krabby shop Apple 50 Oran Berry 30", "piplup shop buy Apple", "piplup steal Oran Berry"]),
        &[&["battlelog", "150", "Krabby: Welcome to my shop!"], &["popup", "krabby", "red", "Thief!"]],
    );
    told(&film(&["krabby shop Apple"]), "shop <shopkeeper>");
}

/// A variant painted facing us only is told to keep facing us (the engine
/// keeps it: `views`); one painted all round turns as anyone.
#[test]
fn a_variant_painted_facing_us_keeps_facing_us() {
    if !variant("tiplouf_chasseur") {
        return;
    }
    // (Piplup's hunter is painted all round: no `views`, he turns.)
    let duck = |lines: &[&str]| format!("scene: town tiplouf_chasseur piplup\n{}\n", lines.join("\n"));
    assert!(!clean(&duck(&["wait 1s"])).iter().any(|fields| fields[0] == "views"));
    let played = played(&duck(&["tiplouf_chasseur walk to left", "wait 1s"]));
    let code = &played.0.names.iter().find(|(_, name)| name == "tiplouf_chasseur").expect("there").0;
    assert_ne!(played.1.cast[&Who::parse(code).expect("someone")].direction, 1);
    // (Any painted facing us only, by its table: told so at the scene's start.)
    let tables = crate::tests::tables();
    for (number, ways) in &tables.views {
        let word = &tables.monsters.iter().find(|(_, (id, _))| id == number).expect("named").0;
        let film = format!("scene: town {word}\nwait 1s\n");
        let fields: Vec<&str> = std::iter::once("views").chain(std::iter::once(word.as_str())).chain(ways.iter().map(String::as_str)).collect();
        wrote(&film, &[&fields]);
    }
}

#[test]
fn a_thing_in_a_directed_fight() {
    let film = "scene: beach krabby piplup\nfight directed krabby against piplup\n  krabby uses Oran Berry +30\n  krabby throws Apple piplup -20\nend\n";
    let (lines, problems) = engine_lines(film);
    assert!(problems.is_empty(), "{problems:?}");
    let fight = lines.iter().find(|fields| fields[0] == "fight").expect("the fight");
    let at = fight.iter().position(|field| field == "uses").expect("eaten");
    assert_eq!(fight[at + 3..at + 5], ["name:Oran Berry".to_string(), "30".to_string()]);
    assert!(fight.iter().any(|field| field == "throws"));
}

/// Whether a variant painted by the user (`data/hand`, `data/extra`) is there.
fn variant(word: &str) -> bool {
    crate::tests::tables().monsters.contains_key(word)
}

#[test]
fn the_pictures_words_pick_by_themselves_are_ours_and_listed() {
    // (Each one is in the repository's images, by its name.)
    let images: Vec<String> = std::fs::read_dir(crate::tests::root().join("library/images"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|file| file.file_name().to_string_lossy().split('.').next().unwrap_or_default().to_string())
        .collect();
    for (family, picture) in crate::LEXICON.own_pictures() {
        assert!(images.iter().any(|name| *name == picture), "{family}: no {picture} in library/images");
    }
}
