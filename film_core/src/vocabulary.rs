//! The films' vocabulary as an author sees it: its families in their order,
//! each word in one. A line of a word said of someone is written who first
//! (`pikachu walk to piplup`), the others word first (`music track`). The
//! optional families come with `uses dungeon|guild|fishing`; the advanced
//! words are folded away.
//!
//! Each word is declared once. A word with a `Word` of its own in a group
//! (`verbs/`, a family crate) is declared on it: its forms, and on one of its
//! declarations its family and its DS status (`Word::family`,
//! `Word::ds_direct`...). What has none (a recipe of the library, a way of
//! writing of the language itself, an engine's command) is declared here,
//! in [`OTHERS`]. The vocabulary is derived from both (`Lexicon::vocabulary`):
//! the catalogue, the Wiki and the completion show each word in its family
//! (`Lexicon::family_of`), an engine's command it does not name is an
//! advanced word. A word of the old vocabulary ([`REPLACED`]) is refused,
//! its new form given as the fix (`told_replaced`). A recipe or a command of
//! [`OTHERS`] the language does not have is "to build" (`Lexicon::pending`);
//! the quality gate (`workshop_wiki/src/tests.rs`) asks the others a page
//! and an example that plays with nothing to say.

use crate::ds::OnDs;
use crate::verbs::Lexicon;

/// Every family in the order shown: the core's ([`CORE`] of them), the
/// optional ones, the advanced words last.
pub const FAMILIES: [&str; 20] = [
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

/// How many of [`FAMILIES`] are the core's: the first.
pub const CORE: usize = 16;

/// Whether that family is one of the core's.
pub fn is_core(family: &str) -> bool {
    FAMILIES[..CORE].contains(&family)
}

/// The optional families, each with the built-in library that brings it
/// into a film (`uses dungeon`).
pub const OPTIONAL: [(&str, &str); 3] = [("Dungeon", "dungeon"), ("Guild", "guild"), ("Fishing", "fishing")];

/// The words for those who want to control everything: folded by default.
pub const ADVANCED: &str = "Advanced";

/// What a word of the vocabulary is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Made {
    /// A `Word` of a group (`film_core/src/verbs/`, a family crate).
    Word,
    /// A recipe written in the language, provided (`film/recipes/`).
    Recipe,
    /// A way of writing of the language itself, no word to look up (a
    /// line said, `{value}` in it): it has no page of its own.
    Syntax,
    /// One of the engine's own commands.
    Command,
}

/// A word of the vocabulary: its family, itself, what it is, whether it is
/// written who first, how it stands on the DS.
#[derive(Clone, Copy, Debug)]
pub struct Planned {
    pub family: &'static str,
    pub word: &'static str,
    pub made: Made,
    pub who_first: bool,
    pub ds: OnDs,
}

/// A word of the vocabulary with no `Word` of its own: its family, itself,
/// its form as an author writes it, what it is, its DS status.
#[derive(Clone, Copy, Debug)]
pub struct Other {
    pub family: &'static str,
    pub word: &'static str,
    pub form: &'static str,
    pub made: Made,
    pub ds: Option<OnDs>,
}

const fn recipe(family: &'static str, word: &'static str, form: &'static str) -> Other {
    Other { family, word, form, made: Made::Recipe, ds: None }
}

const fn syntax(family: &'static str, word: &'static str, form: &'static str) -> Other {
    Other { family, word, form, made: Made::Syntax, ds: None }
}

const fn command(family: &'static str, word: &'static str, form: &'static str) -> Other {
    Other { family, word, form, made: Made::Command, ds: None }
}

impl Other {
    const fn ds_direct(self, how: &'static str) -> Other {
        Other { ds: Some(OnDs::direct(how)), ..self }
    }

    const fn ds_close(self, how: &'static str, lost: &'static str) -> Other {
        Other { ds: Some(OnDs::close(how, lost)), ..self }
    }

    const fn ds_work(self, how: &'static str) -> Other {
        Other { ds: Some(OnDs::work(how)), ..self }
    }

    const fn ds_none(self, how: &'static str) -> Other {
        Other { ds: Some(OnDs::none(how)), ..self }
    }

    const fn ds_fix(self, fix: &'static str) -> Other {
        match self.ds {
            Some(ds) => Other { ds: Some(ds.instead(fix)), ..self },
            None => self,
        }
    }

    /// Written who first (`who hug other`).
    fn who_first(&self) -> bool {
        self.form.starts_with("who ") || self.form.starts_with("*who ")
    }
}

/// What the vocabulary has with no `Word` of its own in a group: the
/// library's recipes, the ways of writing of the language itself, the
/// engine's commands; each declared here once, its family, its form as an
/// author writes it, its DS status. (A word with a `Word` is declared on it.)
pub const OTHERS: &[Other] = &[
    syntax("Structure", "if", "if test ... else ... end").ds_direct("if ( $LOCALn … ) { }: a film's variables are the scene's four local ones"),
    syntax("Speech", "(line)", "who (face): text").ds_direct("message_SetFace(ACTOR, FACE_…, FACE_POS_STANDARD) (a face the species lacks: its normal one; none: message_SetActor, its name alone), message_Talk, message_Close"),
    recipe("Speech", "explain", "who explain").ds_direct("ExecuteCommon(CORO_EXPLANATION_FUNC_SERIES), Sky's own"),
    recipe("Speech", "whisper", "who whisper other").ds_close("its words: walks over, an emote; no smaller text", "no smaller text"),
    recipe("Body", "cower", "who cower [from other]").ds_close("its words: cringe, pain looped, an emote", "an emote and its pain looped"),
    recipe("Body", "hug", "who hug other").ds_close("its words: walks beside the other, both hop", "they stand side by side and hop"),
    recipe("Body", "comfort", "who comfort other").ds_direct("its words: walks beside, nods, the other cheered"),
    recipe("Body", "cheer", "who cheer").ds_direct("its words: hops, pose"),
    recipe("Body", "shiver", "who shiver").ds_close("its words: its pain animation looped (Sky has no shiver), sweat", "no shiver: its pain animation looped"),
    recipe("Body", "stretch", "who stretch").ds_direct("its words: two poses (look up, deep breath), each the group of that species' own sprite"),
    recipe("Body", "celebrate", "who celebrate").ds_direct("its words: a spin, happy hops, its proud pose"),
    recipe("Body", "tug", "who tug other").ds_close("its words: he walks back, the other runs after him (no pull)", "the other runs after him: no one is pulled"),
    recipe("Body", "tickle", "who tickle other").ds_direct("its words: an animation, the laughing effect, hops"),
    recipe("Body", "chomp", "who chomp other").ds_direct("its words: a head animation, the other's pain, an effect"),
    recipe("Body", "high_five", "who high_five other").ds_close("its words: MoveHeight up and down together, effects", "the jumps are MoveHeight, no arc"),
    recipe("Body", "boost", "who boost other [to where]").ds_close("its words: as ride and get_off, the climber staying where he got on until he springs", "the climber stays up where he got on, then runs down to the place"),
    recipe("Body", "lift", "who lift other").ds_close("its words: as carry, the one lifted staying where he was lifted", "the one lifted doesn't follow: he stays up where he was lifted"),
    recipe("Emotions", "react", "who react joy|surprise|anger|fear|doubt|relief").ds_direct("its words: an emote, a hop"),
    recipe("Emotions", "sigh", "who sigh").ds_direct("its words: a deep breath, sweat"),
    recipe("Emotions", "laugh", "who laugh").ds_close("its words: the laughing effect, MoveHeight up and down twice", "the hops are MoveHeight, no arc"),
    recipe("Emotions", "cry", "who cry").ds_close("its words: its pain pose, its pain animation looped (Sky has no shiver), sweat", "no shiver: its pain animation looped"),
    recipe("Emotions", "sulk", "who sulk").ds_direct("its words: turned away, sweat"),
    recipe("Fights", "finish", "who finish target Move").ds_close("its words: the flash, the attack, the faint; bgm_ChangeVolume(0, 0) for the silence, then (0, 256); no slow motion (Sky's scripts have none)", "no slow motion"),
    syntax("Story", "option", "option \"text\" [if test] [once]").ds_direct("case menu(\"…\"):"),
    syntax("Story", "{if}", "{if test: text | other}").ds_direct("if ( $LOCALn … ) { } else { } round the line, one way each"),
    syntax("Story", "{value}", "{value}").ds_close("the number as last written in the script before the line", "Sky's texts print no variable: the number as last written before the line"),
    syntax("Story", "seen()", "seen(name)").ds_close("a scene's own variable, $LOCALn += 1 at the moment", "counted in the scene only"),
    syntax("Story", "#id", "who: text #id").ds_direct("nothing: an id for translators"),
    syntax("Story", "keep", "keep flag = value").ds_close("a scene's own variable ($LOCALn)", "kept in the scene only, not past it"),
    recipe("Fishing", "reel", "who reel [catch what|snap|lose]").ds_none("Sky has no fishing"),
    command(ADVANCED, "layer", "who layer front|behind").ds_work("Sky draws its actors by their height on the screen; an attribute to set"),
    recipe(ADVANCED, "ghost", "who ghost").ds_none("Sky's actors have no see-through").ds_fix("“appear … fade” or “vanish”"),
    command(ADVANCED, "tint", "tint colour").ds_work("screen_FlushOut in that colour"),
    command(ADVANCED, "float", "who float").ds_close("a routine of its own (def n for actor): forever { MoveHeight up; MoveHeight down }, until it lands (MoveHeight 0)", "the hover stops, landing, when it is told anything else"),
    command(ADVANCED, "zoom", "zoom n").ds_none("Sky's camera has no zoom").ds_fix("frame them with “frame” or “glide”"),
    command(ADVANCED, "light", "light ambience").ds_work("screen_FlushOut in the ambience's colour"),
    command(ADVANCED, "spotlight", "spotlight who").ds_none("Sky has no light on a part of the screen"),
    command(ADVANCED, "letterbox", "letterbox").ds_none("Sky's scenes have no black bars"),
    command(ADVANCED, "slowmo", "slowmo").ds_none("Sky's scripts have no slow motion").ds_fix("“wait” between the beats"),
    command(ADVANCED, "spin", "who spin").ds_work("Turn2Direction round, a turn after another"),
    command(ADVANCED, "flip", "who flip").ds_none("Sky's sprites are not turned over").ds_fix("“face” or “fall”"),
    command(ADVANCED, "timer", "timer name d").ds_work("a Wait counted"),
    syntax(ADVANCED, "every", "every d ... end").ds_work("a routine looped beside the scene"),
    syntax(ADVANCED, "while", "while test ... end").ds_work("while ( $LOCALn … ) { }"),
    command(ADVANCED, "random", "random a b").ds_close("switch ( random(n) ) as Sky draws its numbers, a case a tenth of the range setting $LOCALn", "drawn in tenths of the range: within a tenth of it"),
];

/// The words of the old vocabulary, each with the form a word of the vocabulary
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

/// The library a film takes in to have the words of that optional family
/// (`uses dungeon`), if it is one.
pub fn library_of(family: &str) -> Option<&'static str> {
    OPTIONAL.iter().find(|(name, _)| *name == family).map(|(_, library)| *library)
}

/// The vocabulary of a lexicon's words.
impl Lexicon {
    /// Every word of the vocabulary, in the order of [`FAMILIES`]: the words
    /// of the groups that say their family (`Word::family`), then
    /// [`OTHERS`]; read once.
    pub fn vocabulary(&'static self) -> &'static [Planned] {
        self.vocabulary.get_or_init(|| {
            let mut out: Vec<Planned> = Vec::new();
            let declared = self.groups_of_words().flat_map(|group| group.words.iter());
            for word in declared.filter(|word| word.family.is_some()) {
                // (Who first by its forms, those of every group that reads it.)
                let forms =
                    self.groups_of_words().flat_map(|group| group.words.iter()).filter(|known| known.word == word.word).flat_map(|known| known.forms.iter());
                let who_first = !word.word_first && forms.into_iter().any(|form| crate::order::performer_first(&crate::grammar::read(form)));
                let ds = word.ds.unwrap_or(OnDs::none(""));
                out.push(Planned { family: word.family.unwrap_or(ADVANCED), word: word.word, made: Made::Word, who_first, ds });
            }
            for other in OTHERS {
                let ds = other.ds.unwrap_or(OnDs::none(""));
                out.push(Planned { family: other.family, word: other.word, made: other.made, who_first: other.who_first(), ds });
            }
            out.sort_by_key(|planned| FAMILIES.iter().position(|family| *family == planned.family).unwrap_or(FAMILIES.len()));
            out
        })
    }

    /// The word of the vocabulary of that name, if one.
    pub fn planned(&'static self, word: &str) -> Option<&'static Planned> {
        self.vocabulary().iter().find(|planned| planned.word == word)
    }

    /// Whether the vocabulary writes that word who first (`who walk to
    /// where`): none if it does not have it.
    pub fn who_first(&'static self, word: &str) -> Option<bool> {
        self.planned(word).map(|planned| planned.who_first)
    }

    /// The family a word is shown in: its own, else [`ADVANCED`] (an
    /// engine's command).
    pub fn family_of(&'static self, word: &str) -> &'static str {
        self.planned(word).map_or(ADVANCED, |planned| planned.family)
    }

    /// The form a word of the old vocabulary is written in now (`""`:
    /// dropped), if it is one; none for a word of the vocabulary.
    pub fn replaced(&'static self, word: &str) -> Option<&'static str> {
        if self.planned(word).is_some() {
            return None;
        }
        REPLACED.iter().find(|(old, _)| *old == word).map(|(_, now)| *now)
    }

    /// The recipes and commands of [`OTHERS`] the language does not have
    /// (`known`: a word, a command or a recipe of that name), in order.
    pub fn pending(&'static self, known: &dyn Fn(&str) -> bool) -> Vec<&'static Planned> {
        self.vocabulary().iter().filter(|planned| matches!(planned.made, Made::Recipe | Made::Command) && !known(planned.word)).collect()
    }

    /// A form as an author writes it: who first if its first part is who
    /// acts and the word is not written word first (`order::shown_form`).
    pub fn shown_form(&'static self, word: &str, form: &str) -> String {
        crate::order::shown_form(word, form, self.who_first(word) != Some(false))
    }
}

/// The words a form of the vocabulary starts with before its first value:
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
/// left`): refused, with the form of the vocabulary that replaces it, and a fix
/// when the line can be written so word for word (its first words put for
/// the new ones, who first when the new form is).
pub(crate) fn told_replaced(film: &mut crate::film::Film, number: usize, written: &[(String, language::Span)], verb: &str) -> bool {
    let Some(now) = film.lexicon.replaced(verb) else { return false };
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

/// A word of an optional family said in a film that does not take its
/// library in (`bite` with no `uses fishing`): told, with the line that
/// takes it in as its fix (written first in the film).
pub(crate) fn asks_its_library(film: &mut crate::film::Film, number: usize, verb: &str) {
    let Some(library) = film.lexicon.planned(verb).and_then(|planned| library_of(planned.family)) else { return };
    if film.uses.iter().any(|used| used == library) {
        return;
    }
    let family = film.lexicon.planned(verb).map_or("", |planned| planned.family);
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
    fn the_families_and_the_others_are_each_once() {
        for (at, family) in FAMILIES.iter().enumerate() {
            assert!(!FAMILIES[..at].contains(family), "{family} twice");
        }
        for (name, _) in OPTIONAL {
            assert!(FAMILIES[CORE..].contains(&name) && !is_core(name), "{name}");
        }
        for (at, other) in OTHERS.iter().enumerate() {
            assert!(FAMILIES.contains(&other.family), "{}: {}", other.word, other.family);
            assert!(!OTHERS[..at].iter().any(|known| known.word == other.word), "“{}” twice", other.word);
            assert!(other.ds.is_some(), "{}: no DS status", other.word);
        }
        assert_eq!(literal_head("transition fade in [d]"), (false, vec!["transition", "fade", "in"]));
        assert_eq!(literal_head("who face side|other|away"), (true, vec!["face"]));
        assert_eq!(literal_head("wait all"), (false, vec!["wait", "all"]));
        assert_eq!(library_of("Fishing"), Some("fishing"));
    }
}
