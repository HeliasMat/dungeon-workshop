//! What each word of the language becomes on the DS (`scene_player
//! export-ds`): Explorers of Sky's own operations, with less than the film
//! shows, not written yet, or none. One table, read by the export, the
//! language's DS mode (`for ds`) and the Workshop's Wiki and completion.

/// How a word of the language exports to the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Export {
    /// Written as the game's own operations, as the film plays it.
    Direct,
    /// Written, with less than the film shows (said in the table).
    Close,
    /// The game can do it; the exporter does not write it yet.
    Work,
    /// The game has no operation for it: it needs a patch of the game's code.
    None,
}

/// Each word of the core families (`plan::WORDS`), and the game's
/// operations it becomes.
pub const WORDS: &[(&str, Export, &str)] = &[
    // Structure
    ("title", Export::Direct, "back_SetTitleBanner(56, title): alone on black before the scene, as Sky titles its story"),
    (
        "scene",
        Export::Direct,
        "back_SetGround(LEVEL_…), supervision_Acting(0), camera_SetMyself<performer 0>; the top screen's picture as Sky pairs it with the place (back2_SetMode(4), back2_SetGround), else black as most of Sky's scenes; who stands there goes in the .ssa",
    ),
    ("moment", Export::Direct, "a @label"),
    (
        "transition",
        Export::Close,
        "fade: screen_FadeIn/Out; white: screen_WhiteOut, then screen_FadeIn back (as Sky); boss: CallCommon(CORO_BOSS_WIPE_FUNC), Sky's own; wipe and iris: a fade (Sky has neither)",
    ),
    ("banner", Export::Close, "message_Narration on the black screen (Sky's chapter banners need a picture of their own and say “Chapter n”)"),
    ("define", Export::Direct, "nothing: recipes are written out before the export"),
    ("repeat", Export::Direct, "nothing: its turns are written out before the export"),
    ("if", Export::Direct, "if ( $LOCALn … ) { }: a film's variables are the scene's four local ones"),
    ("else", Export::Direct, "} else {"),
    ("end", Export::Direct, "nothing (closes a block)"),
    ("choice", Export::Direct, "switch ( message_SwitchMenu(0, 1) ) { case menu(\"…\"): … }, in the box of the line before"),
    ("set", Export::Direct, "$LOCALn = value (the scene's own; lost once it ends)"),
    // Speech
    (
        "(line)",
        Export::Direct,
        "message_SetFace(ACTOR, FACE_…, FACE_POS_STANDARD) (a face the species lacks: its normal one; none: message_SetActor, its name alone), message_Talk, message_Close",
    ),
    ("think", Export::Direct, "message_SetFace, message_Monologue"),
    ("narrate", Export::Direct, "box: message_ResetActor, message_Talk; screen: message_Narration; sign: message_Explanation"),
    ("box", Export::Close, "top: the portrait on top (FACE_POS_TOP_L_FACEINW); Sky keeps the box at the bottom"),
    ("explain", Export::Direct, "ExecuteCommon(CORO_EXPLANATION_FUNC_SERIES), Sky's own"),
    ("whisper", Export::Close, "its words: walks over, an emote; no smaller text"),
    // Presence
    ("put", Export::Direct, "the actor's place and direction in the .ssa; later: SetPositionMark, SetDirection"),
    ("spot", Export::Direct, "nothing: a named point, read into positions"),
    ("enter", Export::Direct, "SetPositionMark off screen, ResetOutputAttribute(128) (hidden until then), MovePositionMark"),
    ("exit", Export::Direct, "MovePositionMark off screen, SetOutputAttribute(128) (hidden)"),
    ("appear", Export::Close, "smoke: SetEffect(99, smoke puff); shown with ResetOutputAttribute(128): at once, no fading in"),
    ("vanish", Export::Close, "smoke: SetEffect(99); hidden with SetOutputAttribute(128): at once, no fading out"),
    ("crowd", Export::Close, "a free actor each (45 a film, 24 a scene), round the place facing it; no fidgeting"),
    // Formations
    ("form", Export::Direct, "one MovePositionMark each, then their waits"),
    // Movement
    (
        "walk",
        Export::Direct,
        "MovePositionMark<actor>(1, Position) (slow 0.5, brisk faster), WaitExecuteLives; via: a wait at each point; backward: SlidePositionMark",
    ),
    ("run", Export::Direct, "MovePositionMark<actor>(2, Position), WaitExecuteLives"),
    ("shift", Export::Direct, "SlidePositionMark<actor>(speed, Position)"),
    ("jump", Export::Close, "MoveHeight up and down; to a place: a run there (no arc)"),
    ("hop", Export::Direct, "ExecuteCommon<actor>(CORO_JUMP_HAPPY|SURPRISE|ANGRY_FUNC_SERIES, 0), WaitExecuteLives"),
    ("follow", Export::Close, "after each move of the leader: Wait(16), MovePositionLives<follower>(speed, LEADER) (tools/ds/LIMITS.md)"),
    ("chase", Export::Close, "MovePositionLives<actor>(3, OTHER): runs to where the other is, once"),
    ("flee", Export::Direct, "MovePositionMark away, at a run"),
    ("wander", Export::Direct, "ExecuteCommon<actor>(CORO_MOVE_RANDOM, 0), Sky's own stroll; off: back to its place"),
    // Turning
    ("face", Export::Direct, "Turn2DirectionLives<actor>(4, 10, OTHER) / Turn2Direction(4, 10, DIR_…)"),
    ("look", Export::Direct, "ExecuteCommon(CORO_LOOK_AROUND[_LEFT|_RIGHT|_DOWN]_FUNC_SERIES), Sky's own; up: Turn2Direction"),
    ("nod", Export::Direct, "SetAnimation (its nod, once), then 2"),
    ("shake_head", Export::Direct, "ExecuteCommon(CORO_HEAD_SHAKE_FUNC_SERIES), Sky's own"),
    // Body
    (
        "pose",
        Export::Direct,
        "SetAnimation<actor>(n): the group of that species' own sprite showing it (ds_animations.txt: Dusknoir lies with its 27), through Sky's table (once or looped); off: 2",
    ),
    ("sleep", Export::Direct, "SetAnimation(62), as Sky's INIT_SLEEP; off: its waking, once (63)"),
    ("eat", Export::Direct, "SetAnimation (eat), SetEffect(EFFECT_EATING); no item shown"),
    ("hurt", Export::Close, "its pain animation, once; no knock back"),
    ("tremble", Export::Close, "its pain animation, looped: Sky has no shiver"),
    ("fall", Export::Direct, "SetAnimation (tumble), once"),
    ("faint", Export::Direct, "SetAnimation (faint), once; off: waking"),
    ("cower", Export::Close, "its words: cringe, pain looped, an emote"),
    ("hug", Export::Close, "its words: walks beside the other, both hop"),
    ("comfort", Export::Direct, "its words: walks beside, nods, the other cheered"),
    ("cheer", Export::Direct, "its words: hops, pose"),
    // Emotions
    ("emote", Export::Direct, "SetEffect<actor>(EFFECT_…, 3): the engine's emote numbers are the game's; EFFECT_NONE once the line is said"),
    ("react", Export::Direct, "its words: an emote, a hop"),
    ("sigh", Export::Direct, "its words: a deep breath, sweat"),
    // Camera
    ("frame", Export::Direct, "MovePositionMark<performer 0> (the camera, camera_SetMyself) to between them; snap: SetPositionMark"),
    ("glide", Export::Direct, "MovePositionMark<performer 0>(speed, Position), WaitExecutePerformer(0)"),
    ("shake", Export::Direct, "camera_SetEffect(2, 1, 1.0) soft, (2, 2, 3.0) hard, (0, 0, 0) off, as Sky's"),
    // Screen
    ("flash", Export::Direct, "white: screen_WhiteOut(1, 2), screen_FadeIn; a colour: screen_FlushOut / FlushIn(1, n, 2, r, g, b)"),
    ("memory", Export::Direct, "screen_FlushOut(1, 0, 8, 0, 0, 0): Sky's own sepia of memories; screen_FlushIn(1, 0, 1, …) after"),
    // Sound
    ("music", Export::Direct, "bgm_Play(n) / bgm_PlayFadeIn(n, d, 256) / bgm_FadeOut(d): the engine's song numbers are the game's"),
    ("sound", Export::Close, "se_Play(n): the engine's numbers are the game's; on someone: not placed; a move's own sound: none"),
    ("jingle", Export::Direct, "me_Play(n)"),
    // Effects
    ("effect", Export::Close, "SetEffect<actor>(n), WaitEffect: a script effect of the move's type (fire, water, lightning…), not the move's own"),
    (
        "weather",
        Export::Close,
        "SetEffect<performer 3>(rain 698, Sky's very heavy rain of its scenes / hail 20; snow as hail); sand, fog, sun…: none in scripts",
    ),
    (
        "daytime",
        Export::Close,
        "screen_FlushOut(1, d, 4, 224, 176, 136) evening, (…, 4, 80, 112, 144) night (the light of Sky's own evening and night guild crossroads); day: screen_FlushIn; the light's own colour is lost; as a scene opens, the place's own level gives it",
    ),
    (
        "status",
        Export::Close,
        "an effect on the actor: confusion's stars (17), poison bubbles (1), Zs (51), ice (23), stats up (691) / down (685); others none",
    ),
    // Objects
    ("hold", Export::None, "no held item on an actor; an object actor placed by hand"),
    ("drop", Export::None, "no held item on an actor"),
    ("give", Export::None, "no held item on an actor"),
    ("throw", Export::None, "no thrown item in scripts"),
    ("open", Export::Direct, "the thing put in the scene's layout (an object of the game); SetAnimation<object>(17): its animation once"),
    ("close", Export::Direct, "SetAnimation<object>(9): its first picture, held"),
    // Fights
    ("fight", Export::None, "the game fights in dungeons only: main_EnterDungeon to a boss floor"),
    (
        "attack",
        Export::Close,
        "its pain once (a crouch), its own attack (group 1), a script effect of the move's type on the target, a small shake: not the move's own animation (dungeons only)",
    ),
    (
        "evolve",
        Export::Close,
        "a stats-up shine, the screen white, the evolved one (an actor of its own) put where it stood and shown, the other hidden; no change of species (Sky's scripts have none)",
    ),
    ("join", Export::Direct, "its emote and hop, then as Sky: me_Play(5), message_Mail(\"[CN]… joined the team!\"), WaitMe(5)"),
    (
        "roar",
        Export::Direct,
        "as Sky's bosses: se_Play(6930), camera_SetEffect(2, 1, 1.0) (hard: 2, 2, 3.0) a while, then 0, 0, 0; SetAnimation of the sprite's own Roar group once, else its Charge looped (9), as Groudon's",
    ),
    (
        "charge",
        Export::Direct,
        "SetAnimation (its Charge, looped: 9), SetEffect(691, 3) (Sky's stats-up shine), se_Play(4881), a beat, standing; the charged attack's flash and shake as written (screen_WhiteOut, camera_SetEffect)",
    ),
    (
        "dodge",
        Export::Close,
        "Sky's hop (CORO_JUMP_HAPPY_FUNC_SERIES) and a quick MovePositionMark aside, the miss's sound (se_Play); no \"Missed!\" (Sky's scenes have no floating words)",
    ),
    (
        "finish",
        Export::Close,
        "its words: the flash, the attack, the faint; bgm_ChangeVolume(0, 0) for the silence, then (0, 256); no slow motion (Sky's scripts have none)",
    ),
    // Time
    ("wait", Export::Direct, "Wait(frames) / WaitExecuteLives / all: each one moving, WaitExecutePerformer(0)"),
    ("together", Export::Direct, "each one's moves told at once (actors move on their own), then their WaitExecuteLives; its waits are lost"),
    ("mark", Export::Direct, "a @label; what is timed from it is already written as waits"),
    ("at", Export::Work, "positions and frame counts kept by the exporter"),
    ("pace", Export::Direct, "nothing: the waits are written already paced"),
    // Story
    ("option", Export::Direct, "case menu(\"…\"):"),
    ("{if}", Export::Work, "message_SwitchTalk on a variable"),
    ("{value}", Export::Work, "[value:…] in the text"),
    ("go", Export::Close, "go moment: jump @label; go scene: one scene a film on DS"),
    ("seen()", Export::Work, "a game variable counted"),
    ("#id", Export::Direct, "nothing: an id for translators"),
    ("keep", Export::Work, "a game variable"),
    // Dungeon (`uses dungeon`)
    ("stairs", Export::None, "the game's stairs are a dungeon's: main_EnterDungeon"),
    ("trap", Export::None, "traps are a dungeon's"),
    ("ambush", Export::None, "an ambush is a dungeon's fight"),
    ("shop", Export::Work, "Kecleon's shop menu (ProcessSpecial)"),
    ("steal", Export::None, "stealing is a dungeon's"),
    // Guild (`uses guild`)
    ("job", Export::Work, "the guild's job lists (ProcessSpecial)"),
    ("rank", Export::Work, "the team's rank, a game variable"),
    ("morning_call", Export::Work, "Sky's own morning at the guild, its routine called"),
    ("dinner", Export::Work, "Sky's own dinner at the guild, its routine called"),
    // Fishing (`uses fishing`)
    ("cast", Export::None, "Sky has no fishing: no rod, line nor bobber"),
    ("bite", Export::None, "Sky has no fishing"),
    ("reel", Export::None, "Sky has no fishing"),
    ("release", Export::None, "Sky has no fishing"),
    // Advanced
    ("layer", Export::Work, "Sky draws its actors by their height on the screen; an attribute to set"),
    ("ghost", Export::None, "Sky's actors have no see-through"),
    ("tint", Export::Work, "screen_FlushOut in that colour"),
    ("float", Export::Work, "MoveHeight up and down, looped"),
    ("zoom", Export::None, "Sky's camera has no zoom"),
    ("bubble", Export::None, "Sky has no speech bubbles: its words are in the box"),
    ("light", Export::Work, "screen_FlushOut in the ambience's colour"),
    ("spotlight", Export::None, "Sky has no light on a part of the screen"),
    ("letterbox", Export::None, "Sky's scenes have no black bars"),
    ("slowmo", Export::None, "Sky's scripts have no slow motion"),
    ("spin", Export::Work, "Turn2Direction round, a turn after another"),
    ("flip", Export::None, "Sky's sprites are not turned over"),
    ("node", Export::None, "the engine's own nodes: Sky has none"),
    ("attach", Export::None, "the engine's own nodes: Sky has none"),
    ("detach", Export::None, "the engine's own nodes: Sky has none"),
    ("prop", Export::None, "the engine's own nodes: Sky has none"),
    ("timer", Export::Work, "a Wait counted"),
    ("every", Export::Work, "a routine looped beside the scene"),
    ("while", Export::Work, "while ( $LOCALn … ) { }"),
    ("random", Export::Work, "a random number of the game's"),
    ("when", Export::Work, "a routine waiting for someone to arrive"),
    ("beside", Export::Work, "Sky's routines beside the scene's (with(actor))"),
    ("player", Export::None, "a cutscene has no one the player walks"),
    ("dialogue", Export::None, "a cutscene has no one the player talks to"),
    ("explore", Export::None, "a cutscene has no exploring"),
    ("game", Export::None, "the game's own ground play: a cutscene has none"),
    ("way", Export::None, "the game's own ground play: a cutscene has none"),
];

/// What a word written with less than the film shows loses on the DS (its
/// remark in a film `for ds`, `R-DS-LOSS`): each `Close` word's.
pub const LOSSES: &[(&str, &str)] = &[
    ("transition", "a wipe or an iris becomes a fade"),
    ("banner", "a narration on black, not Sky's chapter picture"),
    ("box", "the box stays at the bottom: only the portrait goes on top"),
    ("whisper", "no smaller text"),
    ("appear", "no fading in: shown at once"),
    ("vanish", "no fading out: hidden at once"),
    ("crowd", "no fidgeting, and 24 at most a scene"),
    ("jump", "no arc to a place: a run there"),
    ("follow", "the follower catches up after each move, not step by step"),
    ("chase", "it runs to where the other was, once"),
    ("hurt", "no knock back"),
    ("tremble", "no shiver: its pain animation looped"),
    ("cower", "an emote and its pain looped"),
    ("hug", "they stand side by side and hop"),
    ("sound", "not placed on someone; a move's own sound is none"),
    ("effect", "a script effect of the move's type, not the move's own"),
    ("weather", "sand, fog and sun show nothing; snow is hail"),
    ("daytime", "the light's own colour"),
    ("status", "only confusion, poison, sleep, ice and stats show"),
    ("attack", "a crouch, the attack and an effect of its type: not the move's own animation"),
    ("evolve", "no change of species: the evolved one is another actor"),
    ("dodge", "the “Missed!” won't show"),
    ("finish", "no slow motion"),
    ("go", "a film is one scene after another on DS: “go scene” is not written"),
];

/// How to write a word the DS doesn't have another way (its fix in a film
/// `for ds`, `E-DS-WORD`).
pub const FIXES: &[(&str, &str)] = &[
    ("fight", "write the fight as blows: “attack”, “hurt”, “faint” and lines"),
    ("hold", "say it in a line, or show it with “effect”"),
    ("drop", "say it in a line"),
    ("give", "say it in a line, with “join” or “emote”"),
    ("throw", "write it with “attack” or “effect … from a to b”"),
    ("zoom", "frame them with “frame” or “glide”"),
    ("bubble", "a line said in the box: “who: text”"),
    ("slowmo", "“wait” between the beats"),
    ("flip", "“face” or “fall”"),
    ("cast", "“pose” and “effect” say it"),
    ("ghost", "“appear … fade” or “vanish”"),
];

impl Export {
    /// Whether it is written in the export (with or without a loss).
    pub fn exports(self) -> bool {
        matches!(self, Export::Direct | Export::Close)
    }

    /// Its badge, as the Wiki shows it: `DS direct`, `DS with loss`, `not on DS`.
    pub fn badge(self) -> &'static str {
        match self {
            Export::Direct => "DS direct",
            Export::Close => "DS with loss",
            Export::Work => "not on DS yet",
            Export::None => "not on DS",
        }
    }

    /// Its word in `export-ds --table`.
    pub fn kind(self) -> &'static str {
        match self {
            Export::Direct => "direct",
            Export::Close => "close",
            Export::Work => "work",
            Export::None => "none",
        }
    }
}

/// What it is written as on the DS: the game's operations.
pub fn how(word: &str) -> Option<&'static str> {
    WORDS.iter().find(|(name, _, _)| *name == word).map(|(_, _, how)| *how)
}

/// What a word written with less loses on the DS.
pub fn lost(word: &str) -> Option<&'static str> {
    LOSSES.iter().find(|(name, _)| *name == word).map(|(_, lost)| *lost)
}

/// How to write it another way, for a word the DS doesn't have.
pub fn fix(word: &str) -> Option<&'static str> {
    FIXES.iter().find(|(name, _)| *name == word).map(|(_, fix)| *fix)
}

/// What a film `for ds` is told of a word that doesn't export: why, and
/// how to write it another way.
pub fn refused(word: &str) -> String {
    let why = match export_of(word) {
        Some(Export::Work) => "the export doesn't write it yet",
        Some(Export::None) => how(word).unwrap_or("Sky has nothing for it"),
        _ => "not a word the export knows",
    };
    match fix(word) {
        Some(fix) => format!("“{word}” doesn't export to DS ({why}): {fix}"),
        None => format!("“{word}” doesn't export to DS ({why})"),
    }
}

/// The families the export table covers: the core ones (not the optional
/// nor the advanced).
pub fn core_words() -> impl Iterator<Item = &'static crate::plan::Planned> {
    crate::plan::WORDS.iter().filter(|word| crate::plan::FAMILIES.contains(&word.family))
}

/// How that word exports, if the table has it.
pub fn export_of(word: &str) -> Option<Export> {
    WORDS.iter().find(|(name, _, _)| *name == word).map(|(_, export, _)| *export)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_word_of_the_plan_has_its_ds_status() {
        let missing: Vec<&str> = crate::plan::WORDS.iter().map(|word| word.word).filter(|word| export_of(word).is_none()).collect();
        assert!(missing.is_empty(), "no DS status for {missing:?}");
        for (at, (word, export, how)) in WORDS.iter().enumerate() {
            assert!(crate::plan::WORDS.iter().any(|planned| planned.word == *word), "“{word}” is no word of the plan");
            assert!(!WORDS[..at].iter().any(|(known, _, _)| known == word), "“{word}” twice");
            assert!(!how.is_empty(), "{word}: no note");
            assert_eq!(*export == Export::Close, lost(word).is_some(), "{word}: a loss said if and only if it exports with one");
        }
        for (word, _) in FIXES {
            assert!(!export_of(word).is_some_and(Export::exports), "{word} exports: no fix needed");
        }
        assert!(refused("fight").contains("attack"));
    }

    #[test]
    fn every_core_word_has_its_line_in_the_export_table() {
        let missing: Vec<&str> = core_words().map(|word| word.word).filter(|word| export_of(word).is_none()).collect();
        assert!(missing.is_empty(), "no DS export line for {missing:?}");
    }
}
