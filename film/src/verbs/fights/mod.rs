//! Fights: the film writes lines, and `scenes/src/fight` rules and stages them.
//!
//! - `lines`: what a fight's line is made of;
//! - `directed`: a fight stated in advance, its block read;
//! - here: the words of a fight and the line each writes.

use engine::Cam;
use scenes::vocabulary::POKEMON;

use super::said::told_or_logged;
use super::{Line, Word};
use crate::film::Film;
use crate::words::{digits, plain};
use crate::{emit, fields};

mod state;
pub use state::Combat;
mod blows;
mod directed;
mod lines;

use directed::directed;
use lines::*;

pub const WORDS: &[Word] = &[
    Word::new(
        "fight",
        &[
            "*side_team vs *others [duel|double|relay] [boss] [level] [n:number] [winner wins:pokemon]",
            "directed *side_team against *others [level] [n:number] [winner wins:pokemon]",
            "directed rest...",
            "who:pokemon lv:number hp:number against other:pokemon hp2:number",
            "who:pokemon lv:number hp:number [floor:word]",
        ],
        "fight open (bars, log); fight directed: fight directed",
    )
    .maybe(fought_word),
    Word::new("join", &["who:pokemon [team]"], "emote, jump, the game's sentence").maybe(fought_word),
    Word::new(
        "attack",
        &["who:pokemon move:thing... at target:pokemon", "who:pokemon move:thing..."],
        "face, move (the move's own animation and effects, its sound), recoil and the Hurt pose of the target",
    )
    .does(attacked),
    Word::new(
        "use",
        &["who:pokemon move:thing... on target:pokemon", "who:pokemon move:thing... at x:number y:number", "who:pokemon move:thing..."],
        "a move out of a fight: face, move (his own animation, the move's effects and sound, on him, at the point or on no one); no knock back, no hurt",
    )
    .does(used),
    Word::new("evolve", &["who:pokemon into:pokemon"], "do evolution, the game's sentence").maybe(fought_word),
    Word::new(
        "transform",
        &["who:pokemon back", "who:pokemon into form:pokemon"],
        "_change, mimic, _changed (the evolution's light, no sentence): he looks like the other, his name and his lines his own; back: as he was",
    )
    .does(transformed),
    Word::new("roar", &["who:pokemon [hard]"], "the game's roar, the view shaking (hard: harder), the sprite's own Roar animation, else its Charge and a rise")
        .does(blows::roared),
    Word::new(
        "charge",
        &["who:pokemon move:thing...", "who:pokemon"],
        "his Charge animation, the game's stats-up shine and its sound, held a beat; a move told: his next attack with it lands with a flash and a hard shake",
    )
    .does(blows::charged),
    Word::new(
        "dodge",
        &["who:pokemon [left|right|back]"],
        "a quick hop aside (back by default) with his Hop animation; right after an attack at him, or right before one, it misses: no knock back, \"Missed!\"",
    )
    .does(blows::dodged),
];

fn bar_opens(film: &mut Film, line: &Line) {
    // (`fight pikachu 12 30 [floor]`: the top line of a dungeon, or its name
    // out of one, the log at the bottom, for as long as the fight lasts.
    // `fight pikachu 12 30 against mewtwo 90`: a bar for each.)
    let args = &line.args;
    let number = |name: &str| args.number(name).map_or(1, |number| number as i32);
    let (lead, level, health) = (plain(args.word("who").unwrap_or_default()), number("lv"), number("hp"));
    film.combat.lead = Some(lead.clone());
    film.combat.bars = vec![(lead.clone(), health)];
    film.combat.levels.insert(lead.clone(), level);
    if let Some(rival) = args.word("other").map(plain) {
        film.combat.bars.push((rival.clone(), number("hp2")));
        film.combat.levels.insert(rival.clone(), level);
        by_hand(film, "open", [vec![lead], vec![rival]], Vec::new());
        return;
    }
    // (All of them in view, as long as it lasts; not those who watch.)
    let mut said = fields!["present"];
    said.extend(group_members(film));
    if let Some(floor) = args.word("floor") {
        said.extend(fields!["floor", floor]);
    } else if let Some(floor) = super::dungeon::floor_for_a_fight(film) {
        // (On a dungeon's floor: that floor, untold.)
        said.extend(fields!["floor", floor]);
    }
    by_hand(film, "open", [vec![lead], Vec::new()], said);
}

/// The two sides a line names (`a b against c d`, or `a b`: one each).
fn sides_of(line: &Line) -> Sides {
    let args = &line.args;
    match args.word("who") {
        Some(who) => [vec![plain(who)], vec![plain(args.word("other").unwrap_or_default())]],
        None => [args.words("side_team").to_vec(), args.words("others").to_vec()],
    }
}

/// The level a line says, or that.
fn level_of(line: &Line, usual: i32) -> i32 {
    line.args.number("n").map_or(usual, |level| level as i32)
}

/// The frames from a move's start to its blow landing, and after.
pub(super) const STRIKE_FRAMES: f32 = 24.0;
const STRUCK_FRAMES: i64 = 40;

/// `pikachu attack Thunderbolt at koffing`: the move by its name, as the
/// game shows it: the user's own animation and the move's effects and sound,
/// then the target knocked back, hurt. No bars, no numbers: a fight's words
/// do those (`fight`).
fn attacked(film: &mut Film, line: &Line) {
    let args = &line.args;
    let who = plain(args.word("who").unwrap_or_default());
    let named = args.words("move").join(" ");
    let Some(id) = move_id(film, &named) else {
        let tables = film.tables.clone();
        film.unknown(line.number, "unknown move", &named, tables.moves.keys().map(String::as_str), "");
        return;
    };
    let target = args.word("target").map(plain);
    let (Some(user), aimed) = (film.other(&who), target.as_deref().map(|target| film.other(target))) else {
        film.not_a(&who, "a Pokémon of the scene");
        return;
    };
    let on = match aimed {
        Some(Some(on)) => Some(on),
        Some(None) => {
            film.not_a(target.unwrap_or_default(), "a Pokémon of the scene");
            return;
        }
        None => None,
    };
    // (Charged for this move: it lands stronger; a dodge straight before: it misses.)
    let strong = film.combat.charged.get(&who) == Some(&id);
    if strong {
        film.combat.charged.remove(&who);
    }
    let dodged = film.combat.dodging.take().filter(|(one, written, _)| Some(one) == target.as_ref() && blows::straight_after(film, *written));
    if let Some(target) = &target {
        film.face(&who, target);
    }
    // (Dodged: aimed where they stood.)
    let aimed = dodged.as_ref().and_then(|(_, _, stood)| film.other(stood)).or(on);
    let moved = film.written.len();
    film.beat(emit::uses_move(user, engine::MoveId(id), aimed));
    film.combat.blow = None;
    if let (Some(target), Some(_)) = (&target, on) {
        if dodged.is_some() {
            blows::missed(film, target, STRIKE_FRAMES);
        } else {
            let start = film.written.len();
            film.on(target, |struck| emit::after(STRIKE_FRAMES, 0.0, emit::recoil(struck, user, if strong { 20.0 } else { 12.0 })));
            if let Some(hurt) = emit::anim_of("Hurt") {
                film.on(target, |struck| emit::after(STRIKE_FRAMES, 0.0, emit::anim(struck, hurt, false, false)));
            }
            if strong {
                blows::strong(film, STRIKE_FRAMES);
            }
            let struck = start..film.written.len();
            film.combat.blow = Some(blows::Blow { target: target.clone(), user, id: engine::MoveId(id), moved, struck, written: 0 });
        }
    }
    film.wait_frames((STRIKE_FRAMES as i64 + STRUCK_FRAMES) as i32, line.wait);
    if let Some(blow) = film.combat.blow.as_mut() {
        blow.written = film.written.len();
    }
}

/// `pikachu use Protect`, `a use Hypnosis on b`, `a use Surf at 300 200`: a
/// move as the game shows it, out of a fight: no one knocked back nor hurt.
fn used(film: &mut Film, line: &Line) {
    let args = &line.args;
    let who = plain(args.word("who").unwrap_or_default());
    let named = args.words("move").join(" ");
    let Some(id) = move_id(film, &named) else {
        let tables = film.tables.clone();
        film.unknown(line.number, "unknown move", &named, tables.moves.keys().map(String::as_str), "");
        return;
    };
    let Some(user) = film.one(&who) else {
        return;
    };
    let target = match (args.word("target").map(plain), args.number("x").zip(args.number("y"))) {
        (Some(target), _) => {
            let Some(on) = film.one(&target) else {
                return;
            };
            film.face(&who, &target);
            Some(on)
        }
        // (A point: an unseen mark there, faced, the move aimed at it.)
        (None, Some((x, y))) => {
            let mark = format!("p{}", super::effects::FIRST_MARK + film.timing.marks);
            film.timing.marks += 1;
            film.on(&mark, |mark| emit::put(mark, x, y, engine::Direction::Down));
            film.face(&who, &mark);
            film.other(&mark)
        }
        (None, None) => None,
    };
    film.beat(emit::uses_move(user, engine::MoveId(id), target));
    film.wait_frames((STRIKE_FRAMES as i64 + STRUCK_FRAMES) as i32, line.wait);
}

/// A move of the game by its name (`Thunderbolt`, `Thunder Punch`): its number.
fn move_id(film: &Film, named: &str) -> Option<u16> {
    film.tables.moves.get(&plain(&named.replace(' ', ""))).or_else(|| film.tables.moves.get(&plain(named))).and_then(|row| row.0.parse::<u16>().ok())
}

/// A boss looms: the view on it, the ground shaking, its song.
fn boss_looms(film: &mut Film, who: &str) {
    film.sound("boss");
    film.beat(emit::music(POKEMON.song("battle").map(engine::SongId)));
    film.on(who, |who| emit::glance(Cam::Who(who), 30.0, 90.0));
    film.wait(30);
    film.beat(emit::quake(4.0, 50.0));
    film.sound("stunned");
    film.emote(who, "anger", Some(50.0));
    film.wait(100);
}

/// Who the line says wins, if it says.
fn winner_of(line: &Line) -> Option<String> {
    line.args.word("wins").map(plain)
}

/// `ditto transform into pikachu`: the evolution's light (slowed, a
/// silhouette growing and shrinking, a flash), and he looks like the other;
/// still himself, his name, his portrait and his lines his own.
/// `ditto transform back`: the same, back to himself.
fn transformed(film: &mut Film, line: &Line) {
    let who = plain(line.args.word("who").unwrap_or_default());
    let like = match line.args.word("form") {
        Some(form) => match film.coded(&plain(form)) {
            Some(like) => Some(like),
            None => return,
        },
        None => None,
    };
    film.call(line.number, "_change", &[&who]);
    film.on(&who, |one| emit::mimic(one, like));
    film.call(line.number, "_changed", &[&who]);
}

/// A line of the fights: one reader for all their words, a fight being one
/// thing however it is said; a whole fight (`fight a vs b`, `fight directed
/// …`) puts the dungeon's top line by while it lasts.
pub(crate) fn fought_word(film: &mut Film, line: &Line) -> bool {
    let verb = line.verb.as_str();
    let whole = verb == "fight" && line.args.has("side_team");
    let opens = verb == "fight" && film.combat.lead.is_none();
    if whole || opens {
        super::dungeon::fight_begins(film);
    }
    let lost = super::dungeon::lost_in_a_fight(film, &line.block);
    let done = fought(film, line);
    if done && (whole || verb == "end_fight") {
        super::dungeon::fight_ends(film, lost);
    }
    done
}

fn fought(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    let number = line.number;
    let verb = line.verb.as_str();
    let who = plain(args.word("who").unwrap_or_default());
    match verb {
        // (`fight pikachu vs koffing [double|relay] [boss] [level 12]`: the fight, by the fights' own rules;
        // a boss looms first.)
        "fight" if args.has("vs") => {
            let sides = sides_of(line);
            if args.has("boss")
                && let Some(foe) = sides[1].first()
            {
                boss_looms(film, foe);
            }
            let sort = match () {
                _ if args.has("double") => "double",
                _ => "alone",
            };
            fight_line(film, number, sides, Whole { sort, level: level_of(line, 30), winner: winner_of(line), kept: false, more: Vec::new() });
            closed(film);
        }
        "fight" if args.has("side_team") => directed(film, line),
        "fight" if args.has("directed") => {
            let unknown = args
                .words("rest")
                .iter()
                .find(|word| film.actor(word).is_none() && !digits(word) && !["against", "level", "winner"].contains(&plain(word).as_str()))
                .cloned();
            match unknown {
                Some(word) => film.unknown_someone(number, &word),
                None => film.wrong(number, "fight directed a b against c d [level 60] [winner a]".to_string()),
            }
        }
        "join" => {
            film.emote(&who, "joy", Some(50.0));
            film.jump(&who, 10.0, 10.0);
            let text = format!("{} joined the team!", named(film, &who));
            told_or_logged(film, &text);
        }
        "fight" if args.has("hp") => {
            film.timing.opened.push(|film, line| _ = fought_word(film, &line.as_verb("end_fight")));
            bar_opens(film, line)
        }
        "end_fight" => closed(film),
        "evolve" => {
            // (Slowed, white, growing and shrinking, then what it has become.)
            let into = plain(args.word("into").unwrap_or_default());
            let code = film.code(&into);
            film.call(number, "_evolution", &[&who, &code]);
            film.staging.evolved.insert(into.clone(), who.clone());
            let text = format!("{} evolved into {}!", named(film, &who), named(film, &into));
            told_or_logged(film, &text);
            film.staging.renamed.insert(who, into);
        }
        _ => return false,
    }
    true
}
