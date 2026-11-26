//! All that names the world of this game in its scenes: its faces, songs,
//! sounds, animations, statuses, and the commands only it can play.

use engine::Vocabulary;

/// The number the game keeps the team's money in (see `getmoney`).
pub const MONEY: &str = "money";

/// How the numbers telling how many of an item the team has begin.
const ITEM_NUMBERS: &str = "item_";

/// The emotes by what code names them (`emote::SWEAT`): a name mistyped
/// is a mistake the compiler finds. A film names them by their words
/// (`POKEMON.emote`).
pub mod emote {
    use engine::EmoteId;

    // (By the ROM's own names of its effects: ExclamationMark, QuestionMark,
    // Joyous, Shocked, SweatDrop, Laughing, Angry,
    // SweatDropsFromBothSidesMedium, TwoArrowsAtSide, Eating.)
    pub const EXCLAIM: EmoteId = EmoteId(88);
    pub const QUESTION: EmoteId = EmoteId(89);
    pub const JOY: EmoteId = EmoteId(90);
    pub const SHOCK: EmoteId = EmoteId(91);
    pub const DROP: EmoteId = EmoteId(92);
    pub const LAUGH: EmoteId = EmoteId(93);
    pub const ANGER: EmoteId = EmoteId(94);
    pub const SWEAT: EmoteId = EmoteId(171);
    pub const NOTICE: EmoteId = EmoteId(172);
    pub const MUNCH: EmoteId = EmoteId(642);
    pub const DUST: EmoteId = EmoteId(54);
    pub const SMOKE: EmoteId = EmoteId(100);
    pub const NONE: EmoteId = EmoteId(0);
}

/// Animations by what code names them (`anim::ATTACK`), typed. A film
/// names them by their words (`POKEMON.animation`).
pub mod anim {
    use engine::AnimId;

    pub const ATTACK: AnimId = AnimId(1);
    pub const EAT: AnimId = AnimId(15);
    pub const TRIP: AnimId = AnimId(26);
}

pub static POKEMON: Vocabulary = Vocabulary {
    host_now: &["getmoney", "getitem", "takeitem", "recruit", "top", "topcam", "topimage", "map", "team", "bag"],
    host_course: &["dungeon", "ground", "menu", "open", "unlock", "fight"],
    host_holds: &["menu", "fight"],
    host_numbers: &[("getmoney", 1), ("getitem", 2), ("takeitem", 2)],
    host_counts: &[("getmoney", MONEY)],
    host_asks: crate::commands::game_ask,
    no_item: "Nothing",
    item_name: ("{item:", "}"),
    item_numbers: ITEM_NUMBERS,
    faces: &[
        ("normal", 0),
        ("happy", 1),
        ("pain", 2),
        ("angry", 3),
        ("worried", 4),
        ("sad", 5),
        ("crying", 6),
        ("shouting", 7),
        ("teary_eyed", 8),
        ("determined", 9),
        ("joyous", 10),
        ("inspired", 11),
        ("surprised", 12),
        ("dizzy", 13),
        ("sigh", 16),
        ("stunned", 17),
        // (The community's Special0 to Special3: a face of its own some species have.)
        ("special_one", 14),
        ("special_two", 15),
        ("special_three", 18),
        ("special_four", 19),
    ],
    // (By the original's numbers. `leaping_faces`: joy, surprise.)
    face_emotes: &[(3, 94), (4, 92), (5, 171), (6, 171), (8, 171), (1, 93), (10, 93), (12, 88), (13, 89), (16, 92), (17, 172)],
    leaping_faces: &[10, 12],
    // (What the original's scripts play beside them most: "!", "?", a drop,
    // shock, anger, sweat, notice.)
    // (As their pictures show them: `data/rom/effects`.)
    emotes: &[
        ("exclaim", emote::EXCLAIM.0),
        ("question", emote::QUESTION.0),
        ("joy", emote::JOY.0),
        ("shock", emote::SHOCK.0),
        ("drop", emote::DROP.0),
        ("laugh", emote::LAUGH.0),
        ("anger", emote::ANGER.0),
        ("sweat", emote::SWEAT.0),
        ("notice", emote::NOTICE.0),
        ("munch", emote::MUNCH.0),
        ("dust", emote::DUST.0),
        ("smoke", emote::SMOKE.0),
        ("none", emote::NONE.0),
    ],
    songs: rules::songs::SONG_WORDS,
    // (Each named by what it sounds like.)
    sounds: rules::songs::NOISE_WORDS,
    // (As the original game numbers them and the PMDCollab community names
    // them.)
    animations: &assets::tables::ANIMATION_GROUPS,
    // (The game's own names, those of the pictures of `data/rom/status`.)
    status_icons: &[
        "sleep",
        "burn",
        "poison",
        "toxic",
        "curse",
        "frozen",
        "confused",
        "cringe",
        "protect",
        "safeguard",
        "reflect",
        "light_screen",
        "endure",
        "low_hp",
        "lowered",
        "sleepless",
        "taunt",
        "encore",
        "embargo",
        "sure_shot",
        "set_damage",
        "focus_energy",
        "cross_eyed",
        "grudge",
        "heal_block",
    ],
    given_numbers: &[
        "money",
        "items",
        "hero_species",
        "partner_species",
        "hero",
        "partner",
        "team",
        "answer",
        "recruits",
        "presses",
        "talking",
        "target",
        "timer",
        "clock",
        "music_clock",
        "press_clock",
        "arrows",
        "arrow",
        "arrow_clock",
        "controlled",
        "hp",
        "hp2",
        "champion",
        "summary_blows",
        "summary_crits",
        "summary_misses",
        "summary_hp",
    ],
    given_beginnings: &[
        ITEM_NUMBERS,
        "recruit_",
        "bag_",
        "game_",
        "hero_level",
        "partner_level",
        "recruits",
        "map_choice",
        "dungeon_message",
        "team_size",
        "rank_name",
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_typed_names_are_the_words_of_the_game() {
        assert_eq!(POKEMON.emote("exclaim"), Some(88));
        assert_eq!(POKEMON.emote("sweat"), Some(171));
        assert_eq!(POKEMON.emote("joy"), Some(90));
        assert_eq!(POKEMON.emote("anger"), Some(94));
        assert_eq!(POKEMON.animation("attack"), Some(usize::from(anim::ATTACK.0)));
        assert_eq!(POKEMON.animation("Eat"), Some(usize::from(anim::EAT.0)));
        assert_eq!(POKEMON.animation("Trip"), Some(usize::from(anim::TRIP.0)));
        assert_eq!(POKEMON.sound("click"), Some(rules::songs::noise::CLICK));
    }
}
