//! Fights: the film writes lines, and `scenes/src/fight` rules and stages them.
//!
//! - `lines`: what a fight's line is made of;
//! - `directed`: a fight stated in advance, its block read;
//! - here: the words of a fight and the line each writes.

use engine::Cam;
use scenes::vocabulary::POKEMON;

use film_core::film::Film;
use film_core::verbs::{Line, Word};
use film_core::words::{digits, plain};
use film_core::{emit, fields};
use film_speech::said::told_or_logged;

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
    ).family("Fights").ds_close("staged as Sky's scenes stage fights: each blow the rules played (a directed fight's as written) as attack, the one struck stepping back (SlidePositionMark) in pain, a knock out its faint; the lines said in it kept", "no bars and no log: its blows as attacks, its lines said").word_first()
    .maybe(fought_word),
    Word::new("join", &["who:pokemon [team]"], "emote, jump, the game's sentence").family("Fights").ds_direct("its emote and hop, then as Sky: me_Play(5), message_Mail(\"[CN]… joined the team!\"), WaitMe(5)").maybe(fought_word),
    Word::new(
        "attack",
        &["who:pokemon move:thing... at target:pokemon", "who:pokemon move:thing..."],
        "face, move (the move's own animation and effects, its sound), recoil and the Hurt pose of the target on the frame his sprite marks for the blow (else after 24 frames)",
    ).family("Fights").ds_close("its pain once (a crouch), its own attack (group 1), the move's own effect on the target as a dungeon shows it (SetEffect plays the game's effects by the same ids; Thunderbolt's bolt, Ember's flame; its type's without the game's tables), what the move throws flying to it first, a small shake", "a crouch and the attack, what the move throws flying to the target and its own effect there: no effect before it, no move's own motion, the shot straight and drawn one way")
    .does(attacked),
    Word::new(
        "use",
        &[
            "who:pokemon move:thing... on target:pokemon",
            "who:pokemon move:thing... at x:number y:number",
            "who:pokemon move:thing...",
            "who:pokemon on target:pokemon",
            "who:pokemon at x:number y:number",
            "who:pokemon",
        ],
        "a move out of a fight: face, move (his own animation, the move's effects and sound, on him, at the point or on no one; they leave on the frame his sprite marks for the blow); no knock back, no hurt; no move, or `its move`: one his kind learns by his level, of his type if it can",
    ).family("Fights").ds_close("as attack (a move chosen by the kind's learnset the same), the one it is used on neither knocked back nor hurt: its pain once (a crouch), its own attack (group 1), the move's own effect on the target (at a place or on no one: its own as it strikes, on its user; its type's without the game's tables), what the move throws flying to it first, a small shake", "a crouch and the attack, what the move throws flying to the target and its own effect there: no move's own motion, the shot straight and drawn one way")
    .does(used),
    Word::new(
        "evolve",
        &["who:pokemon into:pokemon", "who:pokemon"],
        "do evolution, the game's sentence; no form: the next of his family (the game's evolutions); one not of his family, remarked",
    ).family("Fights").ds_close("a stats-up shine, the screen white, the evolved one (an actor of its own) put where it stood and shown, the other hidden; no change of species (Sky's scripts have none)", "no change of species: the evolved one is another actor")
    .maybe(fought_word),
    Word::new(
        "transform",
        &["who:pokemon back", "who:pokemon into its next form", "who:pokemon into form:pokemon"],
        "_change, mimic, _changed (the evolution's light, no sentence): he looks like the other, his name and his lines his own; its next form: the next of his family; back: as he was",
    ).family("Fights").ds_close("a stats-up shine, the screen white and back; with the mega patch's dynamic_actor (your own base only: no licence to share it) he is a special actor, taken away under the white and loaded again as the other species (ProcessSpecial(67, species, 2n), supervision_Acting), his lines his own name and face (set-portrait-monster); else he keeps his look", "under a white screen, not the evolution's light; four at most in a film; in a base for sharing he keeps his look")
    .does(transformed),
    Word::new("roar", &["who:pokemon [hard]"], "the game's roar, the view shaking (hard: harder), the sprite's own Roar animation, else its Charge and a rise").family("Fights").ds_direct("as Sky's bosses: se_Play(6930), camera_SetEffect(2, 1, 1.0) (hard: 2, 2, 3.0) a while, then 0, 0, 0; SetAnimation of the sprite's own Roar group once, else its Charge looped (9), as Groudon's")
        .does(blows::roared),
    Word::new(
        "charge",
        &["who:pokemon move:thing...", "who:pokemon"],
        "his Charge animation, the game's stats-up shine and its sound, held a beat; a move told: his next attack with it lands with a flash and a hard shake",
    ).family("Fights").ds_direct("SetAnimation (its Charge, looped: 9), SetEffect(691, 3) (Sky's stats-up shine), se_Play(4881), a beat, standing; the charged attack's flash and shake as written (screen_WhiteOut, camera_SetEffect)")
    .does(blows::charged),
    Word::new(
        "dodge",
        &["who:pokemon [left|right|back]"],
        "a quick hop aside (back by default) with his Hop animation; right after an attack at him, or right before one, it misses: no knock back, \"Missed!\"",
    ).family("Fights").ds_close("Sky's hop (CORO_JUMP_HAPPY_FUNC_SERIES) and a quick MovePositionMark aside, the miss's sound (se_Play); no \"Missed!\" (Sky's scenes have no floating words)", "the “Missed!” won't show")
    .does(blows::dodged),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("fights", WORDS);

fn bar_opens(film: &mut Film, line: &Line) {
    // (`fight pikachu 12 30 [floor]`: the top line of a dungeon, or its name
    // out of one, the log at the bottom, for as long as the fight lasts.
    // `fight pikachu 12 30 against mewtwo 90`: a bar for each.)
    let args = &line.args;
    let number = |name: &str| args.number(name).map_or(1, |number| number as i32);
    let (lead, level, health) = (plain(args.word("who").unwrap_or_default()), number("lv"), number("hp"));
    film.kept::<Combat>().lead = Some(lead.clone());
    film.kept::<Combat>().bars = vec![(lead.clone(), health)];
    film.kept::<Combat>().levels.insert(lead.clone(), level);
    if let Some(rival) = args.word("other").map(plain) {
        film.kept::<Combat>().bars.push((rival.clone(), number("hp2")));
        film.kept::<Combat>().levels.insert(rival.clone(), level);
        by_hand(film, "open", [vec![lead], vec![rival]], Vec::new());
        return;
    }
    // (All of them in view, as long as it lasts; not those who watch.)
    let mut said = fields!["present"];
    said.extend(group_members(film));
    if let Some(floor) = args.word("floor") {
        said.extend(fields!["floor", floor]);
    } else if let Some(floor) = film_dungeon::floor_for_a_fight(film) {
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

/// The frames from a move's start to its blow landing (when its user's
/// sprite marks no step for it), and after.
pub(crate) const STRIKE_FRAMES: f32 = 24.0;
const STRUCK_FRAMES: i64 = 40;

/// The frames from a move's start to its blow landing, for that user: to
/// the step his sprite marks for the blow in the motion the move plays (its
/// own group, else his attack), as the game has it; else `STRIKE_FRAMES`.
pub(crate) fn strike_frames(film: &mut Film, who: &str, id: u16) -> f32 {
    /// The animations every monster has before its own groups; its attack.
    const OWN: u8 = 5;
    const ATTACK: u8 = 4;
    type Known = std::sync::Mutex<std::collections::HashMap<(String, u16, u8), Option<f32>>>;
    static KNOWN: std::sync::OnceLock<Known> = std::sync::OnceLock::new();
    let code = film.actor(who).unwrap_or_default();
    let Some(number) = film_core::names::species(&code).and_then(|number| number.parse::<u16>().ok()) else {
        return STRIKE_FRAMES;
    };
    let tables = film.tables.clone();
    let group = tables.move_motions.get(id as usize).map_or(1, |motion| motion.of(Some(number)).0);
    let own = tables.sprite_groups.get(&number.to_string()).is_some_and(|groups| groups.contains(&u16::from(group)));
    let animation = if own { OWN + group } else { ATTACK };
    let hit = || {
        let steps = assets::tables::steps(&tables.data, number);
        let at = steps.get(&(animation, 0))?.iter().position(|(mark, _)| *mark == assets::tables::HIT_MARK)?;
        Some(tables.sheet_ticks.get(&(number, animation))?.iter().take(at).map(|&ticks| f32::from(ticks)).sum())
    };
    let mut known = KNOWN.get_or_init(Default::default).lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    known.entry((tables.data.clone(), number, animation)).or_insert_with(hit).unwrap_or(STRIKE_FRAMES)
}

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
    let strong = film.kept::<Combat>().charged.get(&who) == Some(&id);
    if strong {
        film.kept::<Combat>().charged.remove(&who);
    }
    let dodged = film.kept::<Combat>().dodging.take().filter(|(one, written, _)| Some(one) == target.as_ref() && blows::straight_after(film, *written));
    if let Some(target) = &target {
        film.face(&who, target);
    }
    // (Dodged: aimed where they stood.)
    let aimed = dodged.as_ref().and_then(|(_, _, stood)| film.other(stood)).or(on);
    let strike = strike_frames(film, &who, id);
    let moved = film.written.len();
    film.beat(emit::uses_move(user, engine::MoveId(id), aimed));
    film.kept::<Combat>().blow = None;
    if let (Some(target), Some(_)) = (&target, on) {
        if dodged.is_some() {
            blows::missed(film, target, strike);
        } else {
            let start = film.written.len();
            film.on(target, |struck| emit::after(strike, 0.0, emit::recoil(struck, user, if strong { 20.0 } else { 12.0 })));
            if let Some(hurt) = emit::anim_of("Hurt") {
                film.on(target, |struck| emit::after(strike, 0.0, emit::anim(struck, hurt, false, false)));
            }
            if strong {
                blows::strong(film, strike);
            }
            let struck = start..film.written.len();
            film.kept::<Combat>().blow = Some(blows::Blow { target: target.clone(), user, id: engine::MoveId(id), moved, struck, written: 0, strike });
        }
    }
    film.wait_frames((strike as i64 + STRUCK_FRAMES) as i32, line.wait);
    let written = film.written.len();
    if let Some(blow) = film.kept::<Combat>().blow.as_mut() {
        blow.written = written;
    }
}

/// `pikachu use Protect`, `a use Hypnosis on b`, `a use Surf at 300 200`: a
/// move as the game shows it, out of a fight: no one knocked back nor hurt.
fn used(film: &mut Film, line: &Line) {
    let args = &line.args;
    let who = plain(args.word("who").unwrap_or_default());
    let named = args.words("move").join(" ");
    let own = named.is_empty() || named.to_lowercase() == "its move";
    let Some(id) = (if own { own_move(film, &who, line.number) } else { move_id(film, &named) }) else {
        if !own {
            let tables = film.tables.clone();
            film.unknown(line.number, "unknown move", &named, tables.moves.keys().map(String::as_str), "");
        }
        return;
    };
    never_learnt(film, &who, id, line.number);
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
            let mark = format!("p{}", film_view::effects::FIRST_MARK + film.timing.marks);
            film.timing.marks += 1;
            film.on(&mark, |mark| emit::put(mark, x, y, engine::Direction::Down));
            film.face(&who, &mark);
            film.other(&mark)
        }
        (None, None) => None,
    };
    let strike = strike_frames(film, &who, id);
    film.beat(emit::uses_move(user, engine::MoveId(id), target));
    film.wait_frames((strike as i64 + STRUCK_FRAMES) as i32, line.wait);
}

/// The level someone is at in a film: his fight's, else a fight's usual.
const OWN_LEVEL: i32 = 30;

/// `a use`, `a use its move`: a move his kind learns by his level (its
/// `learnset`), of his own type if it can; none known: told.
fn own_move(film: &mut Film, who: &str, number: usize) -> Option<u16> {
    let level = film.kept::<Combat>().levels.get(who).copied().unwrap_or(OWN_LEVEL);
    let code = lines::code_now(film, who);
    let chosen = film.own_move(&code, level);
    if chosen.is_none() {
        film.wrong(number, format!("{who} use: the game's tables give no move of his kind (name one: {who} use Tackle)"));
    }
    chosen
}

/// A move someone's kind never learns (by level nor by a machine): told as
/// a remark, the scene played all the same.
pub(crate) fn never_learnt(film: &mut Film, who: &str, id: u16, number: usize) {
    let code = lines::code_now(film, who);
    if film.learns(&code, id) == Some(false) {
        let named = film.tables.moves.values().find(|row| row.0 == id.to_string()).map_or(id.to_string(), |row| row.1.clone());
        film.remark(number, format!("{who} never learns {named} (by level nor by a machine): played all the same"));
    }
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
    let form = match line.args.has("next") {
        true => match next_form_of(film, &who, line.number) {
            Some(next) => Some(next),
            None => return,
        },
        false => line.args.word("form").map(plain),
    };
    let like = match form {
        Some(form) => match film.coded(&form) {
            Some(like) => Some(like),
            None => return,
        },
        None => None,
    };
    film.call(line.number, "_change", &[&who]);
    film.on(&who, |one| emit::mimic(one, like));
    film.call(line.number, "_changed", &[&who]);
}

/// The word of the next form of someone's kind (`evolve`, `transform into
/// its next form`); none, told.
fn next_form_of(film: &mut Film, who: &str, number: usize) -> Option<String> {
    let code = lines::code_now(film, who);
    let next = film.next_form(&code);
    if next.is_none() {
        film.wrong(
            number,
            format!("{who} has no next form: the last of his family, or one the game's tables do not know (name it: {who} evolve into <species>)"),
        );
    }
    next
}

/// A line of the fights: one reader for all their words, a fight being one
/// thing however it is said; a whole fight (`fight a vs b`, `fight directed
/// …`) puts the dungeon's top line by while it lasts.
pub(crate) fn fought_word(film: &mut Film, line: &Line) -> bool {
    let verb = line.verb.as_str();
    let whole = verb == "fight" && line.args.has("side_team");
    let opens = verb == "fight" && film.kept::<Combat>().lead.is_none();
    if whole || opens {
        film_dungeon::fight_begins(film);
    }
    let lost = film_dungeon::lost_in_a_fight(film, &line.block);
    let done = fought(film, line);
    if done && (whole || verb == "end_fight") {
        film_dungeon::fight_ends(film, lost);
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
            let into = match args.word("into") {
                Some(into) => plain(into),
                None => match next_form_of(film, &who, number) {
                    Some(next) => next,
                    None => return true,
                },
            };
            let code = film.code(&into);
            let mine = lines::code_now(film, &who);
            let family = |film: &Film, code: &str| film.kind_of(code).map(|kind| kind.family).filter(|family| *family > 0);
            if let (Some(from), Some(to)) = (family(film, &mine), family(film, &code))
                && from != to
            {
                film.remark(number, format!("{who} evolves into {into}: not of his family (played all the same)"));
            }
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
