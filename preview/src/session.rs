//! A film being written: its text (`film::doc`), changed only by its edits;
//! played anew once it changed, all of it, as `check` plays it, a moment
//! kept every two seconds; then any moment of it at once, drawn. What it
//! tells is of the text as it was when played (`revision`): a text changed
//! since is told so, never with what was found of the one before.

use std::sync::Arc;

use engine::{Diagnostic, Seen, Watch, Who};
use shell::Screen;
use stage_view::ui::Canvas;

use crate::art::Art;
use crate::film::{Film, Snapshot};
use crate::played::{Pass, SECOND, Timeline, tick_at};
use film::doc::{Doc, Edit};
use film::tables::Tables;

/// How often a moment of the film is kept, in frames: going anywhere plays
/// at most that much.
pub const KEPT_EVERY: usize = 2 * SECOND;

/// A film being written.
pub struct Session {
    doc: Doc,
    /// What its films play with, between one and the next.
    art: Option<Art>,
    /// The game's tables its films are compiled with.
    tables: Arc<Tables>,
    played: Option<Played>,
    /// What was found of the text as it was last played; the frame its
    /// last play began at (0: its start; else a moment kept still true).
    report: Report,
    began: usize,
    /// The developer's options each film it plays is given (none: all off).
    pub debug: crate::debug::Debug,
}

/// What was found of a film as it was played.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// The text's revision it is of.
    pub revision: u64,
    /// What reading it found: wrong (it does not play), or to look at.
    pub read: Vec<Diagnostic>,
    /// What playing it found wrong, and to look at.
    pub wrong: Vec<Seen>,
    pub remarks: Vec<Seen>,
    /// When things happen in it; none if it does not play.
    pub timeline: Option<Timeline>,
    /// Who is who, by the name the film gives each one.
    pub names: Vec<(Who, String)>,
    /// What each line of the text was read as: its fields, where they are.
    pub fields: std::collections::BTreeMap<usize, film::ReadAs>,
    /// The songs the film chose itself for scenes that ask none (their
    /// backdrop's), each by the line it plays from.
    pub songs: Vec<(usize, String)>,
    /// Where the view looked as it played, every `VIEWED_EVERY` frames: the
    /// frame, the point of the place in its middle, how close it was.
    pub views: Vec<(usize, [f32; 2], f32)>,
    /// Its plan: a line each, its track, thread, placing and time
    /// (`film::time`), the timeline's spans joined to it.
    pub plan: film::time::Plan,
}

/// How often the view is kept as the film plays.
pub const VIEWED_EVERY: usize = 4;

/// A film played: where it stands, the moments kept of it.
struct Played {
    film: Film,
    /// A moment every `KEPT_EVERY` frames, from the first; what was found
    /// by then (when things happened, what was seen, how many views kept).
    kept: Vec<Snapshot>,
    found: Vec<(Timeline, Watch, usize)>,
    /// The frame it stands at.
    at: usize,
    /// Its last frame.
    end: usize,
}

impl Session {
    /// A film's text, played with that art and compiled with these tables
    /// (its project's, `film::Context::tables_for`, or read anew when places are made).
    pub fn new(text: &str, art: Art, tables: Arc<Tables>) -> Session {
        Session {
            doc: Doc::read(text),
            art: Some(art),
            tables,
            played: None,
            report: Report { revision: u64::MAX, ..Report::default() },
            began: 0,
            debug: crate::debug::Debug::default(),
        }
    }

    /// The frame its last play began at: 0, or a moment kept still true.
    pub fn began(&self) -> usize {
        self.began
    }

    /// Its text, as it is.
    pub fn doc(&self) -> &Doc {
        &self.doc
    }

    /// A change to its text, and the edit that undoes it.
    pub fn edit(&mut self, edit: &Edit) -> Result<Edit, String> {
        self.doc.apply(edit)
    }

    /// A field of a line made that, by the form the line was read by when
    /// last played (“the `x` of line 42 is 312”): the edit made, and the
    /// one that undoes it; why not, if the line has no such field said, or
    /// changed since. The film played anew tells whether it still fits.
    pub fn set_field(&mut self, number: usize, name: &str, words: &[String]) -> Result<Edit, String> {
        let read = self.report.fields.get(&number).ok_or_else(|| format!("line {number}: not read by a form when the film played"))?;
        if !self.fresh() {
            return Err(format!("line {number}: changed since the film played"));
        }
        let edit = read.edit(number, self.doc.line(number).unwrap_or_default(), name, words)?;
        self.doc.apply(&edit)
    }

    /// The fields of a line, as the form it was read by when last played
    /// gave them: each name, its words, its kind, where it is.
    pub fn fields(&self, number: usize) -> &[film::grammar::Value] {
        self.report.fields.get(&number).map_or(&[], |read| read.values.as_slice())
    }

    /// Whether what it tells is of its text as it is.
    pub fn fresh(&self) -> bool {
        self.report.revision == self.doc.revision()
    }

    /// What was found of it, as it was last played.
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// Played anew if its text changed since: read, played through and
    /// watched, a moment kept every two seconds. What was found. Played
    /// again after a change, it goes on from the last moment kept whose story
    /// so far is the same (what came before it is as it was): a change late
    /// in a long film plays only from there.
    pub fn play(&mut self) -> &Report {
        if self.fresh() {
            return &self.report;
        }
        let (art, before) = match self.played.take() {
            Some(Played { film, kept, found, .. }) => (film.parts().0, Some((kept, found, std::mem::take(&mut self.report.views)))),
            None => (self.art.take().expect("what films play with"), None),
        };
        let text = self.doc.text();
        let written = film::compile_with(&text, self.tables.clone());
        let read = crate::check::read_through(&written, &art.folder);
        let names = written.names.iter().filter_map(|(code, name)| Some((Who::parse(code)?, name.clone()))).collect();
        self.report = Report {
            revision: self.doc.revision(),
            read,
            names,
            fields: written.read_as.clone(),
            songs: written.songs.clone(),
            plan: written.plan.clone(),
            ..Report::default()
        };
        if self.report.read.iter().any(|problem| !problem.is_remark()) {
            self.art = Some(art);
            return &self.report;
        }
        let film = match Film::of_written(&written, art) {
            Ok(mut film) => {
                film.debug = self.debug.clone();
                film
            }
            Err(failed) => {
                let (art, why) = *failed;
                self.report.read.push(Diagnostic::said(why));
                self.art = Some(art);
                return &self.report;
            }
        };
        let mut pass = Pass::new(film);
        let mut seen = Watch::default();
        let mut kept = vec![pass.film.snapshot()];
        let mut found = vec![(Timeline::default(), Watch::default(), 0)];
        let mut views = Vec::new();
        // (From the last moment kept still true, with what was found by then.)
        if let Some((before, was, viewed)) = before
            && let Some(at) = (1..before.len().min(was.len())).rev().find(|at| pass.film.continues(&before[*at]))
        {
            pass.film.restore_into(&before[at]);
            pass.frame = at * KEPT_EVERY;
            let (timeline, watch, count) = was[at].clone();
            (pass.timeline, seen) = (timeline, watch);
            views = viewed[..count.min(viewed.len())].to_vec();
            kept = before[..=at].to_vec();
            found = was[..=at].to_vec();
        }
        seen.crowd = written.crowd.iter().filter_map(|code| Who::parse(code)).collect();
        self.began = pass.frame;
        while pass.step() {
            pass.film.watched(&mut seen);
            if pass.frame.is_multiple_of(VIEWED_EVERY) {
                let view = pass.film.view();
                views.push((pass.frame, [view.centre.x, view.centre.y], view.close));
            }
            if pass.frame.is_multiple_of(KEPT_EVERY) {
                kept.push(pass.film.snapshot());
                found.push((pass.timeline.clone(), seen.clone(), views.len()));
            }
        }
        let end = pass.frame;
        let mut timeline = pass.timeline;
        timeline.join(&written.plan);
        (self.report.wrong, self.report.remarks, self.report.timeline) = (seen.problems, seen.remarks, Some(timeline));
        self.report.views = views;
        self.played = Some(Played { film: pass.film, kept, found, at: end, end });
        &self.report
    }

    /// The film at that frame, as it was last played: from the moment kept
    /// before it, or from where it stands if that is nearer.
    pub fn seek(&mut self, frame: usize) -> Option<&Film> {
        let played = self.played.as_mut()?;
        let frame = frame.min(played.end);
        let kept = (frame / KEPT_EVERY).min(played.kept.len() - 1);
        if played.at > frame || played.at < kept * KEPT_EVERY {
            played.film.restore(&played.kept[kept]);
            played.at = kept * KEPT_EVERY;
        }
        while played.at < frame && !played.film.over {
            tick_at(&mut played.film, played.at);
            played.at += 1;
        }
        Some(&played.film)
    }

    /// Where the film is heard (none: not at all, as it is while gone to a
    /// moment): the one it was heard on, given back (the one given, if it does
    /// not play). Take it back before it is played anew: a new film is silent.
    pub fn hear_on(&mut self, audio: Option<Box<dyn shell::sound::Hears>>) -> Option<Box<dyn shell::sound::Hears>> {
        let Some(played) = self.played.as_mut() else {
            return audio;
        };
        played.film.hush();
        let was = std::mem::replace(&mut played.film.audio, audio);
        played.film.music_again();
        was
    }

    /// What it is heard on held where it is (paused), or going on from there.
    pub fn pause_heard(&mut self, paused: bool) {
        if let Some(played) = self.played.as_mut()
            && let Some(audio) = played.film.audio.as_mut()
        {
            audio.pause(paused);
        }
    }

    /// What its films played with, given back.
    /// What its films play with.
    pub fn art(&self) -> &Art {
        self.played.as_ref().map(|played| &played.film.art).or(self.art.as_ref()).expect("what films play with")
    }

    pub fn into_art(self) -> Art {
        self.played.map(|played| played.film.parts().0).or(self.art).expect("what films play with")
    }

    /// The film's last frame, as it was last played.
    pub fn end(&self) -> Option<usize> {
        self.played.as_ref().map(|played| played.end)
    }

    /// That frame of it drawn on the screen (at its scale): false if it does
    /// not play.
    pub fn draw(&mut self, frame: usize, canvas: &mut Canvas, screen: &mut Screen) -> bool {
        if self.seek(frame).is_none() {
            return false;
        }
        let played = self.played.as_mut().expect("played");
        played.film.draw(canvas, screen);
        true
    }

    /// The whole place at that frame, the stage as it is then; false if it
    /// does not play.
    pub fn draw_whole(&mut self, frame: usize, canvas: &mut Canvas, screen: &mut Screen) -> bool {
        if self.seek(frame).is_none() {
            return false;
        }
        let played = self.played.as_mut().expect("played");
        played.film.draw_whole(canvas, screen);
        true
    }

    /// That frame (or the whole place then) drawn with the screen's effects
    /// off: the stage as it stands under a fade or a title; false if it does
    /// not play.
    pub fn draw_staged(&mut self, frame: usize, whole: bool, canvas: &mut Canvas, screen: &mut Screen) -> bool {
        if self.seek(frame).is_none() {
            return false;
        }
        let played = self.played.as_mut().expect("played");
        played.film.draw_staged(canvas, screen, whole);
        true
    }

    /// The first frame whose scene is seen (no fade, title or other effect of
    /// the screen over it), looked for in the first `within` frames; none if
    /// it does not play or stays hidden that long.
    pub fn first_seen(&mut self, within: usize) -> Option<usize> {
        let end = self.end()?.min(within);
        (0..=end).find(|frame| self.seek(*frame).is_some_and(|film| !film.screen_hides()))
    }
}
