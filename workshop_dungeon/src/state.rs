//! The dungeon being made: opened, new, saved, tried; the keys it is played
//! with.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use eframe::egui;
use shell::Screen;
use shell::sound::Hears;

use super::choosers::kept_team;
use super::{ArenaTool, BossForm, DungeonMaking, Form, Trial, offered};
use assets::project::Project;
use workshop_base::session::Session;

impl DungeonMaking {
    pub fn new(session: &Arc<Session>, word: &str) -> DungeonMaking {
        let form = Form::new("New Dungeon");
        DungeonMaking::of(session, word, form, false)
    }

    /// One of the shared library's.
    pub fn open(session: &Arc<Session>, word: &str) -> DungeonMaking {
        DungeonMaking::open_in(session, &session.library().clone(), word)
    }

    /// One written in that project's folder (a game's own, or the shared library): as its file has it, written back there.
    pub fn open_in(session: &Arc<Session>, project: &Project, word: &str) -> DungeonMaking {
        let path = project.dungeons().join(format!("{word}.dungeon"));
        workshop_base::safe::check(session, &path);
        let text = std::fs::read_to_string(path).unwrap_or_default();
        DungeonMaking { project: project.clone(), ..DungeonMaking::of(session, word, Form::read(&text.replace("\r\n", "\n")), true) }
    }

    /// A new boss fight of its own.
    pub fn new_fight(session: &Arc<Session>, word: &str) -> DungeonMaking {
        let boss = BossForm { look: "Apple Woods".to_string(), ..BossForm::new() };
        let form = Form { name: "New Fight".to_string(), boss: Some(boss), ..Form::new("") };
        DungeonMaking { fight: true, ..DungeonMaking::of(session, word, form, false) }
    }

    /// A boss fight of the shared library's.
    pub fn open_fight(session: &Arc<Session>, word: &str) -> DungeonMaking {
        DungeonMaking::open_fight_in(session, &session.library().clone(), word)
    }

    /// A boss fight written in that project's folder: as its file has it, written back there.
    pub fn open_fight_in(session: &Arc<Session>, project: &Project, word: &str) -> DungeonMaking {
        let path = project.fights().join(format!("{word}.fight"));
        workshop_base::safe::check(session, &path);
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let mut form = Form::read(&text.replace("\r\n", "\n"));
        form.boss.get_or_insert_with(BossForm::new);
        DungeonMaking { fight: true, project: project.clone(), ..DungeonMaking::of(session, word, form, true) }
    }

    /// Its file, where it is written (or will be).
    pub fn path(&self) -> std::path::PathBuf {
        let (folder, end) = if self.fight { (self.project.fights(), "fight") } else { (self.project.dungeons(), "dungeon") };
        folder.join(format!("{}.{end}", self.word))
    }

    /// Whether it is the open game's own (written in its folder), not the shared library's.
    pub fn own(&self) -> bool {
        !self.project.is_library()
    }

    /// Its file's text: a dungeon's, or a fight's.
    pub(super) fn file_text(&self) -> String {
        if self.fight { self.form.fight_text() } else { self.form.text() }
    }

    /// What is played to try it: the dungeon with its fight read in, or the fight alone.
    pub(super) fn played_text(&self, fight_alone: bool) -> String {
        if self.fight {
            game::dive::fight_alone(&self.form.fight_text())
        } else if fight_alone {
            Form { floors: 0, ..self.form.clone() }.text()
        } else {
            self.form.text()
        }
    }

    pub(super) fn of(session: &Arc<Session>, word: &str, form: Form, saved: bool) -> DungeonMaking {
        DungeonMaking {
            session: session.clone(),
            project: session.library().clone(),
            word: word.to_string(),
            saved,
            written: if saved { form.clone() } else { Form::new("") },
            form,
            checked: (String::new(), Vec::new()),
            sought_foe: String::new(),
            sought_item: String::new(),
            trial: None,
            said: None,
            portraits: HashMap::new(),
            heard: None,
            fight: false,
            boss_life: session.launch.boss_hp.unwrap_or(100),
            sought_move: String::new(),
            arena_tool: ArenaTool::Item,
            arena_item: "Oran Berry".to_string(),
            arena_trap: "spikes".to_string(),
            trap_hidden: false,
            phase_sought: Vec::new(),
            sought_guard: String::new(),
            sought_ally: String::new(),
            team: kept_team(session.root()),
            numbers: (String::new(), None),
            trying_shown: false,
            floor_shown: super::floor::FloorShown::new(&session.launch),
            tile_import: None,
            pointed: None,
            check_sought: false,
        }
    }

    /// Written in its file.
    pub(super) fn save(&mut self) {
        let word = film::words::plain(&self.word);
        if word.is_empty() {
            self.said = Some("it needs a word (the name of its file)".to_string());
            return;
        }
        self.word = word;
        let path = self.path();
        if let Some(folder) = path.parent() {
            _ = std::fs::create_dir_all(folder);
        }
        let shown = path.strip_prefix(self.project.root()).unwrap_or(&path).display().to_string().replace('\\', "/");
        match workshop_base::safe::write(&path, self.file_text()) {
            Ok(()) => {
                (self.saved, self.written) = (true, self.form.clone());
                self.said = Some(if self.fight {
                    format!("written: {shown} · a dungeon names it: “fight: {}”; alone in a film: “dungeon {}”", self.word, self.word)
                } else {
                    format!("written: {shown} · in a film: “dungeon {}”", self.word)
                });
            }
            Err(why) => self.said = Some(format!("{}: {why}", path.display())),
        }
    }

    /// Played from its first floor, as it is written now (`fight`: its boss fight alone).
    pub(super) fn try_it(&mut self, audio: &mut Option<Box<dyn Hears>>, fight: bool) {
        self.stop(audio);
        let Some(offered) = offered(&self.session) else {
            self.said = Some("the game's data is missing (extractor)".to_string());
            return;
        };
        let (hero, partner, level) = self.team.unwrap_or((offered.team.0, offered.team.1, 5));
        // (The dungeon's own level for the team, if it says one; else the trial's.)
        let team = game::dive::Team { hero, partner, level: if self.form.team_level.is_some() { None } else { Some(level) } };
        let text = self.played_text(fight);
        match game::dive::Diving::of_text(self.session.data(), &self.project, &text, &self.word, team, &game::dive::Carried::default()) {
            Ok(mut diving) => {
                diving.frame.audio = audio.take();
                // (The fight tried from its middle: the boss already wounded.)
                if fight && self.boss_life < 100 {
                    diving.boss_at(self.boss_life);
                }
                self.trial = Some(Trial { diving, screen: Screen::new(2), texture: None, last: Instant::now() });
            }
            Err(why) => self.said = Some(why.to_string()),
        }
    }

    /// The trial over: its sound given back.
    pub(super) fn stop(&mut self, audio: &mut Option<Box<dyn Hears>>) {
        if let Some(mut trial) = self.trial.take()
            && let Some(mut lent) = trial.diving.frame.audio.take()
        {
            lent.song(0, None, 0.0, 0.0);
            *audio = Some(lent);
        }
    }
}

/// The keys of the editor, as the dungeon reads them.
pub(super) fn keys_of(input: &egui::InputState) -> shell::Keys {
    use egui::Key;
    let held = |key| input.key_down(key);
    let axis = |less, more| f32::from(u8::from(held(more))) - f32::from(u8::from(held(less)));
    let names = |keys: Vec<Key>| keys.into_iter().map(|key| key.name().to_lowercase()).collect::<Vec<_>>();
    let all = [Key::A, Key::D, Key::S, Key::W, Key::I, Key::R, Key::Y, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::F5, Key::F9, Key::F10];
    shell::Keys {
        enter: input.key_pressed(Key::Enter),
        up: input.key_pressed(Key::ArrowUp),
        down: input.key_pressed(Key::ArrowDown),
        left: input.key_pressed(Key::ArrowLeft),
        right: input.key_pressed(Key::ArrowRight),
        escape: input.key_pressed(Key::Escape) || input.key_pressed(Key::Backspace),
        tab: input.key_pressed(Key::Tab),
        toward: glam::Vec2::new(axis(Key::ArrowLeft, Key::ArrowRight), axis(Key::ArrowUp, Key::ArrowDown)),
        run: input.modifiers.shift,
        hurry: held(Key::Space),
        named: names(all.iter().copied().filter(|key| input.key_pressed(*key)).collect()),
        held: names(all.iter().copied().filter(|key| input.key_down(*key)).collect()),
        typed: Vec::new(),
    }
}
