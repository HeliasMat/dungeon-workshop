//! What the rules told this frame, seen to; and the time going by for
//! all that shows.

use std::collections::HashMap;

use rules::dungeon::stack;
use rules::run::{Monster, MonsterId, Side, Sight};
use shell::art::Art;
use shell::images::{Anim, read_together};

use super::actors::{Actor, looks_of};
use super::shows::{Digits, Place, number_picture, over};
use super::{sound, tile_middle};
use crate::Dive;
use crate::Turn;

/// The sounds of a miss (`PlayMissSfx`): a foe's, a team member's.
const MISS_FOE: u16 = 0x1306;
const MISS_TEAM: u16 = 0x1307;

/// Those of the five animations of dungeons.
const OF_DUNGEONS: [usize; 5] = [0, 1, 2, 3, 4];

/// A new floor: all of the old one goes; those on it are read at once, those
/// who may come on the side.
fn new_floor(game: &mut Turn<'_, Dive>) {
    let dive = &mut *game.here;
    dive.actors.clear();
    dive.shows.clear();
    dive.looks.forget();
    dive.picture.stale();
    game.show.art.forget_sprites();
    let Some(dungeon) = game.run.dungeon() else {
        return;
    };
    for monster in &dungeon.monsters {
        dive.actors.insert(monster.id, Actor::new(monster, &game.show.art));
    }
    let sprites = dive.actors.values().filter_map(|actor| actor.sprite.as_ref());
    read_together(sprites.flat_map(|sprite| OF_DUNGEONS.map(|at| (sprite.as_ref(), at))));
    let species = dungeon.id.map_or(Vec::new(), |id| game.run.book.floor_species(id, dungeon.floor_number));
    let coming = species.into_iter().filter_map(|species| game.show.art.monster(species));
    dive.behind.read(coming.flat_map(|sprite| OF_DUNGEONS.map(|at| (sprite.clone(), at))).collect());
}

fn actor<'a>(actors: &'a mut HashMap<MonsterId, Actor>, monster: &Monster, art: &Art) -> &'a mut Actor {
    actors.entry(monster.id).or_insert_with(|| Actor::new(monster, art))
}

pub fn see(game: &mut Turn<'_, Dive>, told: Vec<Sight>) {
    game.here.shows.new_sights();
    for sight in told {
        see_one(game, sight);
    }
}

fn see_one(game: &mut Turn<'_, Dive>, sight: Sight) {
    let mut heard = Vec::new();
    match sight {
        Sight::NewFloor => new_floor(game),
        Sight::TilesChanged => game.here.picture.stale(),
        Sight::Music(song) => {
            game.here.song = song;
            play_song(game);
        }
        Sight::Save(text) => {
            let told = match std::fs::write(game.show.quicksave_path(), text) {
                Ok(()) => "Game saved.".to_string(),
                Err(error) => format!("Could not save: {error}"),
            };
            game.run.say(told);
        }
        Sight::Sound(id) => heard.push(id),
        Sight::Jingle(id) => {
            if let Some(audio) = game.show.audio.as_ref().filter(|_| game.run.options().volume > 0) {
                audio.jingle(id);
            }
        }
        Sight::BagDone => game.here.bag.close(),
        Sight::Fainted(_) => {}
        sight => {
            let Some(dungeon) = game.run.dungeon() else {
                return;
            };
            let (dive, art) = (&mut *game.here, &game.show.art);
            let tables = &game.run.book.tables;
            match sight {
                Sight::Strike(who) => {
                    if let Some(monster) = dungeon.monster(who) {
                        actor(&mut dive.actors, monster, art).start(Anim::Attack, true);
                    }
                }
                Sight::Hurt(who, frames) => {
                    if let Some(monster) = dungeon.monster(who) {
                        let delay = dive.shows.landing_on(monster.at).unwrap_or(frames as f32);
                        actor(&mut dive.actors, monster, art).hurt_in = Some(delay);
                    }
                }
                Sight::Number(who, amount) => {
                    // (Not the HP that come back by themselves, one at a time.)
                    if let Some(monster) = dungeon.monster(who).filter(|_| amount != -1) {
                        let (text, row) = match amount {
                            1000.. => (format!("-{amount}"), Digits::Red),
                            1.. => (format!("-{amount}"), Digits::Yellow),
                            _ => (format!("+{}", -amount), Digits::Green),
                        };
                        let shown_at = dive.actors.get(&who).map_or(tile_middle(monster.at), |actor| actor.at);
                        let delay = dive.shows.landing_on(monster.at).unwrap_or(0.0);
                        if let Some(picture) = number_picture(art, &text, row) {
                            dive.shows.number(who, picture, over(dungeon, art, who, shown_at), delay);
                        }
                    }
                }
                Sight::Miss(who) => {
                    if let Some(monster) = dungeon.monster(who) {
                        heard.push(if matches!(monster.side, Side::Foe | Side::Keeper) { MISS_FOE } else { MISS_TEAM });
                        let shown_at = dive.actors.get(&who).map_or(tile_middle(monster.at), |actor| actor.at);
                        let delay = dive.shows.landing_on(monster.at).unwrap_or(0.0);
                        if let Some(picture) = number_picture(art, "MISS", Digits::Green) {
                            dive.shows.number(who, picture, over(dungeon, art, who, shown_at), delay);
                        }
                    }
                }
                Sight::Trap(kind, at) => {
                    if let Some(&effect) = tables.trap_effects.get(kind) {
                        dive.shows.effect(effect, at, &mut Place { dungeon, art, tables, heard: &mut heard });
                    }
                }
                Sight::Item(item, at) => {
                    if let Some(effects) = tables.item_effects.get(stack::id(item) as usize) {
                        dive.shows.effect(effects[0], at, &mut Place { dungeon, art, tables, heard: &mut heard });
                    }
                }
                Sight::Effect(effect, at) => dive.shows.effect(effect, at, &mut Place { dungeon, art, tables, heard: &mut heard }),
                Sight::Thrown(item, from, to) => dive.shows.thrown(item, from, to),
                Sight::Move { move_id, user, from, targets } => {
                    let monster = dungeon.monster(user);
                    let (species, facing) = monster.map_or((None, (0, 1)), |monster| (looks_of(monster), monster.facing));
                    let actor = monster.map(|monster| actor(&mut dive.actors, monster, art));
                    let longest = dive.shows.used(move_id, actor, facing, species, from, &targets, &mut Place { dungeon, art, tables, heard: &mut heard });
                    // No one acts until what shows is over.
                    game.run.wait_for_show(longest as u32);
                }
                _ => {}
            }
        }
    }
    for id in heard {
        sound(&game.show.audio, game.run.options(), id);
    }
}

/// The music as the options have it.
pub fn play_song(game: &mut Turn<'_, Dive>) {
    let options = game.run.options();
    let song = game.here.song.filter(|_| options.music && options.volume > 0);
    if let Some(audio) = &mut game.show.audio {
        audio.song(0, song, options.volume as f32 / 10.0, 0.0);
    }
}

/// A frame goes by for all that shows.
pub fn advance(game: &mut Turn<'_, Dive>) {
    let Some(dungeon) = game.run.dungeon() else {
        return;
    };
    let dive = &mut *game.here;
    dive.behind.take();
    dive.actors.retain(|id, _| dungeon.monster(*id).is_some());
    for monster in &dungeon.monsters {
        actor(&mut dive.actors, monster, &game.show.art).advance(monster, &game.show.art);
    }
    let mut heard = Vec::new();
    dive.shows.advance(&mut heard);
    for id in heard {
        sound(&game.show.audio, game.run.options(), id);
    }
}
