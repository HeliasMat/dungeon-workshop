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
    let film = "for ds\nscene: beach krabby pikachu\npikachu bubble Look!\n";
    let wrong = told(film, codes::DS_WORD);
    assert_eq!(wrong.len(), 1, "{:?}", coded(film));
    assert!(wrong[0].contains("“bubble”") && wrong[0].contains("who: text"), "{wrong:?}");
    // (Not written for the DS: nothing said of it.)
    assert!(told("scene: beach krabby pikachu\npikachu bubble Look!\n", codes::DS_WORD).is_empty());
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
    assert!(wrong[0].contains("26 Pokémon at once") && wrong[0].contains("leaves out 5 of the 25 idle ones"), "{wrong:?}");
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
