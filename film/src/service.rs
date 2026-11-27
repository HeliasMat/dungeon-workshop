//! The language's service: what an editor or an agent asks of a film's
//! text, read once by the language. What is wrong, with where and its
//! fixes as changes of the text; its plan; what folds; where a name is
//! given and used; renaming one; what a word is; what may be written here.
//!
//! Lines and columns from 1, a column counting letters.

use language::ast::{Arg, ArgKind, Block, Call, Expr, ExprKind, File, Kind, Opening, PartKind, Piece, Stmt};
use language::{Source, Span};

use film_core::lines::holds;
use film_core::text::front::internal;

/// A place in the text (lines and columns from 1).
pub use language::Position as Spot;
/// What is wrong, where (its lines and columns worked out), its fixes as
/// edits of the text.
pub use language::{Diagnostic, Edit, Fix, Location, Severity};

/// A piece of the text, its end past its last letter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub start: Spot,
    pub end: Spot,
}

impl From<Range> for Location {
    fn from(range: Range) -> Location {
        Location::between(range.start, range.end)
    }
}

/// A thing of the plan: a scene, a moment, a recipe, a value, a mark…
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub kind: &'static str,
    pub name: String,
    pub detail: String,
    pub line: usize,
    pub end_line: usize,
}

fn range(source: &Source, span: Span) -> Range {
    let start = source.position(span.start);
    let end = source.position(span.end);
    Range { start, end }
}

fn line_range(source: &Source, line: usize) -> Range {
    let long = source.line_text(line).chars().count();
    let lead = source.line_text(line).chars().take_while(|letter| letter.is_whitespace()).count();
    Range { start: Spot { line, column: lead + 1 }, end: Spot { line, column: long + 1 } }
}

fn parsed(text: &str) -> File {
    language::parse_with(text, &holds).file
}

/// A fix of one edit, writing that name there.
fn writing(name: String, at: Range) -> Fix {
    Fix { title: format!("write “{name}”"), edits: vec![Edit { span: at.into(), text: name }] }
}

/// What is wrong with a film: what the language says (its column, its
/// fixes), then what the words say once played through (its line; the
/// nearest names as fixes; [`film_core::codes::WORDS`] for those with no code
/// of their own). Each with its lines and columns, its bytes too when said.
pub fn diagnostics(text: &str, tables: std::sync::Arc<film_core::tables::Tables>) -> Vec<Diagnostic> {
    let source = Source::new(text);
    let mut told: Vec<Diagnostic> =
        internal(&crate::LEXICON, text, 0, film_core::tables::ENGLISH).diagnostics.into_iter().map(|(_, wrong)| wrong.resolved(&source)).collect();
    let said: Vec<(usize, String)> = told.iter().map(|one| (one.span.start.line, one.message.clone())).collect();
    for mut problem in crate::compile_with(text, tables).problems {
        let line = problem.line();
        if line == 0 || problem.span.file.is_some() || said.iter().any(|(at, message)| *at == line && problem.message.starts_with(message.as_str())) {
            continue;
        }
        // (The very words it is about, where the film says them; else, for a
        // name not known, that name found on its line; else the line.)
        let mut at = match problem.span.bytes {
            Some(bytes) => range(&source, bytes),
            None => line_range(&source, line),
        };
        let mut fixes = Vec::new();
        if let Some((word, near)) = film_core::check::offered(&problem.message) {
            let found = || source.line_text(line).find(&word).map(|byte| source.line_text(line)[..byte].chars().count() + 1);
            if problem.span.bytes.is_none()
                && let Some(column) = found()
            {
                at = Range { start: Spot { line, column }, end: Spot { line, column: column + word.chars().count() } };
            }
            if problem.span.bytes.is_some() || found().is_some() {
                fixes = near.into_iter().map(|name| writing(name, at)).collect();
            }
        }
        let bytes = problem.span.bytes;
        problem.span = Location { bytes, ..at.into() };
        problem.code = Some(problem.code_or(film_core::codes::WORDS));
        if !fixes.is_empty() {
            problem.fixes = fixes;
        }
        told.push(problem);
    }
    told.sort_by_key(|one| (one.span.start, !one.is_error()));
    told
}

/// The text with those edits made (they must not overlap), by their bytes,
/// else their lines and columns.
pub fn applied(text: &str, edits: &[Edit]) -> String {
    language::diag::applied(text, edits)
}

/// Each line said of the film's scenes that has no id yet given one, written
/// last (`pikachu: Hi! #k3f9`): its own name, whatever its words become, for
/// those who translate it (its `| fr:` with it). The ids there are kept as
/// they are; a new one is made of the line's words, never one already in
/// the film. The text, and how many were given.
pub fn with_line_ids(text: &str) -> (String, usize) {
    fn said<'a>(stmts: &'a [Stmt], into: &mut Vec<&'a language::ast::Said>) {
        for stmt in stmts {
            match &stmt.kind {
                Kind::Said(line) if line.who.text != "transition" => into.push(line),
                Kind::Block(block) => {
                    said(&block.body, into);
                    for part in &block.parts {
                        said(&part.body, into);
                    }
                }
                _ => {}
            }
        }
    }
    let file = parsed(text);
    let mut lines = Vec::new();
    for scene in &file.scenes {
        said(&scene.body, &mut lines);
    }
    let mut taken: std::collections::HashSet<String> = lines.iter().filter_map(|line| line.id.as_ref().map(|id| id.text.clone())).collect();
    let mut edits = Vec::new();
    for line in lines.iter().filter(|line| line.id.is_none() && !line.text.trim().is_empty()) {
        let mut seed = format!("{}:{}", line.who.text, line.text);
        let id = loop {
            let made = line_id_of(&seed);
            if taken.insert(made.clone()) {
                break made;
            }
            seed.push('+');
        };
        let at = line.text_span.end;
        edits.push(Edit { span: Location::bytes(Span::new(at, at)), text: format!(" #{id}") });
    }
    (applied(text, &edits), edits.len())
}

/// An id made of words: a letter, then three of letters and digits.
fn line_id_of(words: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in words.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    const BOTH: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut id = String::from(LETTERS[(hash % 26) as usize] as char);
    hash /= 26;
    for _ in 0..3 {
        id.push(BOTH[(hash % 36) as usize] as char);
        hash /= 36;
    }
    id
}

/// The plan of a film: its scenes and their moments, its recipes, its
/// values, its marks, its threads.
pub fn outline(text: &str) -> Vec<Symbol> {
    let source = Source::new(text);
    let file = parsed(text);
    let line_of = |span: Span| source.position(span.start).line;
    let mut found = Vec::new();
    let last = source.lines();
    let mut head = Vec::new();
    collect(&file.head, &source, &mut head);
    found.extend(head);
    for (at, scene) in file.scenes.iter().enumerate() {
        let mut end = file.scenes.get(at + 1).map_or(last, |next| line_of(next.span) - 1);
        // (Its blank lines at its end are not its.)
        while end > line_of(scene.span) && source.line_text(end).trim().is_empty() {
            end -= 1;
        }
        let words: Vec<&str> = scene.args.iter().map(|arg| arg.text.as_str()).collect();
        found.push(Symbol {
            kind: "scene",
            name: words.first().copied().unwrap_or("").to_string(),
            detail: words.get(1..).unwrap_or(&[]).join(" "),
            line: line_of(scene.span),
            end_line: end,
        });
        let moments: Vec<(usize, String)> = scene
            .body
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                Kind::Moment(name) => Some((line_of(stmt.span), name.clone())),
                _ => None,
            })
            .collect();
        for (rank, (line, name)) in moments.iter().enumerate() {
            let mut until = moments.get(rank + 1).map_or(end, |(next, _)| next - 1);
            while until > *line && source.line_text(until).trim().is_empty() {
                until -= 1;
            }
            found.push(Symbol { kind: "moment", name: name.clone(), detail: String::new(), line: *line, end_line: until });
        }
        let mut inner = Vec::new();
        collect(&scene.body, &source, &mut inner);
        found.extend(inner);
    }
    found.sort_by_key(|symbol| symbol.line);
    found
}

fn collect(stmts: &[Stmt], source: &Source, into: &mut Vec<Symbol>) {
    let line_of = |span: Span| source.position(span.start).line;
    let end_of = |span: Span| source.position(span.end.saturating_sub(1).max(span.start)).line;
    for stmt in stmts {
        match &stmt.kind {
            Kind::Let { kept, name, value } => into.push(Symbol {
                kind: if *kept { "kept" } else { "value" },
                name: name.text.clone(),
                detail: language::format::expression(value),
                line: line_of(stmt.span),
                end_line: line_of(stmt.span),
            }),
            Kind::Call(call) if call.word.text == "mark" => {
                if let Some(name) = call.args.first() {
                    into.push(Symbol { kind: "mark", name: name.text.clone(), detail: String::new(), line: line_of(stmt.span), end_line: line_of(stmt.span) });
                }
            }
            Kind::Block(block) => {
                let (kind, name, detail) = match &block.opening {
                    Opening::Recipe(recipe) => ("recipe", recipe.form.word.text.clone(), language::format::form_text(&recipe.form)),
                    Opening::Beside | Opening::When(_) | Opening::Every(_) | Opening::After(_) | Opening::At { .. } => (
                        "beside",
                        block.named.as_ref().map_or_else(|| block.opening.word().to_string(), |name| name.text.clone()),
                        language::format::opening_text(block),
                    ),
                    _ => ("block", block.opening.word().to_string(), language::format::opening_text(block)),
                };
                into.push(Symbol { kind, name, detail, line: line_of(stmt.span), end_line: end_of(stmt.span) });
                collect(&block.body, source, into);
                for part in &block.parts {
                    collect(&part.body, source, into);
                }
            }
            _ => {}
        }
    }
}

/// What folds: each scene up to the next, each moment, each block of more
/// than its line (lines from 1, both included).
pub fn folds(text: &str) -> Vec<(usize, usize)> {
    let mut found: Vec<(usize, usize)> = outline(text)
        .into_iter()
        .filter(|symbol| matches!(symbol.kind, "scene" | "moment" | "recipe" | "beside" | "block") && symbol.end_line > symbol.line)
        .map(|symbol| (symbol.line, symbol.end_line))
        .collect();
    found.sort();
    found.dedup();
    found
}

/// What a name written somewhere is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// A word called (a verb, a recipe).
    Word,
    /// A recipe given.
    Recipe,
    /// A value given (`let`, `keep`, `for each x`).
    Gives,
    /// A value read or set.
    Value,
    /// A mark set (`mark name`), a thread named (`& as name`).
    Names,
    /// A mark or a thread waited for or stopped.
    Named,
}

/// Every name the language reads in a file, with its span and its role.
fn names(file: &File) -> Vec<(Span, String, Role)> {
    let mut found = Vec::new();
    stmts(&file.head, &mut found);
    for scene in &file.scenes {
        stmts(&scene.body, &mut found);
    }
    found
}

/// The names a line said reads: its values, its tests and the words they choose.
fn pieces(said: &[Piece], into: &mut Vec<(Span, String, Role)>) {
    for piece in said {
        match piece {
            Piece::Value(value) => expr(value, into),
            Piece::Choose { test, then, otherwise } => {
                expr(test, into);
                pieces(then, into);
                pieces(otherwise, into);
            }
            _ => {}
        }
    }
}

fn stmts(stmts_in: &[Stmt], into: &mut Vec<(Span, String, Role)>) {
    for stmt in stmts_in {
        if let Some(named) = stmt.beside.as_ref().and_then(|beside| beside.named.as_ref()) {
            into.push((named.span, named.text.clone(), Role::Names));
        }
        match &stmt.kind {
            Kind::Said(said) => pieces(&said.pieces, into),
            Kind::Call(call) => self_call(call, into),
            Kind::Let { name, value, .. } => {
                into.push((name.span, name.text.clone(), Role::Gives));
                expr(value, into);
            }
            Kind::Set { name, value } => {
                into.push((name.span, name.text.clone(), Role::Value));
                expr(value, into);
            }
            Kind::Return(Some(value)) => expr(value, into),
            Kind::Block(block) => self_block(block, into),
            _ => {}
        }
    }
}

fn self_call(call: &Call, into: &mut Vec<(Span, String, Role)>) {
    into.push((call.word.span, call.word.text.clone(), Role::Word));
    let marks = matches!(call.word.text.as_str(), "mark");
    let waits = matches!(call.word.text.as_str(), "wait" | "cease");
    for arg in &call.args {
        args(arg, into);
        if matches!(arg.kind, ArgKind::Word) && (marks || waits) {
            into.push((arg.span, arg.text.clone(), if marks { Role::Names } else { Role::Named }));
        }
    }
}

fn args(arg: &Arg, into: &mut Vec<(Span, String, Role)>) {
    if let ArgKind::Value(value) = &arg.kind {
        expr(value, into);
    }
}

fn self_block(block: &Block, into: &mut Vec<(Span, String, Role)>) {
    if let Some(named) = &block.named {
        into.push((named.span, named.text.clone(), Role::Names));
    }
    match &block.opening {
        Opening::If(value) | Opening::Repeat(value) | Opening::While(value) | Opening::Every(value) | Opening::After(value) => expr(value, into),
        Opening::ForEach { name, rank, list } => {
            into.push((name.span, name.text.clone(), Role::Gives));
            if let Some(rank) = rank {
                into.push((rank.span, rank.text.clone(), Role::Gives));
            }
            expr(list, into);
        }
        Opening::When(language::ast::Event::Becomes(value)) => expr(value, into),
        Opening::At { mark, offset, clock } => {
            if *clock == language::ast::Clock::Mark {
                into.push((mark.span, mark.text.clone(), Role::Named));
            }
            if let Some((_, by)) = offset {
                expr(by, into);
            }
        }
        Opening::Recipe(recipe) => into.push((recipe.form.word.span, recipe.form.word.text.clone(), Role::Recipe)),
        Opening::Word(call) => self_call(call, into),
        Opening::Fight(found) => {
            for arg in found {
                args(arg, into);
            }
        }
        _ => {}
    }
    stmts(&block.body, into);
    for part in &block.parts {
        match &part.kind {
            PartKind::ElseIf(value) | PartKind::Option { test: Some(value), .. } => expr(value, into),
            _ => {}
        }
        stmts(&part.body, into);
    }
}

fn expr(value: &Expr, into: &mut Vec<(Span, String, Role)>) {
    match &value.kind {
        ExprKind::Name(name) => into.push((value.span, name.clone(), Role::Value)),
        ExprKind::List(items) => items.iter().for_each(|item| expr(item, into)),
        ExprKind::Point(x, y) | ExprKind::Binary(_, x, y) | ExprKind::Index(x, y) => {
            expr(x, into);
            expr(y, into);
        }
        ExprKind::Unary(_, inner) | ExprKind::Field(inner, _) => expr(inner, into),
        ExprKind::Call(called, given) => {
            into.push((called.span, called.text.clone(), Role::Word));
            given.iter().for_each(|item| expr(item, into));
        }
        _ => {}
    }
}

/// The name at that place, its span and its role.
fn name_at(text: &str, at: Spot) -> Option<(Span, String, Role)> {
    let source = Source::new(text);
    let byte = source.byte(language::Position { line: at.line, column: at.column });
    names(&parsed(text)).into_iter().find(|(span, ..)| span.start <= byte && byte <= span.end)
}

/// Where the name at that place is given: its line in this film, or the
/// file it is in (a recipe shared or of the library) and its line there.
pub fn definition(text: &str, at: Spot, project: &assets::project::Project) -> Option<(Option<String>, usize)> {
    let (_, name, role) = name_at(text, at)?;
    let source = Source::new(text);
    let file = parsed(text);
    let given = names(&file).into_iter().find(|(_, other, other_role)| {
        *other == name
            && match role {
                Role::Word | Role::Recipe => *other_role == Role::Recipe,
                Role::Value | Role::Gives => *other_role == Role::Gives,
                Role::Named | Role::Names => *other_role == Role::Names,
            }
    });
    if let Some((span, ..)) = given {
        return Some((None, source.position(span.start).line));
    }
    if matches!(role, Role::Word) {
        let shared = crate::recipes_for(project, text).into_iter().find(|recipe| recipe.name == name)?;
        return Some((Some(shared.from), shared.line));
    }
    None
}

/// Every place the name at that place is written, as the same thing.
pub fn references(text: &str, at: Spot) -> Vec<Range> {
    let Some((_, name, role)) = name_at(text, at) else { return Vec::new() };
    let source = Source::new(text);
    let alike = |other: Role| match role {
        Role::Word | Role::Recipe => matches!(other, Role::Word | Role::Recipe),
        Role::Value | Role::Gives => matches!(other, Role::Value | Role::Gives),
        Role::Named | Role::Names => matches!(other, Role::Named | Role::Names),
    };
    names(&parsed(text)).into_iter().filter(|(_, other, other_role)| *other == name && alike(*other_role)).map(|(span, ..)| range(&source, span)).collect()
}

/// The text with the name at that place written `new` wherever it is the
/// same thing: a value, a recipe, a mark or a thread.
pub fn renamed(text: &str, at: Spot, new: &str) -> String {
    let edits: Vec<Edit> = references(text, at).into_iter().map(|range| Edit { span: range.into(), text: new.to_string() }).collect();
    applied(text, &edits)
}

/// The calculations' own words, each with what it gives.
const FUNCTIONS: [(&str, &str); 11] = [
    ("seen", "seen(name): how many times that moment or scene was played"),
    ("div", "div(a, b): the whole division (7 and 2: 3); a / b is decimal (3.5)"),
    ("chance", "chance(n): true n times in a hundred"),
    ("random", "random(a, b): a number from a to b (durations too)"),
    ("min", "min(a, b): the smaller"),
    ("max", "max(a, b): the bigger"),
    ("abs", "abs(a): without its sign"),
    ("distance", "distance(a, b): how many pixels between them"),
    ("health", "health(a): his HP in percent (in a fight: when health(a) <= 50)"),
    ("clock", "clock: the frames since the start of the scene"),
    ("music_clock", "music_clock: the frames of the music played since its line “music” (from 8s included, at its speed)"),
];

/// What the name at that place is: a word of the language (its forms, what
/// it does), a recipe, a value given.
pub fn hover(text: &str, at: Spot, project: &assets::project::Project) -> Option<String> {
    // (Who first, `piplup walk to right`: on its second word, the word's help;
    // on who, what its form wants there.)
    if let Some((word, on_the_word)) = who_first_at(text, at) {
        return match on_the_word {
            true => word_help(&word, text, project),
            false => match wanted_of(&word, &[]).into_iter().next()? {
                Wanted::Kind(name, kind) => Some(format!("{name}: {}", kind.said())),
                Wanted::Choice(words) => Some(format!("one of: {}", words.join(", "))),
            },
        };
    }
    let Some((_, name, role)) = name_at(text, at) else {
        // (A value of a word's line: what its form wants there.)
        return match wanted_at(text, at).into_iter().next()? {
            Wanted::Kind(name, kind) => Some(format!("{name}: {}", kind.said())),
            Wanted::Choice(words) => Some(format!("one of: {}", words.join(", "))),
        };
    };
    if matches!(role, Role::Value | Role::Gives)
        && let Some(symbol) = outline(text).into_iter().find(|symbol| matches!(symbol.kind, "value" | "kept") && symbol.name == name)
    {
        return Some(format!("{} {} = {} (line {})", if symbol.kind == "kept" { "keep" } else { "let" }, name, symbol.detail, symbol.line));
    }
    if let Some((_, told)) = FUNCTIONS.iter().find(|(word, _)| *word == name) {
        return Some(told.to_string());
    }
    word_help(&name, text, project)
}

/// What a word of the language, a recipe or an engine's command is: its
/// forms as written, what it does.
fn word_help(name: &str, text: &str, project: &assets::project::Project) -> Option<String> {
    if let Some(recipe) = crate::recipes_for(project, text).into_iter().find(|recipe| recipe.name == name) {
        let from = if recipe.from.is_empty() { format!("line {}", recipe.line) } else { recipe.from.clone() };
        let uses = if recipe.not_used { format!("\nA word of your library: write “uses {}” at the top of the film.", recipe.from) } else { String::new() };
        return Some(format!("{} — {} ({from}){uses}", recipe.shape, recipe.doc));
    }
    let word = film_core::words::plain(name);
    let shown = |known: &str, forms: &[&str], writes: &str, family: &str| {
        let forms: Vec<String> = forms.iter().map(|form| crate::shown_form(known, form)).collect();
        format!("{} — {writes} ({family})", forms.join(" · "))
    };
    if let Some((family, known, forms, writes)) = crate::LEXICON.every_word().find(|(_, known, ..)| *known == word) {
        return Some(shown(known, forms, writes, family));
    }
    crate::LEXICON.command_words().find(|(known, ..)| *known == word).map(|(known, forms, writes)| shown(known, forms, writes, "engine"))
}

/// A recipe offered: its form, what it does, the file of the library it
/// is in (and the `uses` line it needs).
fn with_doc(shape: String, recipe: &film_core::Told) -> String {
    let doc = if recipe.doc.is_empty() { String::new() } else { format!(" — {}", recipe.doc) };
    let uses = if recipe.not_used { format!(" (your library: uses {})", recipe.from) } else { String::new() };
    format!("{shape}{doc}{uses}")
}

/// What a word's line wants at one of its places, by the word's first form.
enum Wanted {
    /// A value of that kind, by its name in the form.
    Kind(String, film_core::grammar::Kind),
    /// One of these very words.
    Choice(Vec<String>),
}

/// The line of the tree a byte stands in, when it is a word's: its call.
fn call_at(stmts: &[Stmt], byte: usize) -> Option<&Call> {
    for stmt in stmts.iter().filter(|stmt| stmt.span.start <= byte && byte <= stmt.span.end) {
        match &stmt.kind {
            Kind::Call(call) => return Some(call),
            Kind::Block(block) => {
                let inner = std::iter::once(&block.body).chain(block.parts.iter().map(|part| &part.body));
                if let Some(call) = inner.into_iter().find_map(|body| call_at(body, byte)) {
                    return Some(call);
                }
            }
            _ => {}
        }
    }
    None
}

/// What the word of the line at that place wants there: its call read on
/// the tree, the place counted by its arguments before; by each of its
/// forms whose very words fit those written before it, the piece that falls
/// there (a team: all that is left).
fn wanted_at(text: &str, at: Spot) -> Vec<Wanted> {
    let source = Source::new(text);
    let byte = source.byte(language::Position { line: at.line, column: at.column });
    let file = parsed(text);
    // (After the line's last space the call is behind the cursor: found back
    // over the spaces.)
    let mut back = byte.min(text.len());
    while back > 0 && matches!(text.as_bytes()[back - 1], b' ' | b'\t') {
        back -= 1;
    }
    let Some(call) = bodies(&file).find_map(|body| call_at(body, byte).or_else(|| call_at(body, back))) else {
        return Vec::new();
    };
    if byte <= call.word.span.end {
        return Vec::new();
    }
    // (The arguments before the one under the cursor, or the next one.)
    let mut before: Vec<String> = call.args.iter().filter(|arg| arg.span.end < byte).map(|arg| film_core::words::plain(&arg.text)).collect();
    let mut word = film_core::words::plain(&call.word.text);
    // (Who first, `pikachu walk `: the word is the second, who its first.)
    if crate::LEXICON.subject_first(&call.word.text, call.args.first().map(|arg| arg.text.as_str()), text) && !before.is_empty() {
        let verb = before.remove(0);
        before.insert(0, std::mem::replace(&mut word, verb));
    }
    wanted_of(&word, &before)
}

/// The bodies of a film's tree: its head's, each scene's.
fn bodies(file: &File) -> impl Iterator<Item = &Vec<Stmt>> {
    std::iter::once(&file.head).chain(file.scenes.iter().map(|scene| &scene.body))
}

/// On a line written who first (`piplup walk to right`), the cursor on its
/// first two words: the line's word, and whether on it (else on who).
fn who_first_at(text: &str, at: Spot) -> Option<(String, bool)> {
    let byte = Source::new(text).byte(language::Position { line: at.line, column: at.column });
    let file = parsed(text);
    let call = bodies(&file).find_map(|body| call_at(body, byte))?;
    let second = call.args.first()?;
    if !crate::LEXICON.subject_first(&call.word.text, Some(&second.text), text) {
        return None;
    }
    let within = |span: Span| span.start <= byte && byte <= span.end;
    match (within(call.word.span), within(second.span)) {
        (_, true) => Some((film_core::words::plain(&second.text), true)),
        (true, false) => Some((film_core::words::plain(&second.text), false)),
        _ => None,
    }
}

/// What a word wants after these words of its line, by each of its forms
/// whose very words fit them (a team: all that is left).
fn wanted_of(word: &str, before: &[String]) -> Vec<Wanted> {
    use film_core::grammar::Piece as FormPiece;
    // (Every word of that name: several families may have one, `walk` a
    // formation's and a walk's.)
    let forms: Vec<&str> = crate::LEXICON.every_word().filter(|(_, known, ..)| *known == word).flat_map(|(_, _, forms, _)| forms.iter().copied()).collect();
    if forms.is_empty() {
        return Vec::new();
    }
    fn flat(pieces: Vec<FormPiece>, into: &mut Vec<FormPiece>) {
        for piece in pieces {
            match piece {
                FormPiece::Maybe(inner) => flat(inner, into),
                other => into.push(other),
            }
        }
    }
    let mut wanted = Vec::new();
    for form in forms.iter() {
        let mut pieces = Vec::new();
        flat(film_core::grammar::read(form), &mut pieces);
        let fits = before.iter().zip(&pieces).all(|(said, piece)| !matches!(piece, FormPiece::Choice(words) if !words.contains(said)));
        if !fits {
            continue;
        }
        let piece = pieces.get(before.len()).or(pieces.last().filter(|piece| matches!(piece, FormPiece::Team(_))));
        wanted.extend(match piece.cloned() {
            Some(FormPiece::One { name, kind, .. }) => Some(Wanted::Kind(name, kind)),
            Some(FormPiece::Team(name)) => Some(Wanted::Kind(name, film_core::grammar::Kind::Someone)),
            Some(FormPiece::Choice(words)) => Some(Wanted::Choice(words)),
            _ => None,
        });
    }
    wanted
}

/// Those the film's scenes name (`scene: beach pikachu piplup`), once.
fn named_in_scenes(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for symbol in outline(text).into_iter().filter(|symbol| symbol.kind == "scene") {
        for name in symbol.detail.split_whitespace() {
            if !names.iter().any(|known| known == name) {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// What may be written where a word wants a value of that kind.
fn offered_for(text: &str, wanted: Wanted) -> Vec<(String, String)> {
    use film_core::grammar::Kind as Of;
    let (name, kind) = match wanted {
        Wanted::Choice(words) => return words.into_iter().map(|word| (word, "one of the words of the form".to_string())).collect(),
        Wanted::Kind(name, kind) => (name, kind),
    };
    let told = format!("{name}: {}", kind.said());
    let offered: Vec<String> = match kind {
        Of::Someone | Of::Code => named_in_scenes(text),
        Of::Place => {
            let mut places: Vec<String> = ["front_of", "behind", "left", "right", "center"].iter().map(|word| word.to_string()).collect();
            places.extend(named_in_scenes(text));
            places
        }
        Of::Time => ["0.5s", "1s", "2s", "3s"].iter().map(|word| word.to_string()).collect(),
        Of::Number | Of::Coordinate => outline(text).into_iter().filter(|symbol| matches!(symbol.kind, "value" | "kept")).map(|symbol| symbol.name).collect(),
        Of::Property => engine::props::PROPS.iter().filter(|prop| prop.animatable).map(|prop| prop.name.to_string()).collect(),
        Of::Word | Of::Thing => Vec::new(),
    };
    offered.into_iter().map(|word| (word, told.clone())).collect()
}

/// The block words the language opens a line with.
const OPENINGS: [(&str, &str); 16] = [
    ("if", "if test … else if … else … end"),
    ("repeat", "repeat n … end"),
    ("for each", "for each x in [a, b] … end"),
    ("while", "while test … end"),
    ("together", "all starts at the same time … end"),
    ("beside", "beside the film … end"),
    ("when", "when a arrives : line, when test : line"),
    ("every", "every 2s : line"),
    ("after", "after 2s : line"),
    ("choice", "choice / option … / end"),
    ("fight directed", "fight directed a against b … end"),
    ("define", "define name who:pokemon … end"),
    ("let", "let n = value"),
    ("keep", "keep n = value (kept in the save)"),
    ("return", "return value (at the end of a recipe)"),
    ("moment:", "moment: its name"),
];

/// Who a line starts with, when the word after him is being written
/// (`pikachu `, `pikachu wa`): a first word no word of the language.
fn who_then_word<'a>(before: &'a str, text: &str) -> Option<&'a str> {
    let words: Vec<&str> = before.split_whitespace().collect();
    let typing_second = match words.len() {
        1 => before.ends_with(char::is_whitespace),
        2 => !before.ends_with(char::is_whitespace),
        _ => false,
    };
    let who = words.first().copied()?;
    let name = who.chars().all(|letter| letter.is_alphanumeric() || letter == '_');
    (typing_second && name && !crate::LEXICON.is_word(who, text) && !OPENINGS.iter().any(|(word, _)| word.trim_end_matches(':') == who)).then_some(who)
}

/// What may be written at that place: at a line's start, the words, the
/// recipes and the blocks; elsewhere, the values given and the
/// calculations' words. Each with what it is.
pub fn completions(text: &str, at: Spot, project: &assets::project::Project) -> Vec<(String, String)> {
    let source = Source::new(text);
    let line = source.line_text(at.line);
    let before: String = line.chars().take(at.column.saturating_sub(1)).collect();
    let first = before.trim().is_empty() || (before.split_whitespace().count() == 1 && !before.ends_with(char::is_whitespace));
    let in_a_value = before.matches('{').count() > before.matches('}').count()
        || ["if ", "while ", "let ", "keep ", "return ", "repeat ", "when "].iter().any(|word| before.trim_start().starts_with(word));
    let typed: String = before.chars().rev().take_while(|letter| letter.is_alphanumeric() || *letter == '_').collect::<String>().chars().rev().collect();
    let mut offered: Vec<(String, String)> = Vec::new();
    if in_a_value {
        for symbol in outline(text).into_iter().filter(|symbol| matches!(symbol.kind, "value" | "kept")) {
            offered.push((symbol.name, format!("{} = {}", symbol.kind, symbol.detail)));
        }
        offered.extend(FUNCTIONS.iter().map(|(word, told)| (word.to_string(), told.to_string())));
    } else if !first && !wanted_at(text, at).is_empty() {
        // (What the word wants there, by its forms: someone, a place, a time…)
        for wanted in wanted_at(text, at) {
            offered.extend(offered_for(text, wanted));
        }
    } else if let Some(who) = who_then_word(&before, text) {
        // (Who first, `pikachu `: the words said of someone, as he would do them.)
        for recipe in crate::recipes_for(project, text).into_iter().filter(|recipe| recipe.of_someone || crate::LEXICON.takes_someone_first(&recipe.name, text)) {
            offered.push((recipe.name.clone(), with_doc(format!("{who} {}", recipe.shape), &recipe)));
        }
        for (word, forms) in
            crate::LEXICON.every_word().map(|(_, word, forms, _)| (word, forms)).chain(crate::LEXICON.shown_commands().map(|(word, forms, _)| (word, forms)))
        {
            if let Some(form) = forms.iter().find(|form| crate::performer_first(&film_core::grammar::read(form))) {
                let shown = crate::shown_form(word, form);
                offered.push((word.to_string(), format!("{who} {}", shown.split_once(' ').map_or("", |(_, rest)| rest))));
            }
        }
    } else if first {
        offered.extend(OPENINGS.iter().map(|(word, told)| (word.to_string(), told.to_string())));
        for recipe in crate::recipes_for(project, text) {
            offered.push((recipe.name.clone(), with_doc(recipe.shape.clone(), &recipe)));
        }
        for (_, word, forms, writes) in crate::LEXICON.every_word() {
            offered.push((word.to_string(), forms.first().map_or_else(|| writes.to_string(), |form| crate::shown_form(word, form))));
        }
        // (The advanced words, the engine's commands, folded: offered once three of their letters are typed.)
        if typed.chars().count() >= 3 {
            for (word, forms, writes) in crate::LEXICON.shown_commands() {
                offered.push((word.to_string(), forms.first().map_or_else(|| writes.to_string(), |form| crate::shown_form(word, form))));
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    offered.retain(|(word, _)| word.starts_with(&typed) && seen.insert(word.clone()));
    offered
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILM: &str = "define double x:number -> number\n  return x * 2\nend\n\nscene: beach pikachu piplup\nmoment: the bet\nlet n = double(4)\nif n > 2\n  pikachu: {n}!\nend\npiplup walk to right & as escape\nwait escape\n";

    #[test]
    fn what_is_wrong_is_said_with_its_column_and_its_fix() {
        let told = diagnostics("scene: beach pikachu\nif n >= \nend\nif pikachu = 1\npikachu jump\nendif\n", crate::tests::tables());
        let first = told.iter().find(|one| one.code == Some("E0005")).expect("the calculation");
        assert_eq!(first.span.start, Spot { line: 2, column: 8 });
        let old = told.iter().find(|one| one.code == Some("E0008")).expect("endif");
        assert_eq!(applied("scene: beach pikachu\nif n >= \nend\nif pikachu = 1\npikachu jump\nendif\n", &old.fixes[0].edits).lines().last(), Some("end"));
        let near = diagnostics("scene: beach pikachu\njum pikachu\n", crate::tests::tables());
        assert!(near.iter().any(|one| one.fixes.iter().any(|fix| fix.edits[0].text == "jump")), "{near:?}");
    }

    /// What a word says is wrong is told on the words it is about, where the
    /// film writes them (not the whole line), even a word written twice.
    #[test]
    fn a_word_s_problem_is_told_on_its_words() {
        let film = "scene: beach pikachu\n  pikachu walk to pikachuu\n";
        let told = diagnostics(film, crate::tests::tables());
        let wrong = told.iter().find(|one| one.code == Some("film") && one.message.contains("pikachuu")).expect("the place not known");
        assert_eq!((wrong.span.start, wrong.span.end), (Spot { line: 2, column: 19 }, Spot { line: 2, column: 27 }), "{told:?}");
        // (The film says it itself, by the bytes of its text.)
        let problem = crate::tests::compile(film).problems.into_iter().find(|problem| problem.message.contains("pikachuu")).expect("told");
        assert_eq!(problem.span.bytes, Some(language::Span::new(39, 47)));
        // (Each value a line gives a word, where it is written, who first.)
        let read = crate::tests::compile("scene: beach pikachu\npikachu walk to right slow\n").read_as.remove(&2).expect("the line read");
        assert_eq!(read.word, "walk");
        let span_of = |name: &str| read.values.iter().find(|value| value.name == name).and_then(|value| value.span);
        assert_eq!(span_of("who"), Some(language::Span::new(21, 28)), "{:?}", read.values);
        assert_eq!(span_of("where"), Some(language::Span::new(37, 42)), "{:?}", read.values);
        // (Its fields written again in place, who first.)
        let edit = read.edit(2, "pikachu walk to right slow", "who", &["piplup".to_string()]).expect("who");
        let mut doc = film_core::doc::Doc::read("scene: beach pikachu\npikachu walk to right slow\n");
        doc.apply(&edit).expect("applied");
        assert_eq!(doc.line(2), Some("piplup walk to right slow"));
        let edit = read.edit(2, "piplup walk to right slow", "where", &["front_of".to_string(), "eevee".to_string()]).expect("where");
        doc.apply(&edit).expect("applied");
        assert_eq!(doc.line(2), Some("piplup walk to front_of eevee slow"));
    }

    /// Who first, `piplup walk to right`: the word's help on its second
    /// word, what the form wants on who.
    #[test]
    fn a_line_written_who_first_is_helped_on_its_word() {
        let walk = hover(FILM, Spot { line: 11, column: 9 }, &crate::tests::project());
        assert!(walk.as_deref().is_some_and(|told| told.starts_with("who:pokemon walk")), "{walk:?}");
        assert!(hover(FILM, Spot { line: 11, column: 2 }, &crate::tests::project()).is_some_and(|told| told.starts_with("who")));
        // (After who, someone of the scene is no word; after the word, its form.)
        let after = completions("scene: beach pikachu eevee\neevee walk to ", Spot { line: 2, column: 15 }, &crate::tests::project());
        assert!(after.iter().any(|(word, _)| word == "pikachu"), "{after:?}");
    }

    #[test]
    fn the_plan_folds_and_names() {
        let plan = outline(FILM);
        let kinds: Vec<(&str, &str)> = plan.iter().map(|symbol| (symbol.kind, symbol.name.as_str())).collect();
        assert!(
            kinds.contains(&("recipe", "double"))
                && kinds.contains(&("scene", "beach"))
                && kinds.contains(&("moment", "the bet"))
                && kinds.contains(&("value", "n")),
            "{kinds:?}"
        );
        assert!(folds(FILM).contains(&(1, 3)));
        // (`n` read in the said line: given on line 7.)
        assert_eq!(definition(FILM, Spot { line: 9, column: 13 }, &crate::tests::project()), Some((None, 7)));
        assert_eq!(definition(FILM, Spot { line: 7, column: 9 }, &crate::tests::project()), Some((None, 1)), "the recipe");
        assert_eq!(references(FILM, Spot { line: 12, column: 10 }).len(), 2, "the thread, named then waited for");
        let renamed = renamed(FILM, Spot { line: 7, column: 5 }, "catches");
        assert!(renamed.contains("let catches = double(4)") && renamed.contains("if catches > 2") && renamed.contains("{catches}!"), "{renamed}");
        assert!(completions(FILM, Spot { line: 8, column: 5 }, &crate::tests::project()).iter().any(|(word, _)| word == "n"));
        assert!(hover(FILM, Spot { line: 11, column: 17 }, &crate::tests::project()).is_some());
        assert!(completions("scene: beach a\ntoget", Spot { line: 2, column: 6 }, &crate::tests::project()).iter().any(|(word, _)| word == "together"));
        // (After the line's last space, what its word wants next: of all the
        // words of that name, `walk` a formation's and a walk's.)
        // (Who first: the same.)
        let next = completions(
            "scene: beach pikachu eevee
eevee walk ",
            Spot { line: 2, column: 12 },
            &crate::tests::project(),
        );
        assert!(next.iter().any(|(word, _)| word == "to"), "{next:?}");
        // (Who written first: what he can do, his forms who first; never a word where nobody acts.)
        let next = completions("scene: beach pikachu eevee\neevee wa", Spot { line: 2, column: 9 }, &crate::tests::project());
        let walk = next.iter().find(|(word, _)| word == "walk").expect("walk");
        assert!(walk.1.starts_with("eevee walk"), "{walk:?}");
        let next = completions("scene: beach pikachu eevee\neevee ", Spot { line: 2, column: 7 }, &crate::tests::project());
        assert!(next.iter().any(|(word, _)| word == "jump") && !next.iter().any(|(word, _)| word == "music"), "{next:?}");
    }
}
