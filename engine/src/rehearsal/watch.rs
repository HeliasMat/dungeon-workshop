//! What is seen as a film goes by ([`Watch`]): its constants, the line on
//! screen, and each frame's look.

use super::*;

/// The frames between two looks at who stands where; those someone must stay
/// still to be at rest; how near two bodies are one on the other; most told.
pub(super) const LOOK_EVERY: u32 = 10;
pub(super) const AT_REST: u32 = 30;
pub(super) const ON_TOP: f32 = 12.0;
pub(super) const MOST_TOLD: usize = 400;
/// The remarks of a kind that count, in a scene: the others are details.
pub(super) const MOST_OF_A_KIND: usize = 3;
/// The kinds that are remarks: what is seen wrong on the screen, whatever the
/// scene means. Who speaks, or is spoken to, out of sight (`view`, `absent`);
/// a body in the scenery (`wall`), two on one another (`on`); a walk that
/// goes nowhere or through the scenery (`stuck`, `through`); a line gone
/// before it is read (`fast`); someone there or gone at once, in sight, with
/// nothing to say why (`pops`); a sound waited for whose length is not
/// known (`sound`). Any other is a detail.
pub(super) const REMARKS: [&str; 9] = ["view", "absent", "wall", "on", "stuck", "through", "fast", "pops", "sound"];
/// How many at most are looked at two by two.
pub(super) const MOST_PAIRED: usize = 120;
/// The frames someone walks without moving before it is said to be stuck.
pub(super) const STUCK: u32 = 40;
/// The frames someone walks without getting nearer where it goes.
pub(super) const SHUFFLING: u32 = 90;
/// How far round the fighters the fight's field reaches.
pub(super) const FIELD: f32 = 8.0;

/// How something showed at a frame: how much of it, where, blinking.
#[derive(Clone)]
pub(super) struct Showing {
    pub(super) shown: f32,
    pub(super) at: Vec2,
    pub(super) blinks: bool,
}

/// A line on screen, as it is watched: its words, who says it, its film
/// line, the frames it has shown.
#[derive(Clone)]
struct Shown {
    text: String,
    who: Option<Who>,
    line: i32,
    frames: u32,
}

/// Something seen as a film went by.
#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    /// The film's line, and the frame it was seen at.
    pub line: i32,
    pub frame: u32,
    pub text: String,
    /// Its kind (see `REMARKS`; `place`, a place the stage found otherwise).
    pub kind: String,
    /// A detail: a place moved a little, one more of a kind seen already.
    pub small: bool,
    /// Those its text names (`diagnostics::Mention`).
    pub mentions: Vec<diagnostics::Mention>,
}

/// What is seen as a film goes by, told with the film's own lines (the number
/// `_ligne`): who speaks out of frame, bodies on the scenery or on one another
/// at rest, a walk that goes nowhere or through the scenery, a crowd among
/// fighters, who a view was to keep and did not, lines gone too fast. It
/// changes nothing.
#[derive(Clone, Default)]
pub struct Watch {
    pub remarks: Vec<Seen>,
    /// What is wrong, seen as it played.
    pub problems: Vec<Seen>,
    /// Those a fight going on has on its field.
    pub fighters: Vec<Who>,
    /// The name each one goes by in what is said (for who is spoken of), and
    /// those of a crowd (no one speaks of them by name; they may hide the others).
    pub names: HashMap<Who, String>,
    pub crowd: HashSet<Who>,
    /// The speaker of the line before, in this scene, and its words: who is
    /// spoken to, if the line answers it.
    pub(super) before: Option<(Who, String)>,
    /// The frames each has been out of the frame while spoken of or to; a big
    /// body cut by the frame's edge; a variant shown a way it is not painted.
    pub(super) away: HashMap<Who, u32>,
    pub(super) cut: HashMap<Who, u32>,
    pub(super) unpainted: HashMap<Who, u32>,
    /// How near each walker came to where it goes, and the looks since.
    pub(super) progress: HashMap<Who, (f32, u32)>,
    pub(super) notes: usize,
    pub(super) told: HashSet<String>,
    shown: Option<Shown>,
    pub(super) still: HashMap<Who, (Vec2, u32)>,
    /// Where the view was at the last look.
    pub(super) looked: Vec2,
    /// How each body showed at the frame before (how much of it), and the
    /// frame the scene began at.
    pub(super) showing: HashMap<Who, Showing>,
    pub(super) scene_began: u32,
    /// How many of each kind were told, by kind and scene.
    pub(super) kinds: HashMap<(String, usize), usize>,
    pub(super) scene: usize,
    pub(super) frame: u32,
    /// The sounds of unknown length told so far.
    sounds_told: usize,
}

impl Watch {
    pub(super) fn tell(&mut self, line: i32, key: String, said: impl Into<diagnostics::Said>) {
        let said = said.into();
        if self.remarks.len() < MOST_TOLD && self.told.insert(key.clone()) {
            let kind = key.split(' ').next().unwrap_or_default().to_string();
            let remark = REMARKS.contains(&kind.as_str());
            let many = self.kinds.entry((kind.clone(), self.scene)).or_insert(0);
            *many += 1;
            let small = *many > MOST_OF_A_KIND || !remark;
            self.remarks.push(Seen { line, frame: self.frame, text: said.text, kind, small, mentions: said.mentions });
        }
    }

    /// A frame has gone by: `page`, the words on screen; `centre`, where the
    /// view is, as it shows.
    pub fn look(&mut self, director: &Director, stage: &Stage, centre: Vec2, page: Option<&str>) {
        let line = director.numbers().get(LINE).copied().unwrap_or(0.0) as i32;
        self.frame += 1;
        let scene = director.at.map_or(0, |at| at.scene);
        if scene != self.scene {
            (self.before, self.scene, self.scene_began) = (None, scene, self.frame);
            self.showing.clear();
        }
        self.notes_of(stage);
        self.sounds_unknown(director, line);
        self.line_shown(director, stage, centre, page, line);
        self.unpainted(director, stage, line);
        self.cut_by_the_edge(stage, centre, line);
        self.pops(stage, centre, line);
        if self.frame.is_multiple_of(LOOK_EVERY) {
            self.bodies(stage, centre, line, self.scene);
        }
    }

    /// The sounds waited for whose length is not known: waited for a while
    /// of their own.
    fn sounds_unknown(&mut self, director: &Director, line: i32) {
        let unknown = director.unknown_sounds().to_vec();
        for id in unknown.iter().skip(self.sounds_told) {
            let text = format!(
                "the length of sound {id} is not known (no game sounds here): “wait sound” waits {:.1} s for it → “wait 1s” if it matters",
                crate::director::SOUND_FALLBACK / 60.0
            );
            self.tell(line, format!("sound {id}"), text);
        }
        self.sounds_told = unknown.len();
    }

    /// What the stage noted as places were found.
    fn notes_of(&mut self, stage: &Stage) {
        for note in stage.place.notes.iter().skip(self.notes) {
            // (A place found otherwise than asked: the stage's own, a detail.)
            let seen = Seen {
                line: note.line,
                frame: self.frame,
                text: note.text.clone(),
                kind: "place".to_string(),
                small: note.small || !note.wrong,
                mentions: note.mentions.clone(),
            };
            match note.wrong {
                true if self.told.insert(note.text.clone()) => self.problems.push(seen),
                false if (note.small || self.remarks.len() < MOST_TOLD) && self.told.insert(note.text.clone()) => self.remarks.push(seen),
                _ => {}
            }
        }
        self.notes = stage.place.notes.len();
    }

    fn line_shown(&mut self, director: &Director, stage: &Stage, centre: Vec2, page: Option<&str>, line: i32) {
        if self.shown.as_ref().map(|shown| shown.text.as_str()) != page {
            if let Some(Shown { text, line, frames, who }) = self.shown.take() {
                if let Some(who) = who {
                    self.before = Some((who, without_tags(&text).to_lowercase()));
                }
                self.away.clear();
                // (Written out at the box's speed, then read: see `least_shown`.)
                if (frames as f32) < crate::director::least_shown(&text) {
                    let words: String = without_tags(&text).chars().filter(|letter| *letter != '\n').take(30).collect();
                    let text = format!(
                        "“{}” stays only {:.1} s on screen, too little to be read → take the “&” off the line, or wait before the next one",
                        words.trim(),
                        frames as f32 / 60.0
                    );
                    self.tell(line, format!("fast {line}"), text);
                }
            }
            // (A narration has no name before its words.)
            self.shown =
                page.map(|text| Shown { text: text.to_string(), who: director.manner.last_speaker.filter(|_| text.starts_with("[CS:N]")), line, frames: 0 });
        }
        let Some(shown) = self.shown.as_mut() else {
            return;
        };
        shown.frames += 1;
        // (Once the view had time to get there: who is spoken of or to.)
        if shown.frames > READ_AFTER + 20 {
            let (speaker, text, line) = (shown.who, shown.text.clone(), shown.line);
            self.spoken_of(stage, centre, speaker, &text, line);
        }
        let Some(shown) = self.shown.as_mut() else {
            return;
        };
        if shown.frames != READ_AFTER + 20 {
            return;
        }
        let (speaker, text, line) = (shown.who, shown.text.clone(), shown.line);
        // (`???: …`, `umbreon?: …`: one not to be seen yet.)
        let unseen = without_tags(&text).starts_with("???");
        if let Some(why) = speaker.filter(|_| !unseen).and_then(|who| out_of_sight(stage, Some(centre), who, &text)) {
            self.tell(line, format!("view {line}"), why);
        }
        if let Some(who) =
            speaker.filter(|who| stage.cast.get(who).is_some_and(|actor| actor.attributes & attribute::HIDDEN != 0 || actor.shown_opacity() <= 0.1))
        {
            let words: String = without_tags(&text).chars().filter(|letter| *letter != '\n').take(30).collect();
            let text = told!("{} speaks while K.O. or hidden (“{}”) → bring them back before, or give the line to another", who, words.trim());
            self.tell(line, format!("absent {line}"), text);
        }
        self.things_spoken_of(stage, centre, &text, line);
    }
}
