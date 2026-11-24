//! A command of the engine's own, as a film writes it (`quake 3 0.67s`,
//! `halo pikachu 50 orange`): each of its words typed by what its form read
//! it as (`grammar::bind`), with the film's names put in (someone by his
//! code, a crowd as each of its members, a thing by its code, a time in
//! frames), and made into its beat by the engine from those typed values
//! (`Beat::build`), never read again from text. What is wrong is told at
//! the word that is.
//!
//! A few short forms are the film's own, made into their beats here: `waitfor
//! a b`, `gone a b` (each of them), `cheer a b [emote]`, `each <a,b> <line
//! with $>`.

use engine::{Arg, Beat, EmoteId, Who};
use scenes::vocabulary::POKEMON;

use crate::film::Film;
use crate::grammar::{Bound, Kind};
use crate::words::plain;

/// One word of a command: the name its form gives it (none: a word of the form itself, `@`, `off`), what it is.
struct Said<'a> {
    name: Option<&'a str>,
    kind: Option<Kind>,
}

impl Film {
    /// One of the engine's commands: its word as written, its words, what
    /// its form read them as (`bound`; none: a line written for the engine
    /// as it is, `>`), made into its beat; what is wrong, told.
    pub fn engine_command(&mut self, verb: &str, words: &[String], bound: Option<&Bound>) {
        let said: Vec<Said> = (0..words.len()).map(|at| said_at(bound, at)).collect();
        let Some((args, from)) = self.typed(verb, words, &said) else { return };
        if self.short_form(verb, words, &args) {
            return;
        }
        let made = Beat::build(&args, &POKEMON);
        let beat = match made {
            Ok(beat) => beat,
            Err(missed) => {
                self.not_built(verb, words, &said, &from, missed);
                return;
            }
        };
        // (A number found as the film plays, where the beat holds one: `put
        // rock {_place3_x} …`; elsewhere, `set note n{i}`, as `story.txt` writes it.)
        let mut found: Vec<String> = args.iter().flat_map(found_in).collect();
        let mut held = beat.vars();
        found.sort();
        held.sort();
        if found != held {
            self.wrong(self.number, format!("“{}”: a number found as the film plays ({{…}}) where the command takes none", line_text(verb, words)));
            return;
        }
        // (A beat written back the very same; a word it does not read, the
        // engine does not either: `particles … @ x y 30 blue 3s`.)
        if beat.written(&POKEMON).is_none() {
            self.wrong(self.number, format!("“{}”: the engine reads it, but not all of it (a word too many?)", line_text(verb, words)));
            return;
        }
        self.beat(beat);
    }

    /// A beat the language made itself (`while`, `if`, `set`, `calc`, `t`…):
    /// as it is, but a test that compares someone, not a number (told).
    pub fn language_beat(&mut self, beat: Beat) {
        use engine::FlowBeat;
        if let Beat::Flow(FlowBeat::If(test) | FlowBeat::ElseIf(test) | FlowBeat::While(test) | FlowBeat::WaitUntil(test)) = &beat {
            let named = test.any.iter().flatten().flat_map(|condition| {
                let value = if let engine::Value::Var(name) = &condition.value { Some(name.as_str()) } else { None };
                std::iter::once(condition.var.as_str()).chain(value)
            });
            if let Some(someone) = named.into_iter().find(|name| self.actors.contains_key(*name) || self.groups.contains_key(*name)).map(str::to_string) {
                self.wrong(self.number, format!("“{someone}”: a test of the engine compares numbers, not someone"));
                return;
            }
        }
        self.beat(beat);
    }

    /// The command's words typed: someone of the scene by his code (a crowd
    /// as each of them), a thing by its code, a number, a time in frames,
    /// one found as the film plays (`{x}`), the words said in one; each with
    /// the word it comes from. None if one cannot be (told).
    fn typed(&mut self, verb: &str, words: &[String], said: &[Said]) -> Option<(Vec<Arg>, Vec<usize>)> {
        let host = POKEMON.host_line(verb);
        let test = matches!(verb, "waituntil" | "if" | "elif" | "while");
        let things = crate::verbs::things_in(self, verb, words);
        let spoken = said_from(verb, words);
        let mut args = vec![Arg::Word(verb.to_string())];
        let mut from = vec![usize::MAX];
        let mut at = 0;
        while at < words.len() {
            let start = at;
            let word = &words[at];
            let kind = said[at].kind;
            let before = if at == 0 { verb } else { words[at - 1].as_str() };
            let heard = matches!(before, "music" | "ambient" | "sound") && (POKEMON.song(word).is_some() || POKEMON.sound(word).is_some());
            // (Someone by his name, but a word the form says is a thing: `sound click`.)
            let named = !things.contains(word) && self.actor(word).is_some();
            let numeric = matches!(kind, Some(Kind::Number | Kind::Time | Kind::Coordinate));
            // (A thing on the stage by its own name, the last one put, in one
            // word or more: `opacity quicksand 0 20`, `jump Rare Fossil 12 16`.)
            let thing = (!named && !numeric && Some(at) != spoken)
                .then(|| {
                    (1..=3).rev().filter(|long| at + long <= words.len()).find_map(|long| {
                        let key = plain(&words[at..at + long].concat());
                        self.things.stuff.iter().rfind(|stuff| stuff.key == key).map(|stuff| (stuff.code.clone(), long))
                    })
                })
                .flatten();
            if Some(at) == spoken {
                // (The words said, one argument: the names in them put in.)
                args.push(Arg::Word(self.names_in(&words[at..].join(" "))));
                at = words.len();
            } else if let Some((code, long)) = thing {
                args.push(Who::parse(&code).map_or(Arg::Word(code), Arg::Who));
                at += long;
            } else {
                at += 1;
                // (Someone of the scene, though the form says a thing: `tremble krabby 20`.)
                let someone = named || self.actors.contains_key(word) || self.groups.contains_key(word);
                if test && someone {
                    self.wrong(self.number, format!("“{word}”: a test of the engine compares numbers, not someone (“{}”)", line_text(verb, words)));
                    return None;
                }
                match kind {
                    _ if numeric => args.push(number(word)),
                    _ if heard => args.push(Arg::Word(word.clone())),
                    _ if word.starts_with('@') && word != "@" => {
                        self.wrong(self.number, format!("“{word}”: a spot by its name is not read here; its point, `@ x y`"));
                        return None;
                    }
                    // (A line the game reads: its words its own, someone by his code.)
                    _ if host => args.push(Arg::Word(if named { self.code(word) } else { word.clone() })),
                    _ if someone => {
                        for one in self.everyone(word) {
                            let code = self.code(&one);
                            args.push(Who::parse(&code).map_or(Arg::Word(code), Arg::Who));
                        }
                    }
                    // (Someone by the name a line shows him by: `pikachu:Pika`.)
                    _ if let Some((name, shown)) = word.split_once(':')
                        && let Some(code) = self.actors.get(name).cloned() =>
                    {
                        args.push(Arg::Word(format!("{code}:{}", self.names_in(shown))));
                    }
                    Some(Kind::Someone | Kind::Code) => args.push(Who::parse(word).map_or_else(|| Arg::Word(word.clone()), Arg::Who)),
                    _ => args.push(Arg::Word(frames_said(word))),
                }
            }
            from.resize(args.len(), start);
        }
        Some((args, from))
    }

    /// A short form of the engine's words, written out as its beats: `waitfor
    /// a b`, `gone a b` (each of them), `cheer a b [emote]`, `each <a,b,c>
    /// <line with $>`. False for a line that is none.
    fn short_form(&mut self, verb: &str, words: &[String], args: &[Arg]) -> bool {
        let many: Vec<Who> = args[1..].iter().filter_map(|arg| if let Arg::Who(who) = arg { Some(*who) } else { None }).collect();
        match verb {
            "waitfor" if args.len() > 2 && many.len() == args.len() - 1 => many.into_iter().for_each(|who| self.beat(super::wait_for(who))),
            "gone" if args.len() > 2 && many.len() == args.len() - 1 => many.into_iter().for_each(|who| self.beat(super::gone(who))),
            "cheer" => self.cheered(&args[1..]),
            "each" => self.each(words),
            _ => return false,
        }
        true
    }

    /// `cheer <who...> [emote]`: each of them hops and shows its joy (or
    /// that emote), not all at once.
    fn cheered(&mut self, args: &[Arg]) {
        let shown = match args.last() {
            Some(Arg::Word(name)) => story::lowering::passes::crowd_emote(name),
            _ => None,
        };
        let crowd = &args[..args.len() - usize::from(shown.is_some())];
        let whos: Option<Vec<Who>> = crowd.iter().map(|arg| if let Arg::Who(who) = arg { Some(*who) } else { None }).collect();
        let Some(whos) = whos.filter(|whos| !whos.is_empty()) else {
            self.wrong(self.number, "“cheer” takes who (a group, or names), then maybe an emote".to_string());
            return;
        };
        let Some(id) = POKEMON.emote(shown.unwrap_or("joy")) else {
            return;
        };
        for who in whos {
            self.beat(super::after(0.0, 20.0, super::emote(who, EmoteId(id), Some(40.0))));
            self.beat(super::after(0.0, 20.0, super::jump(who, 8.0, 12.0)));
        }
    }

    /// `each <a,b,c> <line>`: the engine's command for each of them, `$`
    /// standing for it.
    fn each(&mut self, words: &[String]) {
        let (Some(list), Some(verb)) = (words.first(), words.get(1)) else {
            self.wrong(self.number, "“each” takes a list (a,b,c) then a command with $".to_string());
            return;
        };
        let items: Vec<String> = list.split(',').filter(|item| !item.is_empty()).flat_map(|item| self.everyone(item)).collect();
        for item in items {
            let rest: Vec<String> = words[2..].iter().map(|word| if word == "$" { item.clone() } else { word.clone() }).collect();
            self.engine_command(verb, &rest, None);
        }
    }

    /// A command the engine does not read so: the word that is not as its
    /// place asks, where it is written, and what is asked.
    fn not_built(&mut self, verb: &str, words: &[String], said: &[Said], from: &[usize], missed: Option<engine::Miss>) {
        let text = line_text(verb, words);
        // (The argument back to the word it comes from.)
        let word_of = |at: usize| from.get(at).copied().filter(|word| *word < words.len());
        let message = match missed.and_then(|missed| Some((missed, word_of(missed.at)?))) {
            Some((missed, at)) => {
                let name = said.get(at).and_then(|said| said.name).map_or(String::new(), |name| format!(" ({name})"));
                format!("“{}” is not {}{name}: “{text}”, see engine/docs/COMMANDS.md", words[at], missed.asked)
            }
            None => format!("“{text}”: the engine does not read it so (its form in engine/docs/COMMANDS.md)"),
        };
        self.wrong(self.number, message);
    }
}

/// What its form read the word there as.
fn said_at(bound: Option<&Bound>, at: usize) -> Said<'_> {
    let value = bound.and_then(|bound| bound.values.iter().find(|value| value.at.is_some_and(|from| (from..from + value.words.len()).contains(&at))));
    Said { name: value.map(|value| value.name.as_str()), kind: value.map(|value| value.kind) }
}

/// A number as written: its value, or one found as the film plays (`{x}`); a
/// time in frames (`1.5s`, `6f`); else the word.
fn number(word: &str) -> Arg {
    if let Some(name) = word.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')).filter(|name| !name.is_empty() && !name.contains(['{', '}', ':'])) {
        return Arg::Found(name.to_string());
    }
    let frames = frames_said(word);
    frames.parse::<f64>().map_or_else(|_| Arg::Word(word.to_string()), Arg::Number)
}

/// A time in seconds (`1.5s`) or in frames (`6f`), as frames; any other
/// word as it is.
fn frames_said(word: &str) -> String {
    match crate::words::frames_of(word) {
        Some(frames) if word.ends_with('s') || word.ends_with('f') => frames.to_string(),
        _ => word.to_string(),
    }
}

/// The names of the numbers found as the film plays an argument holds
/// (`{x}`, a spot's `w:{x}:{y}`), but in words said.
fn found_in(arg: &Arg) -> Vec<String> {
    match arg {
        Arg::Found(name) => vec![name.clone()],
        Arg::Word(word) if word.contains('{') && !word.contains("{name:") && !word.contains("{item:") => {
            word.split('{').skip(1).filter_map(|rest| rest.split_once('}')).map(|(name, _)| name.to_string()).collect()
        }
        _ => Vec::new(),
    }
}

/// The line as written, its word first.
fn line_text(verb: &str, words: &[String]) -> String {
    std::iter::once(verb).chain(words.iter().map(String::as_str)).collect::<Vec<_>>().join(" ")
}

/// Where the words said start among a command's words (after its word).
fn said_from(verb: &str, words: &[String]) -> Option<usize> {
    let at = match verb {
        "say" | "bubble" | "think" | "shout" => 2,
        "caption" | "typecaption" => 1,
        "ask" | "banner" | "hud" => 0,
        _ => return None,
    };
    let placed = verb == "hud" && words.first().is_some_and(|place| engine::hud_place(place).is_some());
    Some(if placed { at + 1 } else { at })
}

