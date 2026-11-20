//! A line's time changed by hand (the Workshop's mouse): its placing, or how
//! long it lasts, written back into its line, found where the language
//! reads it. The one way the tools change time in a text.

use language::ast::{Kind, Opening, Stmt};
use language::{Source, Span};

use super::{Clock, Placement};
use crate::doc::{Doc, Edit};

/// What is changed of a line's time.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// Its placing: after, with `&`, at a clock, at an event.
    Placement(Placement),
    /// How long it lasts, in frames: its `in 2s`, its `wait 2s`, the
    /// time it says.
    Length(i32),
}

/// A time as a film writes it: seconds, to a hundredth (`1.5s`).
pub fn seconds_said(frames: i32) -> String {
    let seconds = (f64::from(frames) / 60.0 * 100.0).round() / 100.0;
    format!("{}s", language::line::number_text(seconds))
}

/// The edit that gives that line of the text that time: the line written
/// anew, its indent and its comment kept.
pub fn replace(doc: &Doc, number: usize, change: &Change) -> Result<Edit, String> {
    Ok(Edit::Line { number, text: replaced_line(&doc.text(), number, change)? })
}

/// The statement that starts on that line, however deep in blocks.
fn on_line<'a>(stmts: Vec<&'a Stmt>, source: &Source, number: usize) -> Option<&'a Stmt> {
    for stmt in stmts {
        if source.position(stmt.span.start).line == number {
            return Some(stmt);
        }
        if let Kind::Block(block) = &stmt.kind {
            let inner: Vec<&Stmt> = block.body.iter().chain(block.parts.iter().flat_map(|part| part.body.iter())).collect();
            if let Some(found) = on_line(inner, source, number) {
                return Some(found);
            }
        }
    }
    None
}

/// A line placed as asked: its words (without its placing), then the way
/// that placing is written.
fn placed(plain: &str, placement: &Placement) -> String {
    match placement {
        Placement::After => plain.to_string(),
        Placement::With => format!("{plain} &"),
        Placement::At { clock, offset } => {
            let time = seconds_said(*offset);
            match clock {
                Clock::Scene => format!("at {time} : {plain}"),
                Clock::Music => format!("at music {time} : {plain}"),
                Clock::Mark(mark) => format!("at {mark} + {time} : {plain}"),
                Clock::Here => format!("at +{time} : {plain}"),
            }
        }
        Placement::When(event) => format!("when {event} : {plain}"),
    }
}

/// That line of the text, its time changed.
pub fn replaced_line(text: &str, number: usize, change: &Change) -> Result<String, String> {
    let source = Source::new(text);
    let parsed = language::parse(text);
    let stmts: Vec<&Stmt> = parsed.file.head.iter().chain(parsed.file.scenes.iter().flat_map(|scene| scene.body.iter())).collect();
    let stmt = on_line(stmts, &source, number).ok_or(format!("line {number}: not a line of the film"))?;
    let line = source.line_text(number);
    let indent = &line[..line.len() - line.trim_start().len()];
    // (Its comment at the end, kept.)
    let comment =
        stmt.comment.as_ref().and_then(|comment| line.rfind('#').filter(|at| line[at + 1..].trim() == comment.trim()).map(|at| line[at..].to_string()));
    let uncommented = |said: &str| match &comment {
        Some(comment) => said.trim().strip_suffix(comment.as_str()).unwrap_or(said).trim().to_string(),
        None => said.trim().to_string(),
    };
    let slice = |span: Span| uncommented(&text[span.start..span.end.min(text.len())]);
    let without_beside = |stmt: &Stmt| match &stmt.beside {
        Some(beside) => text[stmt.span.start..beside.span.start].trim().to_string(),
        None => slice(stmt.span),
    };
    let anchored = |opening: &Opening| matches!(opening, Opening::At { .. } | Opening::When(_) | Opening::After(_));
    let (plain, head) = match &stmt.kind {
        Kind::Block(block) if block.short && anchored(&block.opening) => (block.body.first().map(without_beside).unwrap_or_default(), false),
        Kind::Block(block) if anchored(&block.opening) => (String::new(), true),
        Kind::Block(_) => return Err(format!("line {number}: a block, not a line")),
        _ => (without_beside(stmt), false),
    };
    let made = match change {
        // (A block beside the film: its opening placed anew, its lines kept.)
        Change::Placement(placement) if head => placed("", placement)
            .trim_end()
            .strip_suffix(" :")
            .map(str::to_string)
            .ok_or("a block beside the film is placed at a time or at an event (at 12s, at +2s, when a arrives)".to_string())?,
        Change::Placement(placement) => placed(&plain, placement),
        Change::Length(_) if head => return Err(format!("line {number}: the opening of a block has no duration")),
        Change::Length(frames) => {
            let lasting = lasting(&plain, *frames).ok_or(format!("line {number}: \"{plain}\" writes no duration to change"))?;
            slice(stmt.span).replacen(&plain, &lasting, 1)
        }
    };
    let made = match &comment {
        Some(comment) => format!("{made} {comment}"),
        None => made,
    };
    Ok(format!("{indent}{}", made.trim_end()))
}

/// What a line plays, without its placing (`at 3s : `, `when a arrives : `,
/// `&`): read alone, as in a scene; the line as it is if it is no line
/// placed (a block's opening).
pub fn played_words(line: &str) -> String {
    let text = format!(
        "scene: a
{}
",
        line.trim()
    );
    replaced_line(&text, 2, &Change::Placement(Placement::After)).map_or_else(|_| line.trim().to_string(), |plain| plain.trim().to_string())
}

/// A line's words with its time made so long: its `in 2s`, a `wait 2s`,
/// or the first time it says; none if it says no time.
fn lasting(plain: &str, frames: i32) -> Option<String> {
    let mut words: Vec<String> = plain.split(' ').map(str::to_string).collect();
    let time = |word: &str| crate::words::frames_of(word).is_some() && !crate::words::digits(word);
    let at = words
        .iter()
        .position(|word| word == "in")
        .map(|at| at + 1)
        .filter(|at| words.get(*at).is_some_and(|word| time(word)))
        .or_else(|| words.iter().position(|word| time(word)))?;
    words[at] = seconds_said(frames);
    Some(words.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Length;

    const FILM: &str = "scene: beach pikachu eevee\n  pikachu walk to 200 120 in 2s\n  eevee: Wait!  # she runs\n  at 3s : eevee jump\n  when pikachu arrives\n    pikachu jump\n  end\n  wait 1s\n";

    fn changed(number: usize, change: Change) -> String {
        replaced_line(FILM, number, &change).unwrap_or_else(|why| panic!("{why}"))
    }

    /// The text with that line made so.
    fn edited(number: usize, line: &str) -> String {
        FILM.lines().enumerate().map(|(at, old)| if at + 1 == number { line.to_string() } else { old.to_string() }).collect::<Vec<_>>().join("\n") + "\n"
    }

    /// The plan's line of that line of the text, as compiled (the last).
    fn planned(text: &str, number: usize) -> crate::time::PlanLine {
        let plan = crate::tests::compile(text).plan;
        plan.shown().map(|(_, line)| line.clone()).filter(|line| line.number == number).last().unwrap_or_else(|| panic!("{number} : {:?}", plan.lines))
    }

    #[test]
    fn a_line_is_placed_with_and_after_and_back() {
        let with = changed(2, Change::Placement(Placement::With));
        assert_eq!(with, "  pikachu walk to 200 120 in 2s &");
        let text = edited(2, &with);
        assert_eq!(planned(&text, 2).placement, Placement::With);
        assert_eq!(replaced_line(&text, 2, &Change::Placement(Placement::After)).as_deref(), Ok("  pikachu walk to 200 120 in 2s"));
    }

    #[test]
    fn a_line_is_placed_at_a_time_and_its_comment_kept() {
        let at = changed(3, Change::Placement(Placement::At { clock: Clock::Here, offset: 90 }));
        assert_eq!(at, "  at +1.5s : eevee: Wait!  # she runs", "a \"#\" said is said");
        let text = edited(3, &at);
        assert_eq!(planned(&text, 3).placement, Placement::At { clock: Clock::Here, offset: 90 });
        assert_eq!(replaced_line(&text, 3, &Change::Placement(Placement::After)).as_deref(), Ok("  eevee: Wait! # she runs"));
    }

    #[test]
    fn an_anchored_line_and_a_block_change_their_clock() {
        assert_eq!(changed(4, Change::Placement(Placement::When("pikachu arrives".to_string()))), "  when pikachu arrives : eevee jump");
        assert_eq!(changed(4, Change::Placement(Placement::With)), "  eevee jump &");
        assert_eq!(changed(5, Change::Placement(Placement::At { clock: Clock::Scene, offset: 120 })), "  at 2s");
        assert!(replaced_line(FILM, 5, &Change::Placement(Placement::After)).is_err());
    }

    #[test]
    fn a_comment_at_the_end_is_kept() {
        let text = "scene: beach pikachu
  pikachu jump  # up high
";
        assert_eq!(replaced_line(text, 2, &Change::Placement(Placement::With)).as_deref(), Ok("  pikachu jump & # up high"));
    }

    #[test]
    fn how_long_a_line_lasts_is_its_time_written() {
        assert_eq!(changed(2, Change::Length(150)), "  pikachu walk to 200 120 in 2.5s");
        assert_eq!(changed(8, Change::Length(30)), "  wait 0.5s");
        assert_eq!(planned(&edited(8, "  wait 0.5s"), 8).length, Length::Known(30));
        assert!(replaced_line(FILM, 3, &Change::Length(30)).is_err(), "a line said has no time written");
        let doc = Doc::read(FILM);
        assert_eq!(replace(&doc, 8, &Change::Length(90)), Ok(Edit::Line { number: 8, text: "  wait 1.5s".to_string() }));
    }

    #[test]
    fn what_a_line_plays_is_read_without_its_placing() {
        assert_eq!(played_words("  at 3s : eevee jump"), "eevee jump");
        assert_eq!(played_words("when pikachu arrives : pikachu: Ah!"), "pikachu: Ah!");
        assert_eq!(played_words("pikachu walk to right &"), "pikachu walk to right");
        assert_eq!(played_words("when pikachu arrives"), "when pikachu arrives");
    }
}
