//! A film written `for ds`: what the DS can't play of it told, each by its
//! code; nothing told without the line.

use film_core::codes;

/// The codes a film is told, in order.
fn coded(film: &str) -> Vec<(&'static str, String)> {
    crate::tests::compile(film).problems.into_iter().filter_map(|problem| Some((problem.code?, problem.message))).collect()
}

fn told(film: &str, code: &str) -> Vec<String> {
    coded(film).into_iter().filter(|(told, _)| *told == code).map(|(_, message)| message).collect()
}

#[test]
fn for_ds_is_read_and_written_back() {
    let film = "for ds\nscene: beach krabby\nkrabby: Hi.\n";
    assert!(film_core::ds::written_for_ds(film));
    assert!(!film_core::ds::written_for_ds("scene: beach krabby\nkrabby: Hi.\n"));
    assert!(crate::tests::compile(film).problems.iter().all(|problem| problem.is_remark()), "{:?}", coded(film));
    assert_eq!(crate::formatted(film), film);
    assert!(crate::tests::compile("for ds\nscene: beach krabby\nkrabby: Hi.\n").problems.iter().all(|problem| problem.code != Some(codes::DS_HERO)));
    assert!(!crate::tests::compile("for nds\nscene: beach krabby\n").problems.is_empty(), "only the DS");
}

#[test]
fn a_word_the_ds_lacks_is_wrong_with_its_fix() {
    let film = "for ds\nscene: beach krabby pikachu\npikachu flip\n";
    let wrong = told(film, codes::DS_WORD);
    assert_eq!(wrong.len(), 1, "{:?}", coded(film));
    assert!(wrong[0].contains("“flip”") && wrong[0].contains("“face” or “fall”"), "{wrong:?}");
    // (Not written for the DS: nothing said of it.)
    assert!(told("scene: beach krabby pikachu\npikachu flip\n", codes::DS_WORD).is_empty());
}

#[test]
fn a_word_with_a_loss_says_what_is_lost_once() {
    let film = "for ds\nscene: beach krabby pikachu\npikachu dodge\nkrabby dodge\n";
    let lost = told(film, codes::DS_LOSS);
    assert_eq!(lost.len(), 1, "{:?}", coded(film));
    assert!(lost[0].contains("Missed!") && lost[0].contains("1 more line"), "{lost:?}");
    let problems = crate::tests::compile(film).problems;
    let loss = problems.iter().find(|problem| problem.code == Some(codes::DS_LOSS)).expect("told");
    assert!(loss.is_remark());
    assert_eq!(loss.line(), 3);
    assert!(told("scene: beach krabby pikachu\npikachu dodge\n", codes::DS_LOSS).is_empty());
}

#[test]
fn too_many_at_once_says_who_is_left_out() {
    let film = "for ds\nscene: beach krabby\ncrowd pidgey 25\nkrabby: So many.\n";
    let wrong = told(film, codes::DS_ACTORS);
    assert_eq!(wrong.len(), 1, "{:?}", coded(film));
    assert!(wrong[0].contains("26 Pokémon at once") && wrong[0].contains("leaves out 5 of the 25 idle"), "{wrong:?}");
    assert!(told("for ds\nscene: beach krabby\ncrowd pidgey 12\n", codes::DS_ACTORS).is_empty());
    assert!(told("scene: beach krabby\ncrowd pidgey 25\n", codes::DS_ACTORS).is_empty());
    // (Each scene counted alone: the bodies of every scene told at the start are no one there.)
    let two = "for ds\nscene: beach krabby\ncrowd pidgey 12\ntransition fade\nscene: forest caterpie\ncrowd weedle 12\n";
    assert!(told(two, codes::DS_ACTORS).is_empty(), "{:?}", coded(two));
}

#[test]
fn a_place_of_ones_own_is_a_map_of_its_own_on_ds() {
    let tables = crate::tests::tables();
    // (Without one's own game's data, what Sky has is not known.)
    if tables.sky_levels.is_empty() {
        return;
    }
    assert!(told("for ds\nscene: beach krabby\nkrabby: Hi.\n", codes::DS_PLACE).is_empty());
    let Some(own) = tables.decors.keys().find(|name| !tables.sky_levels.contains(&tables.decors[*name].0)) else { return };
    let film = format!("for ds\nscene: {own} krabby\nkrabby: Hi.\n");
    assert!(told(&film, codes::DS_PLACE).is_empty(), "{:?}", coded(&film));
    let lost = told(&film, codes::DS_LOSS);
    assert!(lost.iter().any(|said| said.contains("a place of your own")), "{:?}", coded(&film));
}

#[test]
fn the_hero_is_not_chosen_yet_as_the_ds_boots() {
    let film = "for ds\nscene: beach hero krabby\nhero: Hi.\nkrabby: \"hero\" you say?\n";
    let wrong = told(film, codes::DS_HERO);
    assert_eq!(wrong.len(), 2, "the scene's line and the hero's, not what krabby says: {:?}", coded(film));
    assert!(wrong[0].contains("for ds scene"));
    assert!(told(&film.replace("for ds\n", "for ds scene\n"), codes::DS_HERO).is_empty(), "in a game going on");
}

#[test]
fn a_zoom_or_a_wide_frame_would_step_back() {
    let zoom = "for ds\nscene: beach krabby\nzoom 0.5 1\n";
    assert_eq!(told(zoom, codes::DS_ZOOM).len(), 1, "{:?}", coded(zoom));
    assert!(told(zoom, codes::DS_WORD).is_empty(), "said once");
    let wide = "for ds\nscene: beach krabby pikachu\nkrabby put 100 200\npikachu put 600 200\nframe krabby pikachu\n";
    assert_eq!(told(wide, codes::DS_ZOOM).len(), 1, "{:?}", coded(wide));
    let near = "for ds\nscene: beach krabby pikachu\nkrabby put 300 200\npikachu put 340 200\nframe krabby pikachu\n";
    assert!(told(near, codes::DS_ZOOM).is_empty(), "{:?}", coded(near));
}

/// None idle: the export still makes room, leaving out those only acting,
/// then those moving, never one who speaks; who, said.
#[test]
fn a_crowd_with_no_one_idle_leaves_out_who_acts_least() {
    let names = "bulbasaur charmander charizard squirtle caterpie metapod butterfree weedle kakuna beedrill pidgey pidgeot rattata spearow ekans pikachu clefairy vulpix jigglypuff oddish diglett meowth poliwag abra";
    let hops: Vec<String> = names.split(' ').skip(1).map(|name| format!("{name} hop")).collect();
    let film = format!("for ds\nscene: beach {names}\n{}\nbulbasaur: So many.\n", hops.join("\n"));
    let wrong = told(&film, codes::DS_ACTORS);
    assert_eq!(wrong.len(), 1, "{:?}", coded(&film));
    assert!(!wrong[0].contains("idle") && !wrong[0].contains("still"), "{wrong:?}");
    assert!(wrong[0].contains("leaves out 3 of the 23 only acting") && !wrong[0].contains("bulbasaur"), "{wrong:?}");
}

/// A line said in a bubble: the DS's box, closing by itself (once a film).
#[test]
fn a_bubble_line_is_a_box_line_on_ds() {
    let film = "for ds\nscene: beach krabby pikachu\npikachu (bubble, happy): Look!\nkrabby (bubble): Hm.\npikachu bubble Wow\n";
    let lost = told(film, codes::DS_LOSS);
    let line = lost.iter().find(|said| said.starts_with("a line said in a bubble")).expect("told");
    assert!(line.contains("one at a time") && line.contains("1 more line"), "{lost:?}");
    assert!(lost.iter().any(|said| said.starts_with("“bubble”")), "the word too: {lost:?}");
    assert!(told(film, codes::DS_WORD).is_empty(), "{:?}", coded(film));
}

/// An engine's command the export has nothing for: left out, told; an item
/// put, held, on a path: shown on DS (an object of the scene's), not told.
#[test]
fn what_the_export_leaves_out_is_told() {
    let film = "for ds\nscene: beach krabby pikachu\nput object apple 300 200\nkrabby shadow off\npikachu hold Apple\ntrackpath i109 30 catmull linear / 300 200 / 320 180\n";
    let left = told(film, codes::DS_LEFT);
    assert!(left.iter().any(|said| said.contains("“shadow”") && said.contains("leaves it out")), "{:?}", coded(film));
    assert_eq!(left.len(), 1, "the items shown: {:?}", coded(film));
    assert!(told(film, codes::DS_WORD).is_empty(), "{:?}", coded(film));
    assert!(told("scene: beach krabby pikachu\nkrabby shadow off\n", codes::DS_LEFT).is_empty());
}

/// A form SpriteCollab adds is put in the ROM where the data has its sheets
/// (nothing told), else drawn as its base species on DS: said where it is
/// first named (where the data has the form).
#[test]
fn a_form_the_rom_lacks_is_told_as_its_base() {
    let tables = crate::tests::tables();
    if !tables.monsters.contains_key("pichu_spiky") {
        return;
    }
    let film = "for ds\nscene: beach pichu_spiky\npichu_spiky: Mine!\n";
    let drawn = told(film, codes::DS_SPECIES);
    if matches!(film_core::ds::drawn_on_ds(&tables.data, 5257), film_core::ds::Drawn::Added(_)) {
        assert!(drawn.is_empty(), "put in the ROM from its sheets: {drawn:?}");
        return;
    }
    assert_eq!(drawn.len(), 1, "{:?}", coded(film));
    assert!(drawn[0].contains("“pichu_spiky”") && drawn[0].contains("drawn as Pichu"), "{drawn:?}");
    assert!(told("for ds\nscene: beach pichu\npichu: Mine!\n", codes::DS_SPECIES).is_empty());
}

/// A pose no sprite of Sky's plays: the nearest one instead, told once.
#[test]
fn a_pose_the_sprite_lacks_is_told_with_the_one_instead() {
    let film = "for ds\nscene: beach pikachu\npikachu celebrate\npikachu celebrate\n";
    let lost = told(film, codes::DS_LOSS);
    assert!(lost.iter().any(|said| said.contains("has no Rotate: Hop instead") && said.contains("1 more line")), "{:?}", coded(film));
}

/// Pikachu and 20 of Sky's largest sprites in a scene: over the room the DS
/// has for its sprites (with the game's sizes or the guesses), told once
/// on the scene with the species, the bytes and the room, what the export
/// does (splits it at a moment where both parts fit, else leaves out the
/// least needed) and how to keep them all; nothing told when they fit.
#[test]
fn species_past_the_sprites_room_are_told_with_what_the_export_does() {
    let legends = "pikachu groudon kyogre rayquaza dialga palkia giratina lugia snorlax wailord steelix gyarados dragonite tyranitar metagross salamence garchomp mewtwo regigigas heatran lapras";
    let all: Vec<&str> = legends.split(' ').collect();
    let film = |moments: &[&[&str]]| -> String {
        let mut film = format!("for ds\nscene: town {legends}\n");
        for (at, one) in all.iter().enumerate() {
            film += &format!("{one} put {} 170\n", 40 + 24 * at);
        }
        for (at, moment) in moments.iter().enumerate() {
            film += &format!("moment: part {}\n", at + 1);
            for one in *moment {
                film += &format!("{one}: Here.\n");
            }
        }
        film
    };
    let split = told(&film(&[&all[..10], &all[10..]]), codes::DS_SPRITES);
    assert_eq!(split.len(), 1, "{split:?}");
    assert!(split[0].starts_with("21 species in this scene") && split[0].contains("KB") && split[0].contains("magenta squares"), "{split:?}");
    assert!(split[0].contains("splits it into two DS scenes at the moment “part 2”") && split[0].contains("split the scene"), "{split:?}");
    let problems = crate::tests::compile(&film(&[&all[..10], &all[10..]])).problems;
    assert!(problems.iter().filter(|problem| problem.code == Some(codes::DS_SPRITES)).all(|problem| problem.is_remark() && problem.line() == 2));
    let trimmed = told(&film(&[&all[..2], &all]), codes::DS_SPRITES);
    assert!(trimmed.len() == 1 && trimmed[0].contains("the export leaves out the least needed"), "{trimmed:?}");
    // (A few species fit; not written for the DS, nothing said.)
    assert!(told("for ds\nscene: town pikachu groudon kyogre\npikachu: Hi.\ngroudon: Hi.\n", codes::DS_SPRITES).is_empty());
    assert!(told(&film(&[&all[..10], &all[10..]]).replacen("for ds\n", "", 1), codes::DS_SPRITES).is_empty());
}
