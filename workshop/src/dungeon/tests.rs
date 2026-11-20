//! The tests of the dungeon editor: its forms written and read back.

use super::*;

#[test]
fn a_fight_of_its_own_is_read_and_written_again_and_named_by_a_dungeon() {
    let fight = "# a fight\nname: The Lair\nboss_look: Miracle Sea 2\nboss: Gyarados 34\nboss_guards: Tentacruel 28 @7,5\nboss_hp: x4\nboss_arena: 21x13\nboss_shape: pool\nboss_title: The Ancient\n";
    let form = Form::read(fight);
    let boss = form.boss.clone().expect("a boss");
    assert_eq!((form.name.as_str(), boss.look.as_str(), boss.look_floor), ("The Lair", "Miracle Sea", 2));
    assert_eq!(form.fight_text(), fight);
    let dungeon = Form::read("name: Forest\nfloors: 2\nlook: Apple Woods\npokemon: Caterpie 3\nrecruiting: yes\nupward: yes\nfight: the_lair\n");
    assert_eq!((dungeon.fight.as_str(), dungeon.boss.is_none()), ("the_lair", true));
    assert!(dungeon.text().ends_with("fight: the_lair\n"));
    // (Its arena painted: written as it is read, a tile changed by the brush.)
    let mut painted = Form::read(
        "name: Pool
boss: Gyarados 34
boss_arena: 7x6
boss_map: ......./.X...X./.~~.~~./......./.X...X./.......
",
    );
    let boss = painted.boss.as_mut().expect("a boss");
    assert_eq!((boss.tile((1, 1)), boss.tile((1, 2)), boss.tile((0, 0))), (1, 2, 0));
    boss.paint((0, 0), 1);
    assert!(painted.fight_text().contains("boss_map: X....../.X...X./"));
    // (Played alone: a dungeon of no floor, by its name.)
    assert!(game::dive::fight_alone(fight).starts_with("name: The Lair\nfloors: 0\n"));
}

#[test]
fn a_dungeon_s_text_is_read_and_written_again() {
    let text = "# a forest\nname: Firefly Forest\nfloors: 4\nlook: Beach Cave 2\nmusic: forest\npokemon: Caterpie 3 x3, Pidgey 5 floors 2-4\nitems: Apple x2, Oran Berry floor 1\nrecruiting: no\nupward: yes\nlevel: 8\nboss: Butterfree 12\nboss_guards: Metapod 8 @5,3, Kakuna 8\nboss_allies: Lapras 14 @6,7, Tentacool 9\nboss_hero: Eevee\nboss_partner: Piplup\nboss_level: 15\nboss_at: 7 2\nboss_team_at: 7 7\nboss_floor_items: Sitrus Berry @3,4, Blast Seed @10,6\nboss_traps: spikes @4,4, slumber @9,5 hidden\nboss_hp: x2.5\nboss_arena: 15x9\nboss_shape: pillars\nboss_music: battle\nboss_says: You shall go no further!\nboss_falls: Well played…\nboss_moves: Gust, Sleep Powder\nboss_phase: 50% | says Enough games! | anger +2\nboss_phase: 40% | says To me! | reinforcements Caterpie 6, Weedle 6 | weather hail\nboss_phase: 25% | heal 30% | moves Hyper Beam\nboss_recruit: yes\nboss_item: Golden Apple\n";
    let form = Form::read(text);
    let boss = form.boss.clone().expect("a boss");
    assert_eq!(
        (boss.species.as_str(), boss.level, boss.guards.len(), boss.life, boss.room, boss.shape.as_str()),
        ("Butterfree", 12, 2, 2.5, (15, 9), "pillars")
    );
    assert_eq!(
        (boss.phases.len(), boss.phases[0].fiercer, boss.phases[1].help.len(), boss.phases[1].weather.as_str(), boss.phases[2].heal, boss.joins),
        (3, 2, 2, "hail", 30, true)
    );
    assert_eq!((boss.guards[0].2, boss.guards[1].2, boss.place, boss.team), (Some((5, 3)), None, Some((7, 2)), Some((7, 7))));
    assert_eq!(
        (boss.allies[0].clone(), boss.allies[1].2, boss.hero.as_str(), boss.partner.as_str(), boss.team_level),
        (("Lapras".to_string(), 14, Some((6, 7))), None, "Eevee", "Piplup", Some(15))
    );
    assert_eq!((boss.lying.len(), boss.traps.clone()), (2, vec![("spikes".to_string(), (4, 4), false), ("slumber".to_string(), (9, 5), true)]));
    // (The lines of before are read as phases.)
    let before = Form::read("boss: Butterfree 12\nboss_anger: +2 Enough games!\nboss_reinforcements: 40% Caterpie 6 : To me!\nboss_heal: 25% 30%\n");
    let phases = before.boss.expect("a boss").phases;
    assert_eq!(
        phases.iter().map(|phase| (phase.share, phase.fiercer, phase.help.len(), phase.heal)).collect::<Vec<_>>(),
        vec![(50, 2, 0, 0), (40, 0, 1, 0), (25, 0, 0, 30)]
    );
    assert_eq!((form.floors, form.look.as_str(), form.look_floor, form.recruit), (4, "Beach Cave", 2, false));
    assert_eq!(form.foes[0], Foe { species: "Caterpie".to_string(), level: 3, often: 3, floors: None });
    assert_eq!(form.foes[1], Foe { species: "Pidgey".to_string(), level: 5, often: 1, floors: Some((2, 4)) });
    assert_eq!(form.text(), text);
}

#[test]
fn the_options_of_its_rules_and_its_types_are_kept() {
    let text = "name: Fairy Woods\nfloors: 2\nlook: Apple Woods\npokemon: Jigglypuff 5\nrecruiting: yes\nupward: yes\nrules: fairy, push_allies\npokemon_type: Jigglypuff normal fairy\nmove_type: Charm fairy\n";
    let mut form = Form::read(text);
    assert!(form.options.fairy && form.options.push_allies && !form.options.team_control);
    assert_eq!(form.text(), text);
    form.options = Default::default();
    assert!(!form.text().contains("rules"));
}

/// The floors' grid: a click grows or shrinks a range of floors, a right click all; who is met on a floor, how often; the floors with no one.
#[test]
fn the_floors_grid_answers_clicks() {
    use super::dungeon_columns::{empty_floors, floors_clicked, format_boss_floor, met_on_floor, on_floor};
    assert_eq!(floors_clicked(None, 3, false), Some((3, 3)));
    assert_eq!(floors_clicked(Some((3, 5)), 1, false), Some((1, 5)), "below: grows down");
    assert_eq!(floors_clicked(Some((3, 5)), 8, false), Some((3, 8)), "above: grows up");
    assert_eq!(floors_clicked(Some((3, 5)), 3, false), Some((4, 5)), "its first: shrinks");
    assert_eq!(floors_clicked(Some((3, 5)), 5, false), Some((3, 4)), "its last: shrinks");
    assert_eq!(floors_clicked(Some((3, 5)), 4, false), Some((4, 4)), "inside: that one alone");
    assert_eq!(floors_clicked(Some((4, 4)), 4, false), Some((4, 4)), "one floor stays");
    assert_eq!(floors_clicked(Some((3, 5)), 4, true), None, "right click: all");
    assert!(on_floor(None, 9) && on_floor(Some((2, 4)), 4) && !on_floor(Some((2, 4)), 5));
    let foes = vec![
        Foe { species: "Caterpie".to_string(), level: 3, often: 3, floors: Some((1, 2)) },
        Foe { species: "Pidgey".to_string(), level: 4, often: 1, floors: Some((2, 2)) },
    ];
    assert_eq!(met_on_floor(&foes, 2), "floor 2\nCaterpie Lv. 3: 75 %\nPidgey Lv. 4: 25 %");
    assert_eq!(met_on_floor(&foes, 3), "floor 3\nno one");
    assert_eq!(empty_floors(&foes, 4), vec![3, 4]);
    assert_eq!(format_boss_floor("Gyarados", 34, 2), "at the end: Gyarados Lv. 34 (and 2 guards)");
}

/// A fight in numbers: how hard it looks by its ratio; the ratio rises with the foes' strength; the phases' shares.
#[test]
fn a_fight_is_weighed_and_its_phases_placed() {
    use super::fight_columns::{first_anger, next_phase_share};
    use super::layout::{Fighter, rough_ratio, verdict_of};
    assert_eq!(verdict_of(0.2).0, "easy");
    assert_eq!(verdict_of(0.5).0, "balanced");
    assert_eq!(verdict_of(1.0).0, "hard");
    assert_eq!(verdict_of(2.0).0, "very hard");
    let one = |hp: i32, attack: i32| Fighter { name: String::new(), level: 10, hp, attack, defense: 10, sp_attack: 0, sp_defense: 10 };
    let (boss, weak_boss, hero) = (one(200, 40), one(50, 20), one(60, 30));
    let hard = rough_ratio(&[&boss], &[&hero], 0);
    let easy = rough_ratio(&[&weak_boss], &[&hero], 0);
    assert!(easy < hard, "{easy} < {hard}");
    assert!(rough_ratio(&[&boss], &[&hero], 2) > hard, "reinforcements make it harder");
    let phases = vec![PhaseForm { share: 60, ..PhaseForm::default() }, PhaseForm { share: 30, fiercer: 1, ..PhaseForm::default() }];
    assert_eq!(next_phase_share(&phases), 15);
    assert_eq!(next_phase_share(&[]), 50);
    assert_eq!(first_anger(&phases), Some(30));
}

#[test]
fn a_game_s_own_dungeon_is_listed_as_its_own_and_written_back_in_its_folder() {
    let session = std::sync::Arc::new(crate::session::Session::for_tests());
    let game = session.library().at(session.root().join("tests/games/test_village"));
    let listed = summaries_in(&session, &game);
    let forest = listed.iter().find(|one| one.word == "test_forest").expect("the game's dungeon listed");
    assert!(forest.own && forest.path.starts_with(game.folder()), "{:?}", forest.path);
    assert!(summaries(&session).iter().all(|one| !one.own), "the library's are not a game's");
    let making = DungeonMaking::open_in(&session, &game, "test_forest");
    assert!(making.own() && making.saved);
    assert_eq!(making.path(), game.dungeons().join("test_forest.dungeon"));
    assert!(making.form.name.contains("Test Forest"), "read from the game's file: {:?}", making.form.name);
}
