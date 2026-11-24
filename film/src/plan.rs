//! The vocabulary the films' language is being rebuilt to (the v2.1 core):
//! its families in their order, each word with its form as an author writes
//! it, who first when someone acts (`who walk to where`), the word first
//! when nobody does (`music track`). The optional families come with
//! `uses dungeon|guild|fishing`; the advanced words are folded away.
//!
//! Every word of the language is in one of these families: the catalogue,
//! the Wiki and the completion show them so (`family_of`); an engine's
//! command the plan does not name is an advanced word. A word of the old
//! vocabulary ([`REPLACED`]) is refused, its new form given as the fix
//! (`told_replaced`). A word of the plan the language does not have yet is
//! "to build" ([`pending`]); the quality gate (`workshop/src/wiki/tests.rs`)
//! asks the others a page and an example that plays with nothing to say.

/// The families of the core, in their order.
pub const FAMILIES: [&str; 16] = [
    "Structure",
    "Speech",
    "Presence",
    "Formations",
    "Movement",
    "Turning",
    "Body",
    "Emotions",
    "Camera",
    "Screen",
    "Sound",
    "Effects",
    "Objects",
    "Fights",
    "Time",
    "Story",
];

/// The optional families, each with the built-in library that brings it
/// into a film (`uses dungeon`).
pub const OPTIONAL: [(&str, &str); 3] = [("Dungeon", "dungeon"), ("Guild", "guild"), ("Fishing", "fishing")];

/// The words for those who want to control everything: folded by default.
pub const ADVANCED: &str = "Advanced";

/// What a word of the plan is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Made {
    /// A word of the language (a family of `film/src/verbs/`).
    Word,
    /// A recipe written in the language, provided (`film/recipes/`).
    Recipe,
    /// A way of writing of the language itself, no word to look up (a
    /// line said, `{value}` in it): it has no page of its own.
    Syntax,
}

/// A word of the plan: its family, itself, its form as an author writes it.
#[derive(Clone, Copy, Debug)]
pub struct Planned {
    pub family: &'static str,
    pub word: &'static str,
    pub form: &'static str,
    pub made: Made,
}

const fn word(family: &'static str, word: &'static str, form: &'static str) -> Planned {
    Planned { family, word, form, made: Made::Word }
}

const fn recipe(family: &'static str, word: &'static str, form: &'static str) -> Planned {
    Planned { family, word, form, made: Made::Recipe }
}

const fn syntax(family: &'static str, word: &'static str, form: &'static str) -> Planned {
    Planned { family, word, form, made: Made::Syntax }
}

/// Every word of the plan: the core's, the provided recipes, the optional
/// families', the advanced ones.
pub const WORDS: &[Planned] = &[
    // Structure
    word("Structure", "title", "title name"),
    word("Structure", "scene", "scene: place who..."),
    word("Structure", "moment", "moment: name"),
    word("Structure", "transition", "transition fade|white|boss|wipe|iris [in|out] [d]"),
    word("Structure", "banner", "banner chapter|place|dungeon text"),
    syntax("Structure", "define", "define name ... end"),
    word("Structure", "repeat", "repeat n ... end"),
    syntax("Structure", "if", "if test ... else ... end"),
    syntax("Structure", "else", "if test ... else ... end"),
    syntax("Structure", "end", "... end"),
    syntax("Structure", "choice", "choice ... option ... end"),
    word("Structure", "set", "set flag [value] / set flag off"),
    // Speech
    syntax("Speech", "(line)", "who (face): text"),
    word("Speech", "think", "who think (face): text"),
    word("Speech", "narrate", "narrate [box|screen|sign]: text"),
    word("Speech", "box", "box top|bottom|auto"),
    recipe("Speech", "explain", "who explain"),
    recipe("Speech", "whisper", "who whisper other"),
    // Presence
    word("Presence", "put", "who put where [facing side]"),
    word("Presence", "spot", "spot name x y"),
    word("Presence", "enter", "who enter [from side] [to where]"),
    word("Presence", "exit", "who exit [via side]"),
    word("Presence", "appear", "who appear [where] smoke|fade|flash|vision|light"),
    word("Presence", "vanish", "who vanish smoke|fade|flash"),
    word("Presence", "crowd", "crowd species n area"),
    // Formations
    word("Formations", "form", "form row|column|ring|lineup who... [to where] [walk|run]"),
    // Movement
    word("Movement", "walk", "who walk to where [slow|brisk] [backward] [via a b...]"),
    word("Movement", "run", "who run to where"),
    word("Movement", "shift", "who shift to|by where"),
    word("Movement", "jump", "who jump [to where] [height h]"),
    word("Movement", "hop", "who hop [happy|surprised|angry] [times n]"),
    word("Movement", "follow", "who follow other / who follow off"),
    word("Movement", "chase", "who chase other / who chase off"),
    word("Movement", "flee", "who flee from other [to where]"),
    word("Movement", "wander", "who wander [area] / who wander off"),
    word("Movement", "ride", "who ride on other [back|head] [high h] [behind b]"),
    word("Movement", "get_off", "who get_off [to where]"),
    // Turning
    word("Turning", "face", "who face side|other|away [fast|slow]"),
    word("Turning", "look", "who look around|left|right|up|down"),
    word("Turning", "nod", "who nod [times n]"),
    word("Turning", "shake_head", "who shake_head"),
    // Body
    word("Body", "pose", "who pose animation [once|loop|hold] / who pose off"),
    word("Body", "sleep", "who sleep / who sleep off"),
    word("Body", "eat", "who eat [item]"),
    word("Body", "hurt", "who hurt [hard] [back n]"),
    word("Body", "tremble", "who tremble / who tremble off"),
    word("Body", "fall", "who fall [over|back|trip]"),
    word("Body", "faint", "who faint / who faint off"),
    word("Body", "carry", "who carry other"),
    word("Body", "put_down", "who put_down other [to where]"),
    recipe("Body", "cower", "who cower [from other]"),
    recipe("Body", "hug", "who hug other"),
    recipe("Body", "comfort", "who comfort other"),
    recipe("Body", "cheer", "who cheer"),
    // Emotions
    word("Emotions", "emote", "who emote exclaim|shock|sweat|joy|laugh|question|notice|anger|munch [silent] / who emote off"),
    recipe("Emotions", "react", "who react joy|surprise|anger|fear|doubt|relief"),
    recipe("Emotions", "sigh", "who sigh"),
    // Camera
    word("Camera", "frame", "frame who... [where] [snap]"),
    word("Camera", "glide", "glide to where|who [in d]"),
    word("Camera", "shake", "shake [soft|hard] / shake off"),
    // Screen
    word("Screen", "flash", "flash [white|colour]"),
    recipe("Screen", "memory", "memory ... end"),
    // Sound
    word("Sound", "music", "music track [fade d] [volume v] / music off [fade d]"),
    word("Sound", "sound", "sound name [on who] [loop] / sound name off"),
    word("Sound", "jingle", "jingle name"),
    // Effects
    word("Effects", "effect", "effect name on who|at where|from a to b"),
    word("Effects", "weather", "weather rain|snow|sand|fog|hail|sun [heavy] / weather off"),
    word("Effects", "daytime", "daytime morning|day|evening|night [stars]"),
    word("Effects", "status", "who status state|stat up|down / who status off"),
    // Objects
    word("Objects", "hold", "who hold item"),
    word("Objects", "drop", "who drop [where]"),
    word("Objects", "give", "who give item to other"),
    word("Objects", "throw", "who throw item at other|where"),
    word("Objects", "open", "open thing"),
    word("Objects", "close", "close thing"),
    // Fights
    word("Fights", "fight", "fight who... vs who... [duel|double|relay] [boss] [level n]"),
    word("Fights", "attack", "who attack Move [at target]"),
    word("Fights", "evolve", "who evolve species"),
    word("Fights", "join", "who join [team]"),
    word("Fights", "roar", "who roar [hard]"),
    word("Fights", "charge", "who charge [Move]"),
    word("Fights", "dodge", "who dodge [left|right|back]"),
    recipe("Fights", "finish", "who finish target Move"),
    // Time
    word("Time", "wait", "wait d|who|all|sound|music"),
    syntax("Time", "together", "together ... end"),
    word("Time", "mark", "mark name / at name + d:"),
    syntax("Time", "at", "at name [+ d]: ... / at 12s: ..."),
    word("Time", "pace", "pace slow|brisk|normal|n / pace lines d|off"),
    // Story
    syntax("Story", "option", "option \"text\" [if test] [once]"),
    syntax("Story", "{if}", "{if test: text | other}"),
    syntax("Story", "{value}", "{value}"),
    word("Story", "go", "go moment name / go scene name"),
    syntax("Story", "seen()", "seen(name)"),
    syntax("Story", "#id", "who: text #id"),
    syntax("Story", "keep", "keep flag = value"),
    // Dungeon (uses dungeon)
    word("Dungeon", "stairs", "stairs [where] / stairs take who..."),
    word("Dungeon", "trap", "trap kind [on who] / trap off"),
    word("Dungeon", "ambush", "ambush who... on other"),
    word("Dungeon", "shop", "who shop item price... / who shop buy item"),
    word("Dungeon", "steal", "who steal item"),
    // Guild (uses guild)
    word("Guild", "job", "job take|done|fail name"),
    word("Guild", "rank", "rank team rank"),
    recipe("Guild", "morning_call", "morning_call"),
    recipe("Guild", "dinner", "dinner"),
    // Fishing (uses fishing)
    word("Fishing", "cast", "who cast [at where]"),
    word("Fishing", "bite", "bite [strong|nibble]"),
    word("Fishing", "reel", "who reel [catch what|snap|lose]"),
    word("Fishing", "release", "who release item"),
    // Advanced: Sky's own workings, the two words Sky never uses, the engine's commands
    word(ADVANCED, "layer", "who layer front|behind"),
    word(ADVANCED, "ghost", "who ghost"),
    word(ADVANCED, "tint", "tint colour"),
    word(ADVANCED, "float", "who float"),
    word(ADVANCED, "zoom", "zoom n"),
    word(ADVANCED, "bubble", "who bubble text"),
    word(ADVANCED, "light", "light ambience"),
    word(ADVANCED, "spotlight", "spotlight who"),
    word(ADVANCED, "letterbox", "letterbox"),
    word(ADVANCED, "slowmo", "slowmo"),
    word(ADVANCED, "spin", "who spin"),
    word(ADVANCED, "flip", "who flip"),
    word(ADVANCED, "node", "node name kind"),
    word(ADVANCED, "attach", "who attach parent"),
    word(ADVANCED, "detach", "who detach"),
    word(ADVANCED, "prop", "prop who property value..."),
    word(ADVANCED, "timer", "timer name d"),
    syntax(ADVANCED, "every", "every d ... end"),
    syntax(ADVANCED, "while", "while test ... end"),
    word(ADVANCED, "random", "random a b"),
    word(ADVANCED, "when", "when who arrives: ..."),
    syntax(ADVANCED, "beside", "beside [name] ... end"),
    // Advanced: the lines of a place's scene the game makes (`game/src/roam.rs`)
    word(ADVANCED, "player", "player who [followed by friend]"),
    word(ADVANCED, "dialogue", "dialogue who ... end"),
    word(ADVANCED, "explore", "explore"),
    word(ADVANCED, "game", "game words..."),
    word(ADVANCED, "way", "way x y width height words..."),
];

/// The words of the old vocabulary, each with the form a word of the plan
/// writes it in now (`""`: dropped, nothing in its place). A film's line
/// with one is refused, this form its fix (`told_replaced`).
pub const REPLACED: &[(&str, &str)] = &[
    // Structure
    ("chapter", "banner chapter text"),
    ("chapter_over", "banner chapter text"),
    ("story", "banner chapter text"),
    ("place_title", "banner place text"),
    ("title_card", "banner place text"),
    ("enter_dungeon", "banner dungeon text"),
    ("fadein", "transition fade in [d]"),
    ("fadeout", "transition fade out [d]"),
    ("whitein", "transition white in [d]"),
    ("whiteout", "transition white out [d]"),
    ("iris", "transition iris in|out [d]"),
    ("wipe", "transition wipe in|out [d]"),
    ("clear", "set flag off"),
    ("opening", ""),
    // Speech
    ("thought", "who think (face): text"),
    ("sign", "narrate sign: text"),
    // Presence
    ("put_there", "who put where [facing side]"),
    ("run_in", "who enter [from side] [to where]"),
    // Formations
    ("row", "form row who... [where]"),
    ("column", "form column who... [where]"),
    ("triangle", "form ring who... [where]"),
    ("gather", "form ring who... [where]"),
    ("lineup", "form lineup who... [where]"),
    ("rank", "form lineup who... [where]"),
    // Movement
    ("advance", "who walk to where"),
    ("path", "who walk to where via a b..."),
    ("runpath", "who run to where"),
    ("train", "who follow other"),
    ("leap_along", "who jump to where"),
    ("shove", "who shift by dx dy"),
    ("push", "who shift by dx dy"),
    ("orbit", ""),
    ("come_back", ""),
    ("hold_back", ""),
    ("find_place", ""),
    // Turning
    ("turn", "who face side|other|away"),
    ("face_dir", "who face side"),
    ("look_around", "who look around"),
    ("search", "who look around"),
    ("nod_yes", "who nod"),
    ("shake_no", "who shake_head"),
    // Body
    ("nap", "who sleep"),
    ("wake", "who sleep off"),
    ("wake_up", "who sleep off"),
    ("snore", "who sleep"),
    ("blow", "who hurt"),
    ("trip", "who fall trip"),
    ("shiver", "who tremble"),
    ("take_cover", "who cower from other"),
    ("buddies", "who hug other"),
    ("greet", "who hop happy"),
    ("bravo", "who cheer"),
    ("clap", "who cheer"),
    ("dance", "who pose spin"),
    ("pant", "who pose breathe"),
    ("yawn", "who pose breathe"),
    // Emotions
    ("joy", "who react joy"),
    ("surprise", "who react surprise"),
    ("fear", "who react fear"),
    ("anger", "who react anger"),
    ("question", "who react doubt"),
    ("ponder", "who react doubt"),
    ("laugh", "who emote laugh"),
    ("sadness", "who emote sweat"),
    ("blush", "who emote sweat"),
    ("red_face", "who emote anger"),
    ("fume", "who emote anger"),
    ("in_love", "who emote joy"),
    ("startle", "who react surprise"),
    // Camera
    ("view", "frame who... follows"),
    ("camera", "glide to where|who [in d]"),
    ("shot", "frame who...|where"),
    ("scroll", "glide to where [in d]"),
    ("dolly", "glide to where [in d]"),
    ("pan", "glide to where|who [in d]"),
    ("quake", "shake [soft|hard] [d]"),
    ("wave", "shake [soft|hard] [d]"),
    // Screen
    ("grey", "memory ... end"),
    // Sound
    ("ambient", "sound name loop"),
    ("cease", "sound name off"),
    ("fade", "music off fade d"),
    ("volume", "music track volume v"),
    ("fademusic", "music off fade d"),
    ("playsound", "sound name"),
    ("movesound", "sound name"),
    ("fadesound", "sound name off"),
    // Effects
    ("sky", "weather rain|snow|sand|fog|hail|sun [heavy]"),
    ("moveeffect", "effect name on who"),
    ("moveshot", "effect name from a to b"),
    ("starry_night", "daytime night stars"),
    ("status_asleep", ""),
    ("status_burned", ""),
    ("status_frozen", ""),
    ("status_poisoned", ""),
    ("status_confused", ""),
    ("status_afraid", ""),
    ("status_paralyzed", ""),
    // Objects
    ("set_down", "who drop at where"),
    ("pick_up", "who hold item"),
    ("catch", "who throw item at other"),
    ("miss", "who throw item at where"),
    ("shovel", ""),
    ("shovel_blow", ""),
    ("pitchfork", ""),
    ("prod", ""),
    ("mine", ""),
    ("pick_bounce", ""),
    ("split", ""),
    ("chop_down", ""),
    ("light_torch", ""),
    ("brandish", ""),
    ("crackle", ""),
    ("blow_out_torch", ""),
    ("shoulder_arms", ""),
    ("sniper_shot", ""),
    ("burst_on", ""),
    ("bazooka_shot", ""),
    ("pull_pin", ""),
    ("throw_grenade", ""),
    ("reload_sniper", ""),
    ("scope_glint", ""),
    ("sniper_miss", ""),
    ("track_sight", ""),
    ("load_rocket", ""),
    ("bazooka_recoil", ""),
    ("wild_rocket", ""),
    ("rocket_rain", ""),
    ("juggle", ""),
    ("hot_potato", ""),
    ("hit_the_dirt", ""),
    ("throw_smoke", ""),
    ("star_grenade", ""),
    ("sniper_burst", ""),
    ("lob_shot", ""),
    ("volley", ""),
    ("ricochet_shot", ""),
    ("crossfire", ""),
    ("stray_bullet", ""),
    ("grenade_there", ""),
    ("load_mortar", ""),
    ("mortar_shot", ""),
    ("mortar_fires", ""),
    ("barrage", ""),
    ("shell_stuck", ""),
    ("shell_planted", ""),
    // Fights
    ("move", "who attack Move [at target]"),
    ("duel", "fight who vs who"),
    ("double", "fight who... vs who... double"),
    ("relay", "fight who... vs who... relay"),
    ("boss", "fight who... vs who... boss"),
    ("recruit", "who join [team]"),
    ("new_recruit", "who join [team]"),
    ("stat", "who status stat up|down"),
    ("aim", ""),
    ("reload", ""),
    // Time
    ("waitall", "wait all"),
    ("waitfor", "wait who"),
    ("waitsound", "wait sound"),
    ("t", "wait d"),
    // Dungeon
    ("take_stairs", "stairs take who..."),
    ("step_on", "trap kind on who"),
    ("buy", "who shop buy item"),
    // Guild
    ("take_job", "job take name"),
    ("report", "job done|fail name"),
    ("mission_complete", "job done name"),
    ("team_rank", "rank team rank"),
    ("mission_posted", "narrate sign: text"),
    // Fishing
    ("reel_in", "who reel"),
    ("nibble", "bite nibble"),
    ("snap", "who reel snap"),
    ("fishing_report", ""),
];

/// The word of the plan of that name, if one.
pub fn planned(word: &str) -> Option<&'static Planned> {
    WORDS.iter().find(|planned| planned.word == word)
}

/// Whether the plan writes that word who first (`who walk to where`): none
/// if the plan does not have it.
pub fn who_first(word: &str) -> Option<bool> {
    planned(word).map(|planned| planned.form.starts_with("who ") || planned.form.starts_with("*who "))
}

/// The library a film takes in to have the words of that optional family
/// (`uses dungeon`), if it is one.
pub fn library_of(family: &str) -> Option<&'static str> {
    OPTIONAL.iter().find(|(name, _)| *name == family).map(|(_, library)| *library)
}

/// Every family in the order shown: the core's, the optional ones, the
/// advanced words last.
pub const SHOWN: [&str; 20] = [
    "Structure",
    "Speech",
    "Presence",
    "Formations",
    "Movement",
    "Turning",
    "Body",
    "Emotions",
    "Camera",
    "Screen",
    "Sound",
    "Effects",
    "Objects",
    "Fights",
    "Time",
    "Story",
    "Dungeon",
    "Guild",
    "Fishing",
    ADVANCED,
];

/// The same, in turn.
pub fn shown_families() -> impl Iterator<Item = &'static str> {
    SHOWN.iter().copied()
}

/// The family a word is shown in: the plan's, else [`ADVANCED`] (an
/// engine's command).
pub fn family_of(word: &str) -> &'static str {
    planned(word).map_or(ADVANCED, |planned| planned.family)
}

/// The form a word of the old vocabulary is written in now (`""`: dropped),
/// if it is one; none for a word of the plan.
pub fn replaced(word: &str) -> Option<&'static str> {
    if planned(word).is_some() {
        return None;
    }
    REPLACED.iter().find(|(old, _)| *old == word).map(|(_, now)| *now)
}

/// The words a form of the plan starts with before its first value:
/// `transition fade in [d]` gives `transition fade in`, `who walk to where`
/// `walk to` (who first: told apart).
fn literal_head(form: &str) -> (bool, Vec<&str>) {
    let mut words = form.split_whitespace().peekable();
    let who_first = words.peek() == Some(&"who");
    if who_first {
        words.next();
    }
    const VALUES: [&str; 22] = [
        "who", "who...", "other", "where", "d", "text", "item", "name", "n", "side", "team", "rank", "a", "b", "what", "flag", "value", "track", "species",
        "kind", "dx", "v",
    ];
    let head = words.take_while(|word| !VALUES.contains(word) && !word.contains(['|', '[', ':', '.'])).collect();
    (who_first, head)
}

/// A line of a word of the old vocabulary (`fadein 30`, `turn pikachu
/// left`): refused, with the form of the plan that replaces it, and a fix
/// when the line can be written so word for word (its first words put for
/// the new ones, who first when the new form is).
pub(crate) fn told_replaced(film: &mut crate::film::Film, number: usize, written: &[(String, language::Span)], verb: &str) -> bool {
    let Some(now) = replaced(verb) else { return false };
    let Some((first, first_at)) = written.first() else { return false };
    if crate::words::plain(first) != verb {
        // (Who first, `pikachu turn left`: the word is second.)
        if written.get(1).is_none_or(|(second, _)| crate::words::plain(second) != verb) {
            return false;
        }
    }
    let before = written.iter().map(|(said, _)| said.as_str()).collect::<Vec<_>>().join(" ");
    if now.is_empty() {
        let message = format!("“{verb}” is no word of the language any more: write the line another way (film/FILMS.md)");
        film.problems.push(engine::Diagnostic::at(number, message).on(Some((first_at.start, first_at.end))).coded(crate::codes::OLD_WORD));
        return true;
    }
    let (who_first, head) = literal_head(now);
    let word_first = crate::words::plain(first) == verb;
    let rest: Vec<&str> = written.iter().skip(1).map(|(said, _)| said.as_str()).collect();
    // (The one who acts: second when the old line was written word first.)
    let (who, after): (Option<&str>, &[&str]) = match (who_first, word_first, rest.split_first()) {
        (true, true, Some((who, after))) if film.actor(who).is_some() => (Some(who), after),
        (true, false, _) => (Some(first.as_str()), &rest[1..]),
        (false, true, _) => (None, &rest[..]),
        _ => (None, &[]),
    };
    // (Word for word only: no line said, `who think: text`; the form's values given, if it has any.)
    // (Its other values as many as the old line's other words, at most; those not optional, at least.)
    let tail: Vec<&str> = now.split_whitespace().skip(head.len() + usize::from(who_first)).collect();
    let (mut required, mut inside) = (0, 0);
    for value in &tail {
        if inside == 0 && !value.starts_with('[') {
            required += 1;
        }
        inside += value.matches('[').count();
        inside -= value.matches(']').count().min(inside);
    }
    let fits = after.len() <= tail.len() && after.len() >= required;
    let fixed = (!head.is_empty() && (!who_first || who.is_some()) && !now.contains(':') && fits)
        .then(|| who.into_iter().chain(head.iter().copied()).chain(after.iter().copied()).collect::<Vec<_>>().join(" "));
    let span = written.last().map_or(*first_at, |(_, last)| language::Span::new(first_at.start, last.end));
    let fixed = fixed.filter(|fixed| *fixed != before);
    let message = match &fixed {
        Some(fixed) => format!("“{verb}” is no word of the language any more: “{now}” says it now; write “{fixed}”"),
        None => format!("“{verb}” is no word of the language any more: “{now}” says it now"),
    };
    let mut problem = engine::Diagnostic::at(number, message).on(Some((span.start, span.end))).coded(crate::codes::OLD_WORD);
    if let Some(fixed) = fixed {
        problem = problem.fix(format!("write “{fixed}”"), span, fixed);
    }
    film.problems.push(problem);
    true
}

/// Whether the language has that word of the plan already: a word, a
/// command of the engine or a recipe of that name; a way of writing, by
/// what the language reads (`implemented`).
pub fn built(planned: &Planned, known: &dyn Fn(&str) -> bool) -> bool {
    match planned.made {
        Made::Syntax => true,
        Made::Word | Made::Recipe => known(planned.word),
    }
}

/// The words of the plan the language does not have yet, in the plan's
/// order: what the family waves are to build.
pub fn pending(known: &dyn Fn(&str) -> bool) -> Vec<&'static Planned> {
    WORDS.iter().filter(|planned| !built(planned, known)).collect()
}

/// A word of an optional family said in a film that does not take its
/// library in (`bite` with no `uses fishing`): told, with the line that
/// takes it in as its fix (written first in the film).
pub(crate) fn asks_its_library(film: &mut crate::film::Film, number: usize, verb: &str) {
    let Some(library) = planned(verb).and_then(|planned| library_of(planned.family)) else { return };
    if film.uses.iter().any(|used| used == library) {
        return;
    }
    let family = planned(verb).map_or("", |planned| planned.family);
    let message = format!("“{verb}” is a word of the {family} family: “uses {library}” before the first scene");
    let problem = engine::Diagnostic::at(number, message).as_remark().coded(crate::codes::USES).fix(
        format!("write “uses {library}”"),
        language::Span::new(0, 0),
        format!("uses {library}\n"),
    );
    film.problems.push(problem);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_has_each_word_once_in_a_family_shown() {
        let families: Vec<&str> = shown_families().collect();
        let made: Vec<&str> = FAMILIES.iter().copied().chain(OPTIONAL.iter().map(|(name, _)| *name)).chain([ADVANCED]).collect();
        assert_eq!(families, made);
        for (at, planned) in WORDS.iter().enumerate() {
            assert!(families.contains(&planned.family), "{}: {}", planned.word, planned.family);
            assert!(!WORDS[..at].iter().any(|known| known.word == planned.word), "“{}” twice", planned.word);
        }
        assert_eq!(who_first("walk"), Some(true));
        assert_eq!(who_first("music"), Some(false));
        assert_eq!(who_first("fight"), Some(false));
        assert_eq!(who_first("dungeon_hud"), None);
        assert_eq!(family_of("walk"), "Movement");
        assert_eq!(family_of("musicspeed"), ADVANCED);
        assert_eq!(replaced("fadein"), Some("transition fade in [d]"));
        assert_eq!(replaced("walk"), None);
        assert_eq!(replaced("rank"), None);
        assert_eq!(literal_head("transition fade in [d]"), (false, vec!["transition", "fade", "in"]));
        assert_eq!(literal_head("who face side|other|away"), (true, vec!["face"]));
        assert_eq!(literal_head("wait all"), (false, vec!["wait", "all"]));
        assert_eq!(library_of("Fishing"), Some("fishing"));
    }
}
