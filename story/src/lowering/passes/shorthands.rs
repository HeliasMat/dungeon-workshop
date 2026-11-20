//! What a file writes short, written out: each shorthand gives the lines
//! the engine reads.

use super::Lines;
use super::names::page_face;
use engine::Diagnostic;
use engine::Vocabulary;

/// The number `oneof` draws its line by.
pub(super) const ONEOF: &str = "_oneof";

/// How many times `repeat` plays what it holds at most.
const MOST: usize = 1000;

/// Words as fields.
fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

/// A `switch` open: its line, its number's name, whether a `case` has
/// opened its `if`.
struct Switch {
    line: usize,
    name: String,
    opened: bool,
}

/// The lines written out so far.
struct Writer<'a> {
    out: Lines,
    problems: &'a mut Vec<Diagnostic>,
    vocabulary: &'a Vocabulary,
    switches: Vec<Switch>,
    /// A table of ways on is open: `when` lines one after another.
    ways_open: bool,
}

/// All of a file's lines written out.
pub(super) fn shorthands(lines: Lines, problems: &mut Vec<Diagnostic>, vocabulary: &Vocabulary) -> Lines {
    let mut writer = Writer { out: Vec::with_capacity(lines.len()), problems, vocabulary, switches: Vec::new(), ways_open: false };
    let mut lines = lines.into_iter();
    while let Some((number, fields)) = lines.next() {
        writer.line(number, fields, &mut lines);
    }
    writer.finish()
}

impl Writer<'_> {
    fn push(&mut self, number: usize, fields: Vec<String>) {
        self.out.push((number, fields));
    }

    fn wrong(&mut self, number: usize, message: &str) {
        self.problems.push(Diagnostic::at(number, message.to_string()));
    }

    /// What was left open told, closed.
    fn finish(mut self) -> Lines {
        if self.ways_open {
            self.out.push((0, words(&["endif"])));
        }
        for Switch { line, name, .. } in std::mem::take(&mut self.switches) {
            self.problems.push(Diagnostic::at(line, format!("“switch {name}” without “endswitch”")));
        }
        self.out
    }

    fn line(&mut self, number: usize, fields: Vec<String>, rest: &mut impl Iterator<Item = (usize, Vec<String>)>) {
        if self.ways_open && !matches!(fields[0].as_str(), "when" | "otherwise") {
            self.push(number, words(&["endif"]));
            self.ways_open = false;
        }
        match fields[0].as_str() {
            "when" if fields.len() >= 3 => self.way(number, &fields),
            // (`otherwise` in a `switch` of no ways on: its `default`.)
            "otherwise" if !self.ways_open && !self.switches.is_empty() => self.default(number),
            "otherwise" => self.other_way(number, fields.get(1)),
            "once" => self.once(number, fields.get(1)),
            "endonce" => self.push(number, words(&["endif"])),
            "list" => self.list(number, fields.get(1), fields.get(2)),
            "cheer" | "boo" => self.crowd(number, &fields),
            "bolt" => self.lightning(number, &fields),
            "switch" => self.switch(number, fields.get(1)),
            "case" => self.case(number, fields.get(1)),
            "default" => self.default(number),
            "endswitch" => self.end_switch(number),
            "repeat" => self.repeat(number, &fields, rest),
            "endrepeat" => self.wrong(number, "“endrepeat” without “repeat” before"),
            "waitfor" | "gone" if fields.len() > 2 => self.out.extend(fields[1..].iter().map(|who| (number, vec![fields[0].clone(), who.clone()]))),
            "each" => self.each(number, &fields),
            "line" => self.said(number, &fields),
            "question" => self.question(number, &fields),
            "narrate" => self.narrate(number, &fields),
            "chat" => self.chat(number, &fields),
            "oneof" => self.one_of(number, &fields),
            _ => self.push(number, fields),
        }
    }

    /// `when <test> <scene>` lines, then maybe `otherwise <scene>`: on to the
    /// scene of the first whose test holds.
    fn way(&mut self, number: usize, fields: &[String]) {
        let test = &fields[1..fields.len() - 1];
        self.push(number, [vec![if self.ways_open { "elif" } else { "if" }.to_string()], test.to_vec()].concat());
        self.push(number, words(&["gotoscene", &fields[fields.len() - 1]]));
        self.ways_open = true;
    }

    fn other_way(&mut self, number: usize, scene: Option<&String>) {
        match (self.ways_open, scene) {
            (true, Some(scene)) => {
                self.push(number, words(&["else"]));
                self.push(number, words(&["gotoscene", scene]));
                self.push(number, words(&["endif"]));
                self.ways_open = false;
            }
            _ => self.wrong(number, "“otherwise” takes a scene, after “when” lines"),
        }
    }

    /// `once <name>` ... `endonce`: played the first time only; the number
    /// of that name remembers it, kept by the story.
    fn once(&mut self, number: usize, name: Option<&String>) {
        match name {
            Some(name) => {
                self.push(number, words(&["if", name, "==", "0"]));
                self.push(number, words(&["set", name, "1"]));
            }
            None => self.wrong(number, "“once” takes a name (the number that remembers it)"),
        }
    }

    /// `list <name> <a,b,c>`: the numbers name1, name2, name3, and how
    /// many, name_count; read by their rank: `{name{i}}`.
    fn list(&mut self, number: usize, name: Option<&String>, values: Option<&String>) {
        let (Some(name), Some(values)) = (name, values) else {
            return self.wrong(number, "“list” takes a name and values (a,b,c)");
        };
        let values: Vec<&str> = values.split(',').map(str::trim).filter(|value| !value.is_empty()).collect();
        for (at, value) in values.iter().enumerate() {
            self.push(number, words(&["set", &format!("{name}{}", at + 1), value]));
        }
        self.push(number, words(&["set", &format!("{name}_count"), &values.len().to_string()]));
    }

    /// `cheer <who...> [emote]`: each of them hops and shows its joy (or
    /// that emote), not all at once; `boo`: anger, without hopping.
    fn crowd(&mut self, number: usize, fields: &[String]) {
        let booed = fields[0] == "boo";
        let shown = crowd_emote(fields.last().map_or("", String::as_str));
        let crowd = &fields[1..fields.len() - usize::from(shown.is_some())];
        if crowd.is_empty() {
            self.problems.push(Diagnostic::at(number, format!("“{}” takes who (a group, or names), then maybe an emote", fields[0])));
            return;
        }
        let emote = shown.unwrap_or(if booed { "anger" } else { "joy" });
        for who in crowd {
            self.push(number, words(&["after", "0~20", "emote", who, emote, "40"]));
            if !booed {
                self.push(number, words(&["after", "0~20", "jump", who, "8", "12"]));
            }
        }
    }

    /// `bolt <who|@ x y>`: the bolt, its flash, its thunder a moment after.
    fn lightning(&mut self, number: usize, fields: &[String]) {
        self.push(number, [vec!["lightning".to_string()], fields[1..].to_vec()].concat());
        self.push(number, words(&["flash", "white", "8"]));
        self.push(number, words(&["after", "20", "sound", "flash"]));
    }

    /// `switch <name>`, `case <value>[,<value>...]`, `default`, `endswitch`:
    /// an `if` and its `elif`s on that number.
    fn switch(&mut self, number: usize, name: Option<&String>) {
        match name {
            Some(name) => self.switches.push(Switch { line: number, name: name.clone(), opened: false }),
            None => self.wrong(number, "“switch” takes the name of a number"),
        }
    }

    fn case(&mut self, number: usize, values: Option<&String>) {
        let Some(switch) = self.switches.last_mut() else {
            return self.wrong(number, "“case” outside a “switch”");
        };
        let Some(values) = values else {
            return self.wrong(number, "“case” takes a value (or several: 1,2)");
        };
        let mut test = words(&[if switch.opened { "elif" } else { "if" }]);
        for (at, value) in values.split(',').filter(|value| !value.is_empty()).enumerate() {
            if at > 0 {
                test.push("or".to_string());
            }
            test.extend(words(&[&switch.name, "==", value]));
        }
        switch.opened = true;
        self.push(number, test);
    }

    fn default(&mut self, number: usize) {
        match self.switches.last().map(|switch| switch.opened) {
            Some(true) => self.push(number, words(&["else"])),
            // (No case before it: played whatever the number.)
            Some(false) => {}
            None => self.wrong(number, "“default” outside a “switch”"),
        }
    }

    fn end_switch(&mut self, number: usize) {
        match self.switches.pop().map(|switch| switch.opened) {
            Some(true) => self.push(number, words(&["endif"])),
            Some(false) => {}
            None => self.wrong(number, "“endswitch” without “switch” before"),
        }
    }

    /// `repeat <n>` ... `endrepeat`: the lines between, that many times.
    fn repeat(&mut self, number: usize, fields: &[String], rest: &mut impl Iterator<Item = (usize, Vec<String>)>) {
        let times = fields.get(1).and_then(|times| times.parse::<usize>().ok()).unwrap_or(0).min(MOST);
        // (Up to its own `endrepeat`, others inside it included.)
        let (mut body, mut depth, mut closed) = (Vec::new(), 0, false);
        for (inner, fields) in rest {
            match fields[0].as_str() {
                "repeat" => depth += 1,
                "endrepeat" if depth == 0 => {
                    closed = true;
                    break;
                }
                "endrepeat" => depth -= 1,
                _ => {}
            }
            body.push((inner, fields));
        }
        if !closed {
            self.wrong(number, "“repeat” without “endrepeat”");
        }
        let body = shorthands(body, self.problems, self.vocabulary);
        for _ in 0..times {
            self.out.extend(body.iter().cloned());
        }
    }

    /// `each <a,b,c> <line>`: the line for each of them, `$` standing for it.
    fn each(&mut self, number: usize, fields: &[String]) {
        let (Some(list), Some(_)) = (fields.get(1), fields.get(2)) else {
            return self.wrong(number, "“each” takes a list (a,b,c) then a command with $");
        };
        for item in list.split(',').filter(|item| !item.is_empty()) {
            let line = fields[2..].iter().map(|field| if field == "$" { item.to_string() } else { field.clone() }).collect();
            self.out.extend(shorthands(vec![(number, line)], self.problems, self.vocabulary));
        }
    }

    /// `line <who> <face> <text>`: a line said, then waited for; `||` in
    /// its words, several boxes one after another. A page may change the
    /// face (`||[angry]...`), kept by the pages after it.
    fn said(&mut self, number: usize, fields: &[String]) {
        let pages: Vec<String> = fields.get(3).map_or(vec![String::new()], |text| text.split("||").map(str::to_string).collect());
        let mut face = fields.get(2).cloned();
        for page in pages {
            let mut said = fields[1..].to_vec();
            let (shown, page) = page_face(&page, self.vocabulary);
            face = shown.or(face);
            if let (Some(slot), Some(face)) = (said.get_mut(1), &face) {
                *slot = face.clone();
            }
            if let Some(slot) = said.get_mut(2) {
                *slot = page;
            }
            self.push(number, [vec!["say".to_string()], said].concat());
            self.push(number, words(&["said"]));
        }
    }

    /// `question <who> <face> <words> <a|b|c>`: the line, not waited for,
    /// and the question asked under it.
    fn question(&mut self, number: usize, fields: &[String]) {
        self.push(number, [vec!["say".to_string()], fields[1..fields.len().min(4)].to_vec()].concat());
        self.push(number, vec!["ask".to_string(), fields.get(4).cloned().unwrap_or_default()]);
    }

    /// `narrate <text>`: a narration, centred, then waited for.
    fn narrate(&mut self, number: usize, fields: &[String]) {
        let text = fields.get(1).cloned().unwrap_or_default();
        let centred = format!("[CN]{}", text.trim_start().replace("\\n", "\\n[CN]"));
        self.push(number, words(&["say", "-", "-", &centred]));
        self.push(number, words(&["said"]));
    }

    /// `chat <who> <words> <who> <words>...`: bubbles one after the other,
    /// each long enough to be read.
    fn chat(&mut self, number: usize, fields: &[String]) {
        if fields.len() < 3 || fields.len().is_multiple_of(2) {
            return self.wrong(number, "“chat” takes <who> <words> <who> <words>...");
        }
        for pair in fields[1..].chunks(2) {
            self.push(number, vec!["bubble".to_string(), pair[0].clone(), "-".to_string(), pair[1].clone()]);
            self.push(number, words(&["waitbubbles"]));
        }
    }

    /// `oneof <who> <face> <a|b|c>`: one of the lines, by chance.
    fn one_of(&mut self, number: usize, fields: &[String]) {
        let (Some(who), Some(face), Some(lines)) = (fields.get(1), fields.get(2), fields.get(3)) else {
            return self.wrong(number, "“oneof” takes <who> <face> <line|line|...>");
        };
        let lines: Vec<&str> = lines.split('|').collect();
        let mut chosen = vec![(number, words(&["random", ONEOF, &lines.len().to_string()]))];
        for (at, said) in lines.iter().enumerate() {
            chosen.push((number, words(&["if", ONEOF, "==", &at.to_string()])));
            chosen.push((number, words(&["line", who, face, said])));
            chosen.push((number, words(&["endif"])));
        }
        self.out.extend(shorthands(chosen, self.problems, self.vocabulary));
    }
}

/// The emote a crowd shows by its name, as the engine names it (see
/// `cheer`).
fn crowd_emote(name: &str) -> Option<&'static str> {
    Some(match name {
        "joy" | "sparkle" => "joy",
        "anger" => "anger",
        "sweat" | "tears" => "sweat",
        "surprise" | "exclaim" | "!" => "exclaim",
        "question" | "?" => "question",
        "shock" => "shock",
        "laugh" => "laugh",
        _ => return None,
    })
}
