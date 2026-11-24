//! The words of what someone says and does where they stand (Speech,
//! Presence, Movement, Turning, Body, Emotions): each written who first,
//! each played with the game's own assets, the sprites' animations, the
//! emote balloons and their sounds, the effects (`data/rom/effects`). A word
//! also read by other groups (`jump`, `face`) has its family and DS status
//! on one declaration only; the others add their forms.

use engine::{AnimId, Beat, BodyBeat, EmoteId, MotionBeat, Pose, Spot};

use film_core::emit;
use film_core::film::Film;
use film_core::spots::routes::found;
use film_core::spots::{Goal, goal_of};
use film_core::verbs::{Line, Names, Word};
use film_core::words::{WAYS, plain};

pub const WORDS: &[Word] = &[
    // Speech.
    Word::new("think", &["who:pokemon text..."], "a line thought, in the box, its portrait (face), its words in brackets").family("Speech").ds_direct("message_SetFace, message_Monologue").maybe(thought),
    Word::new(
        "narrate",
        &["text..."],
        "narrate: words of no one's in the box; narrate screen: words on the screen, centred; narrate sign: a sign read, centred in the box",
    ).family("Speech").ds_direct("box: message_ResetActor, message_Talk; screen: message_Narration; sign: message_Explanation")
    .names(Names::NoOne)
    .maybe(narrated),
    Word::new(
        "box",
        &["auto"],
        "boxside auto: where it hides no speaker, as the game does: at the foot of the screen, at the top for a line of one standing low",
    ).family("Speech").ds_close("top: the portrait on top (FACE_POS_TOP_L_FACEINW); Sky keeps the box at the bottom", "the box stays at the bottom: only the portrait goes on top")
    .maybe(read_box_auto),
    // Presence.
    Word::new("vanish", &["who:pokemon [smoke|fade|flash|vision]"], "gone in the game's smoke, a fade (by default), a flash or a vision").family("Presence").ds_close("smoke: SetEffect(99); hidden with SetOutputAttribute(128): at once, no fading out", "no fading out: hidden at once").maybe(read_vanished),
    // Movement.
    Word::new(
        "shift",
        &["who:pokemon by dx:number dy:number [in d:duration]", "who:pokemon [to] where:place [in d:duration]"],
        "slides there, still looking the same way: go, sliding",
    ).family("Movement").ds_direct("SlidePositionMark<actor>(speed, Position)")
    .maybe(read_shifted),
    Word::new(
        "jump",
        &["who:pokemon [to] where:place [height h:number]", "who:pokemon height h:number", "who:pokemon"],
        "a leap of his body, higher than a hop: on the spot, or to a place, so high",
    ).family("Movement").ds_close("MoveHeight up and down; to a place: a run there (no arc)", "no arc to a place: a run there")
    .maybe(read_jumped),
    Word::new(
        "hop",
        &["who:pokemon [happy|surprised|angry] [times n:number]"],
        "the game's hop: his Hop animation; happy, surprised, angry: its emote and its sound (angry: two)",
    ).family("Movement").ds_direct("ExecuteCommon<actor>(CORO_JUMP_HAPPY|SURPRISE|ANGRY_FUNC_SERIES, 0), WaitExecuteLives")
    .maybe(hopped),
    Word::new("follow", &["who:pokemon off"], "follow -: stops following").family("Movement").ds_close("after each move of the leader: Wait(16), MovePositionLives<follower>(speed, LEADER) (tools/ds/LIMITS.md)", "the follower catches up after each move, not step by step").maybe(read_stops_following),
    Word::new("chase", &["who:pokemon off"], "stop: stops chasing").family("Movement").ds_close("MovePositionLives<actor>(3, OTHER): runs to where the other is, once", "it runs to where the other was, once").maybe(read_stops),
    Word::new("flee", &["who:pokemon from other:pokemon [to where:place]"], "runs away from the other: to a place, or a little way").family("Movement").ds_direct("MovePositionMark away, at a run").maybe(read_fled),
    Word::new("wander", &["who:pokemon off"], "roams 0: stops wandering").family("Movement").ds_direct("ExecuteCommon<actor>(CORO_MOVE_RANDOM, 0), Sky's own stroll; off: back to its place").maybe(read_stops_wandering),
    // Turning.
    Word::new(
        "face",
        &["who:pokemon away [fast|slow]", "who:pokemon to:word [fast|slow]"],
        "turns that way, to someone, or round (away); fast: at once; slow: through each way in between, a step every few frames (turnslow)",
    ).family("Turning").ds_direct("Turn2DirectionLives<actor>(4, 10, OTHER) / Turn2Direction(4, 10, DIR_…)")
    .maybe(turned),
    Word::new("look", &["who:pokemon around"], "the game's look around: a glance one way, the other, back").family("Turning").ds_direct("ExecuteCommon(CORO_LOOK_AROUND[_LEFT|_RIGHT|_DOWN]_FUNC_SERIES), Sky's own; up: Turn2Direction").maybe(read_looked_around),
    Word::new("nod", &["who:pokemon [times n:number]"], "his Nod animation, so many times").family("Turning").ds_direct("SetAnimation (its nod, once), then 2").maybe(read_nodded),
    Word::new("shake_head", &["who:pokemon"], "the game's head shake: quick turns of the head, one way, the other, back").family("Turning").ds_direct("ExecuteCommon(CORO_HEAD_SHAKE_FUNC_SERIES), Sky's own").maybe(read_shook_head),
    // Body.
    Word::new(
        "pose",
        &["who:pokemon off", "who:pokemon name:word [once|loop|hold]"],
        "one of his sprite's animations by a clear name: once, again and again, or held at its end; off: standing",
    ).family("Body").ds_direct("SetAnimation<actor>(n): the group of that species' own sprite showing it (ds_animations.txt: Dusknoir lies with its 27), through Sky's table (once or looped); off: 2")
    .maybe(posed),
    Word::new("sleep", &["who:pokemon off"], "awake: his Wake animation, the sleep's mark gone").family("Body").ds_direct("SetAnimation(62), as Sky's INIT_SLEEP; off: its waking, once (63)").maybe(read_woke),
    Word::new("hurt", &["who:pokemon [hard] [back n:number]"], "his Pain animation, knocked back the way he looks; hard: further, the view shaking").family("Body").ds_close("its pain animation, once; no knock back", "no knock back")
        .maybe(read_hurt),
    Word::new("tremble", &["who:pokemon off", "who:pokemon"], "trembles until told off").family("Body").ds_close("its pain animation, looped: Sky has no shiver", "no shiver: its pain animation looped").maybe(read_trembled),
    Word::new("fall", &["who:pokemon [over|back|trip]"], "falls and stays down: his Tumble (over), TumbleBack (back) or Trip animation").family("Body").ds_direct("SetAnimation (tumble), once").maybe(read_fell),
    Word::new("faint", &["who:pokemon off", "who:pokemon"], "his Faint animation, held, until he gets up (off: his Wake animation)").family("Body").ds_direct("SetAnimation (faint), once; off: waking").maybe(read_fainted),
    // Emotions.
    Word::new(
        "emote",
        &["who:pokemon off", "who:pokemon kind:word [silent]"],
        "an emote balloon of the game over him, with its sound (silent: none); off: none",
    ).family("Emotions").ds_direct("SetEffect<actor>(EFFECT_…, 3): the engine's emote numbers are the game's; EFFECT_NONE once the line is said")
    .maybe(emoted),
];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("acting", WORDS);

/// The game's emote balloons by their names in the films: the effect, its sound
/// (as the game's scripts play them beside it most), if any.
pub const EMOTES: [(&str, u16, Option<&str>); 9] = [
    ("exclaim", 88, Some("startle")),
    ("question", 89, Some("question")),
    ("joy", 90, None),
    ("shock", 91, Some("shock")),
    ("sweat", 171, Some("sweat")),
    ("laugh", 93, None),
    ("anger", 94, Some("anger")),
    ("notice", 172, Some("alert")),
    ("munch", 642, None),
];

/// The game's puff of smoke, and its white light.
const SMOKE: u16 = 99;
const LIGHT: u16 = 551;
/// The sparkles that gather into a ring (Sky's EFFECT_LIGHT_ENGULFING2),
/// and their sound, as Uxie appears in Sky (D10P41A/m10a1101).
const GATHERING: u16 = 638;
const GATHERING_SOUND: &str = "6920";

/// The sprites' animations by clear names, each played once, held at its
/// end, or looped when nothing is said.
pub const POSES: [(&str, &str, Plays); 33] = [
    ("walk", "Walk", Plays::Loop),
    ("attack", "Attack", Plays::Once),
    ("sleep", "Sleep", Plays::Loop),
    ("hurt", "Hurt", Plays::Hold),
    ("stand", "Idle", Plays::Loop),
    ("swing", "Swing", Plays::Once),
    ("double", "Double", Plays::Once),
    ("hop", "Hop", Plays::Once),
    ("charge", "Charge", Plays::Loop),
    ("spin", "Rotate", Plays::Once),
    ("doze", "EventSleep", Plays::Loop),
    ("wake", "Wake", Plays::Once),
    ("eat", "Eat", Plays::Loop),
    ("tumble", "Tumble", Plays::Hold),
    ("proud", "Pose", Plays::Once),
    ("pull", "Pull", Plays::Loop),
    ("pain", "Pain", Plays::Hold),
    ("float", "Float", Plays::Loop),
    ("breathe", "DeepBreath", Plays::Once),
    ("nod", "Nod", Plays::Once),
    ("sit", "Sit", Plays::Hold),
    ("look_up", "LookUp", Plays::Hold),
    ("sink", "Sink", Plays::Hold),
    ("trip", "Trip", Plays::Hold),
    ("lie", "Laying", Plays::Hold),
    ("leap", "LeapForth", Plays::Once),
    ("headbutt", "Head", Plays::Once),
    ("cringe", "Cringe", Plays::Hold),
    ("stagger", "LostBalance", Plays::Hold),
    ("tumble_back", "TumbleBack", Plays::Hold),
    ("faint", "Faint", Plays::Hold),
    ("land", "HitGround", Plays::Once),
    ("shoot", "Shoot", Plays::Once),
];

/// How an animation plays: once then standing, held at its end, again and again.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Plays {
    Once,
    Hold,
    Loop,
}

/// How someone comes or goes: in the game's smoke, a fade, a white flash, a
/// vision, the light that gathers into a ring.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Manner {
    Smoke,
    Fade,
    Flash,
    Vision,
    Light,
}

/// The manner a line says, smoke if none.
pub fn manner(args: &film_core::grammar::Bound) -> Manner {
    match () {
        _ if args.has("fade") => Manner::Fade,
        _ if args.has("flash") => Manner::Flash,
        _ if args.has("vision") => Manner::Vision,
        _ if args.has("light") => Manner::Light,
        _ => Manner::Smoke,
    }
}

/// Someone put there already, shown (`coming`) or hidden in that manner;
/// the film waits for it.
pub fn shows(film: &mut Film, who: &str, manner: Manner, coming: bool) {
    let to = if coming { 1.0 } else { 0.0 };
    if coming {
        film.on(who, |who| emit::opacity(who, 0.0, 0.0));
    }
    match manner {
        Manner::Smoke => {
            film.on(who, |who| emit::emote(who, EmoteId(SMOKE), None));
            film.sound("shock");
            film.wait(6);
            film.on(who, |who| emit::opacity(who, to, 8.0));
            film.wait(16);
        }
        Manner::Fade => {
            film.on(who, |who| emit::opacity(who, to, 40.0));
            film.wait(40);
        }
        Manner::Flash => {
            film.beat(emit::flash([255.0, 255.0, 255.0], 24.0));
            film.sound("flash");
            film.on(who, |who| emit::emote(who, EmoteId(LIGHT), None));
            film.wait(4);
            film.on(who, |who| emit::opacity(who, to, 6.0));
            film.wait(26);
        }
        Manner::Vision => {
            film.sound("vision");
            film.beat(emit::flash([255.0, 255.0, 255.0], 40.0));
            film.wait(8);
            film.on(who, |who| emit::opacity(who, to, 50.0));
            film.wait(50);
        }
        // (As Sky brings Uxie: a double blink 10 frames apart, half a
        // second, the sparkles gathering into a ring with their sound, and
        // in 1.8 s the body fades in at its centre.)
        Manner::Light => {
            for blink in 0..2 {
                film.sound("flash");
                film.beat(emit::flash([255.0, 255.0, 255.0], 12.0));
                film.wait(if blink == 0 { 10 } else { 30 });
            }
            film.sound(GATHERING_SOUND);
            film.on(who, |who| emit::emote(who, EmoteId(GATHERING), None));
            film.wait(108);
            film.on(who, |who| emit::opacity(who, to, 40.0));
            film.wait(40);
        }
    }
}

/// `pikachu think (happy): Hmm`: a line in the box, his portrait, the words
/// in brackets as the game writes a thought.
fn thought(film: &mut Film, line: &Line) -> bool {
    if !line.args.has("who") {
        return false;
    }
    let who = film_core::verbs::who_said(line);
    // (`think a - Hi`, `think a 3 Hi`: the engine's bubble.)
    let first = line.args.words("text").first().cloned().unwrap_or_default();
    if first == "-" || first.parse::<u16>().is_ok() {
        return false;
    }
    let said = line.args.words("text").join(" ");
    let said = said.trim_start_matches(':').trim();
    let (face, text) = match said.strip_prefix('(').and_then(|rest| rest.split_once(')')) {
        Some((face, rest)) => (Some(face.trim().to_string()), rest.trim_start().trim_start_matches(':').trim()),
        None => (None, said),
    };
    if text.is_empty() || face.as_deref().is_some_and(|face| face.parse::<u16>().is_ok()) {
        return false;
    }
    film.said(&who, face.as_deref(), &format!("({text})"), line.number);
    true
}

/// `narrate: words`, `narrate screen: words`, `narrate sign: words`.
fn narrated(film: &mut Film, line: &Line) -> bool {
    if !line.args.has("text") {
        return false;
    }
    let words = line.args.words("text");
    let first = words.first().map(|word| word.trim_end_matches(':').to_lowercase()).unwrap_or_default();
    let (how, rest) = match first.as_str() {
        "box" | "screen" | "sign" => (first.clone(), &words[1..]),
        _ => ("box".to_string(), words),
    };
    let said = rest.join(" ");
    let said = said.trim_start_matches(':').trim();
    if said.is_empty() {
        film.wrong(line.number, "narrate what? narrate: Long ago... (or narrate screen:, narrate sign:)".to_string());
        return true;
    }
    match how.as_str() {
        "screen" => {
            let frames = 90 + said.chars().count() * 3;
            film.caption(frames as f32, &film_core::screen::on_screen(said), false);
            film.wait(frames as i64);
        }
        "sign" => film.narrate(&film_core::screen::on_screen(said)),
        _ => film_core::groups::said::told_or_logged(film, said),
    }
    true
}

/// `box auto`: where it hides no speaker, as the game does: at the foot
/// of the screen, at the top for a line of one standing low on it.
fn box_auto(film: &mut Film, _: &Line) {
    film.box_kept = false;
    film.beat(emit::box_auto());
}

/// `pikachu vanish [smoke|fade|flash|vision]`: gone, a fade by default.
fn vanished(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let how = if line.args.has("smoke") || line.args.has("flash") || line.args.has("vision") { manner(&line.args) } else { Manner::Fade };
    shows(film, &who, how, false);
    film.on(&who, emit::gone);
}

/// `pikachu shift by 12 0`, `pikachu shift to rock`: slides there, still
/// looking the same way (in so long, a third of a second otherwise).
fn shifted(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let frames = line.args.frames("d").unwrap_or(20) as f32;
    let spot = if line.args.has("by") {
        let (dx, dy) = (line.args.number("dx").unwrap_or_default(), line.args.number("dy").unwrap_or_default());
        film.spot_by(&who, dx, dy)
    } else {
        spot_of(film, line)
    };
    let Some(spot) = spot else { return };
    let to = found(film, &who, spot, &["exact"]);
    film.on_at(&who, to, |one, at| emit::go_point(one, at, frames, true));
    film.wait_for(&who, line.wait);
}

/// Where a line's `where` says, someone's side for someone.
fn spot_of(film: &mut Film, line: &Line) -> Option<Spot> {
    let place = line.args.words("where").to_vec();
    match goal_of(film, &place, line.number) {
        Goal::Spot(spot) => Some(spot),
        Goal::Nothing => film.spot_beside(place.last().map(String::as_str).unwrap_or_default()),
        Goal::Told => None,
    }
}

/// The frames of a hop, the Hop animation played once.
const HOP: i64 = 24;

/// His Hop animation, so many times (the animation lifts him itself).
fn hop_anim(film: &mut Film, who: &str, times: u16) {
    match emit::anim_of("Hop") {
        Some(group) => film.on(who, |one| emit::anim_times(one, group, times)),
        None => film.jump(who, 10.0, 10.0),
    }
}

/// `pikachu jump`, `pikachu jump to rock height 20`: a leap of his body,
/// higher than a hop (no animation of his, the game's own jump).
fn jumped(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let height = line.args.number("h").unwrap_or(16.0);
    if line.args.has("where") {
        let Some(spot) = spot_of(film, line) else { return };
        let to = found(film, &who, spot, &["exact"]);
        film.on_at(&who, to, |one, at| emit::hop_point(one, at, height, 24.0));
        film.wait_for(&who, line.wait);
        return;
    }
    film.jump(&who, height, 16.0);
    film.wait_frames(18, line.wait);
}

/// `pikachu hop [happy|surprised|angry] [times 2]`: the game's routine,
/// his Hop animation and a little leap (two low ones, angry); its emote and
/// sound if a mood is said.
fn hopped(film: &mut Film, line: &Line) -> bool {
    if !line.args.has("who") {
        return false;
    }
    let who = film_core::verbs::who_said(line);
    let Some(times) = line.args.number("n").map_or(Some(1), |times| (times >= 1.0).then_some(times as u32)) else {
        film.not_a(line.args.word("n").unwrap_or_default(), "a number of hops");
        return true;
    };
    let mood = ["happy", "surprised", "angry"].into_iter().find(|mood| line.args.has(mood));
    let emote = match mood {
        Some("happy") => Some("joy"),
        Some("surprised") => Some("exclaim"),
        Some("angry") => Some("anger"),
        _ => None,
    };
    if let Some(emote) = emote {
        balloon(film, &who, emote, true);
    }

    // (Angry: two quick stamps, as the game's.)
    let times = if mood == Some("angry") { times * 2 } else { times };
    hop_anim(film, &who, times as u16);
    film.wait_frames(HOP * i64::from(times), line.wait);
    true
}

/// `pikachu follow off`.
fn stops_following(film: &mut Film, line: &Line) {
    film.on(&film_core::verbs::who_said(line), |who| emit::follow(who, None, None));
}

/// `pikachu chase off`: no longer chasing; he stops where he is.
fn stops(film: &mut Film, line: &Line) {
    film.on(&film_core::verbs::who_said(line), emit::stop);
}

/// `pikachu wander off`: no longer wandering.
fn stops_wandering(film: &mut Film, line: &Line) {
    film.on(&film_core::verbs::who_said(line), |who| emit::roams(who, 0.0));
}

/// `pikachu flee from gengar [to left]`: he runs off, to a place or a
/// little way from the other.
fn fled(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let other = plain(line.args.word("other").unwrap_or_default());
    if line.args.has("where") {
        let Some(spot) = spot_of(film, line) else { return };
        film_core::spots::routes::go(film, &who, spot, "run", line.wait, film_core::spots::routes::USUAL, true);
        return;
    }
    if let Some(from) = film.other(&other) {
        film.on(&who, |one| Beat::Motion(MotionBeat::Flee { who: one, from, distance: 48.0 }));
    }
    film.wait_for(&who, line.wait);
}

/// `pikachu face right`, `pikachu face piplup slow`, `pikachu face away`.
fn turned(film: &mut Film, line: &Line) -> bool {
    if !line.args.has("who") {
        return false;
    }
    let who = film_core::verbs::who_said(line);
    let to = plain(line.args.word("to").unwrap_or("away"));
    if !line.args.has("away") && !WAYS.contains(&to.as_str()) && film.actor(&to).is_none() {
        return false;
    }
    // (Slow: through each way in between, a step every few frames, as the
    // game's slow turns; then a moment, the turn done.)
    if line.args.has("slow") {
        film.on(&who, |one| Beat::Body(BodyBeat::TurnSlow { who: one, every: engine::Frames(SLOW_STEP) }));
    }
    if line.args.has("away") {
        film.on(&who, |one| Beat::Body(BodyBeat::TurnAround(one)));
    } else {
        film_core::spots::faced(film, &who, &to);
    }
    if line.args.has("slow") {
        film.wait_for(&who, line.wait);
        film.wait_frames(8, line.wait);
        return true;
    }
    if !line.args.has("fast") {
        film.wait_frames(6, line.wait);
    }
    true
}

/// The frames of each step of a slow turn.
const SLOW_STEP: f32 = 6.0;

/// Turns so many eighths at once (counter-clockwise; clockwise below 0).
fn turn_by(film: &mut Film, who: &str, eighths: f32) {
    film.on(who, |one| Beat::Body(BodyBeat::TurnBy { who: one, eighths }));
}

/// `pikachu look around`: as the game's routine, a glance one way, a
/// longer one the other, and back.
fn looked_around(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    for (eighths, wait) in [(2.0, 15), (-4.0, 18), (2.0, 10)] {
        turn_by(film, &who, eighths);
        film.wait(wait);
    }
    let _ = line.wait;
}

/// `pikachu shake_head`: the game's head shake, quick turns of an eighth
/// and back (no sprite has an animation of it).
fn shook_head(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    for eighths in [1.0, -2.0, 2.0, -1.0] {
        turn_by(film, &who, eighths);
        film.wait(5);
    }
    film.wait_frames(10, line.wait);
}

/// The frames of one nod.
const NOD: i64 = 24;

/// `pikachu nod [times 2]`: his Nod animation.
fn nodded(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let times = line.args.number("n").unwrap_or(1.0).clamp(1.0, 9.0) as u16;
    match emit::anim_of("Nod") {
        Some(group) => film.on(&who, |one| emit::anim_times(one, group, times)),
        None => film.not_a("Nod", "an animation of the game"),
    }
    film.wait_frames(NOD * i64::from(times), line.wait);
}

/// The animation of a clear name, and how it holds by default.
pub fn pose_named(name: &str) -> Option<(AnimId, Plays)> {
    let (_, anim, hold) = POSES.iter().find(|(said, ..)| *said == name)?;
    Some((emit::anim_of(anim)?, *hold))
}

/// `pikachu pose sit [once|loop|hold]`, `pikachu pose off`.
fn posed(film: &mut Film, line: &Line) -> bool {
    if !line.args.has("who") {
        return false;
    }
    let who = film_core::verbs::who_said(line);
    if line.args.has("off") {
        film.on(&who, |one| emit::pose(one, Pose::Idle));
        return true;
    }
    let name = plain(line.args.word("name").unwrap_or_default());
    if matches!(name.as_str(), "spin" | "dance") {
        film_core::groups::aura::moved(film, &who, film_core::groups::aura::Mood::Dancing);
    }
    // (`pose a sleep`, said with no way of playing it: the engine's own pose, lying asleep.)
    let said_how = line.args.has("once") || line.args.has("loop") || line.args.has("hold");
    let own = match name.as_str() {
        "sleep" => Some(Pose::Sleep),
        "hurt" => Some(Pose::Hurt),
        "attack" => Some(Pose::Attack),
        "walk" => Some(Pose::Walk),
        _ => None,
    };
    if let Some(pose) = own.filter(|_| !said_how) {
        film.on(&who, |one| emit::pose(one, pose));
        return true;
    }
    let Some((group, by_default)) = pose_named(&name) else {
        return false;
    };
    let hold = match () {
        _ if line.args.has("once") => Plays::Once,
        _ if line.args.has("loop") => Plays::Loop,
        _ if line.args.has("hold") => Plays::Hold,
        _ => by_default,
    };
    for one in film.members(&who) {
        // (An animation his sprite lacks: the nearest it has, else he stands, told.)
        let Some(group) = animation_had(film, &one, group, line.number) else {
            continue;
        };
        film.on(&one, |one| match hold {
            Plays::Once => emit::anim_times(one, group, 1),
            Plays::Hold => emit::anim(one, group, false, false),
            Plays::Loop => emit::anim(one, group, true, false),
        });
    }
    true
}

/// For an animation a sprite may lack, the nearest ones to play instead,
/// in turn (most sprites have Hurt, Hop, Charge and Attack).
const NEAREST: [(&str, &[&str]); 18] = [
    ("Sit", &["Laying", "Sleep"]),
    ("Laying", &["Sleep"]),
    ("LookUp", &["Charge"]),
    ("Nod", &["Hop"]),
    ("DeepBreath", &["Nod", "Charge"]),
    ("Pose", &["Hop", "Charge"]),
    ("Cringe", &["Pain", "Hurt"]),
    ("LostBalance", &["Pain", "Hurt"]),
    ("Pain", &["Hurt"]),
    ("Faint", &["Sleep", "Hurt"]),
    ("Tumble", &["TumbleBack", "Hurt"]),
    ("TumbleBack", &["Tumble", "Hurt"]),
    ("Trip", &["Tumble", "Hurt"]),
    ("HitGround", &["Hop"]),
    ("LeapForth", &["Hop"]),
    ("Head", &["Attack"]),
    ("Pull", &["Charge"]),
    ("Wake", &["Hop"]),
];

/// The animation someone plays for that one: itself if his sprite has it
/// (or his sheets are not known), else the nearest he has; none, told: he
/// stands.
pub fn animation_had(film: &mut Film, who: &str, group: AnimId, number: usize) -> Option<AnimId> {
    let Some(code) = film.actor(who) else {
        return Some(group);
    };
    let has = |film: &Film, group: AnimId| film.has_animation(&code, group.0).unwrap_or(true);
    if has(film, group) {
        return Some(group);
    }
    let name = scenes::vocabulary::POKEMON.animations.iter().find(|(_, at)| *at == usize::from(group.0)).map_or("that", |(name, _)| *name);
    let nearest = NEAREST.iter().find(|(said, _)| *said == name).map_or(&[][..], |(_, nearest)| *nearest);
    if let Some(instead) = nearest.iter().filter_map(|name| emit::anim_of(name)).find(|instead| has(film, *instead)) {
        return Some(instead);
    }
    film.remark(number, format!("{who} has no {name} animation: he stays standing"));
    None
}

/// `pikachu sleep off`: awake, his Wake animation.
fn woke(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    film.on(&who, |one| emit::status(one, None));
    match emit::anim_of("Wake") {
        Some(group) => film.on(&who, |one| emit::anim_times(one, group, 1)),
        None => film.on(&who, |one| emit::pose(one, Pose::Idle)),
    }
    film.wait_frames(40, line.wait);
}

/// `pikachu hurt [hard] [back 12]`: his Pain animation, knocked back the
/// way he looks; hard: further, the view shaking.
fn hurt(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let hard = line.args.has("hard");
    let back = line.args.number("n").unwrap_or(if hard { 20.0 } else { 8.0 });
    film.on(&who, |one| emit::recoil(one, one, back));
    if hard {
        film.beat(emit::quake(3.0, 16.0));
        film.sound("jolt");
    }
    film.wait(2);
    match emit::anim_of("Pain") {
        Some(group) => film.on(&who, |one| emit::anim_times(one, group, 1)),
        None => film.not_a("Pain", "an animation of the game"),
    }
    film.wait_frames(if hard { 40 } else { 30 }, line.wait);
}

/// So long as a line's tremble lasts, until told off.
const LASTING: f32 = 1_000_000.0;

/// `pikachu tremble`, `pikachu tremble off`.
fn trembled(film: &mut Film, line: &Line) {
    let frames = if line.args.has("off") { 0.0 } else { LASTING };
    film.on(&film_core::verbs::who_said(line), |one| emit::tremble(one, frames, 1.0));
}

/// `pikachu fall [over|back|trip]`: down and staying down.
fn fell(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let anim = match () {
        _ if line.args.has("back") => "TumbleBack",
        _ if line.args.has("trip") => "Trip",
        _ => "Tumble",
    };
    match emit::anim_of(anim) {
        Some(group) => film.on(&who, |one| emit::anim(one, group, false, false)),
        None => film.not_a(anim, "an animation of the game"),
    }
    film.wait(14);
    film.sound("fall");
    film.wait_frames(30, line.wait);
}

/// `pikachu faint`: his Faint animation, held; `faint off`: up again.
fn fainted(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let (anim, group_held) = if line.args.has("off") { ("Wake", false) } else { ("Faint", true) };
    match emit::anim_of(anim) {
        Some(group) if group_held => film.on(&who, |one| emit::anim(one, group, false, false)),
        Some(group) => film.on(&who, |one| emit::anim_times(one, group, 1)),
        None => film.not_a(anim, "an animation of the game"),
    }
    film.wait_frames(40, line.wait);
}

/// The frames a line of an emote lasts.
const BALLOON: i64 = 40;

/// An emote balloon of the game over someone, its sound unless `silent`.
fn balloon(film: &mut Film, who: &str, kind: &str, sound: bool) -> bool {
    let Some((_, id, heard)) = EMOTES.iter().find(|(name, ..)| *name == kind) else {
        return false;
    };
    film.on(who, |one| emit::emote(one, EmoteId(*id), Some(60.0)));
    match kind {
        "anger" => film_core::groups::aura::moved(film, who, film_core::groups::aura::Mood::Angry),
        "sweat" | "shock" => film_core::groups::aura::moved(film, who, film_core::groups::aura::Mood::Scared),
        _ => {}
    }
    if let Some(heard) = heard.filter(|_| sound) {
        film.sound(heard);
    }
    true
}

/// `pikachu emote exclaim [silent]`, `pikachu emote off`; a name of before
/// (`surprise`, `tears`) is the older word's.
fn emoted(film: &mut Film, line: &Line) -> bool {
    if !line.args.has("who") {
        return false;
    }
    let who = film_core::verbs::who_said(line);
    if line.args.has("off") {
        film.on(&who, |one| emit::emote(one, EmoteId(0), None));
        return true;
    }
    let kind = plain(line.args.word("kind").unwrap_or_default());
    if !balloon(film, &who, &kind, !line.args.has("silent")) {
        return false;
    }
    // (A moment to see it, as the game waits after one.)
    film.wait_frames(BALLOON, line.wait);
    true
}

/// `pikachu walk to rock backward`: his back to where he goes, stepping
/// (his Walk animation) as he slides there, in so long (a second otherwise).
pub fn backward(film: &mut Film, line: &Line, who: &str, spot: Spot) {
    let frames = line.args.frames("time").unwrap_or(60) as f32;
    let to = found(film, who, spot, &["apart"]);
    film.on_at(who, to.clone(), emit::look_at_point);
    film.on(who, |one| Beat::Body(BodyBeat::TurnAround(one)));
    if let Some(group) = emit::anim_of("Walk") {
        film.on(who, |one| emit::anim(one, group, true, false));
    }
    film.on_at(who, to, |one, at| emit::go_point(one, at, frames, true));
    film.on(who, emit::wait_for);
    film.on(who, |one| emit::pose(one, Pose::Idle));
    let _ = line.wait;
}

/// Each word played only for a line its own form read (who said, or `box
/// auto`): another family's line, read by none of its forms, is not its own.
macro_rules! read_by_its_form {
    ($($read:ident => $play:ident),* $(,)?) => {
        $(fn $read(film: &mut Film, line: &Line) -> bool {
            if !line.args.has("who") && !line.args.has("auto") {
                return false;
            }
            $play(film, line);
            true
        })*
    };
}

read_by_its_form! {
    read_box_auto => box_auto,
    read_vanished => vanished,
    read_shifted => shifted,
    read_jumped => jumped,
    read_stops_following => stops_following,
    read_stops => stops,
    read_fled => fled,
    read_stops_wandering => stops_wandering,
    read_looked_around => looked_around,
    read_nodded => nodded,
    read_shook_head => shook_head,
    read_woke => woke,
    read_hurt => hurt,
    read_trembled => trembled,
    read_fell => fell,
    read_fainted => fainted,
}
