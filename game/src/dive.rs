//! A dungeon (or a boss fight) of one's own gone through by the game: its
//! team, lent (`Playthrough::lend`), goes through the floors of
//! `<project>/dungeons/<name>.dungeon` as the game's dungeons go (the rules
//! of `rules`, the screen of `dive`), then is given back with
//! what it has become; the game goes on knowing how it ended.

use std::sync::Arc;

use assets::project::Project;
use dive::{Dive, Done, Frame, Turn};
use rules::run::dungeon::Outcome;
use rules::run::{Options, Run};
use shell::{Keys, Screen};
use stage_view::ui::{Canvas, UiArt};

/// Who goes through: the species of the hero and of its partner, the level
/// they come in at (none: the dungeon's, or as they start).
#[derive(Clone, Copy, Debug)]
pub struct Team {
    pub hero: u16,
    pub partner: u16,
    pub level: Option<i32>,
}

/// What the team carries: the items by their names as the game writes them,
/// how many of each; the money.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Carried {
    pub items: Vec<(String, u16)>,
    pub money: i32,
}

/// Why a game lent to a dungeon could not go in, and the game given back.
pub type Refused = (String, Box<Run>);

/// A dungeon being gone through, in a film.
pub struct Diving {
    /// The project whose dungeons, fights, tilesets and sounds it plays.
    project: Project,
    run: Run,
    /// The run is the film's game, lent (given back by `give_back`); the
    /// film's game kept aside meanwhile, if the dungeon went with its own.
    lent: bool,
    kept: Option<Run>,
    /// The screens it draws on (its sound: the film's, lent while it lasts).
    pub frame: Frame,
    dive: Dive,
    /// Over: cleared or not; what the rules said of it as it was left (what fainting cost, the jobs paid).
    pub ended: Option<bool>,
    pub left_said: String,
    /// How many floors it has; the last one reached.
    floors: usize,
    pub reached: usize,
    /// Its boss fight, as it goes; whether the boss was beaten.
    boss: Option<BossFight>,
    pub beaten: bool,
    /// Its time not yet played (in its frames), and the keys pressed meanwhile.
    owed: f32,
    /// The fight's staging: a banner (its two lines, frames left), a white
    /// flash and a shake of the screen (frames left); an end held a moment
    /// for the victory to show (how it ended, frames left).
    banner: Option<(String, String, u32)>,
    flash: u32,
    shake: u32,
    held_end: Option<(bool, u32)>,
    pending: Option<Keys>,
}

/// The victory tune (“Mission accomplished”).
const VICTORY: u16 = 5;

/// The banner's band on the lower screen: its top, its height, in pixels of the game.
const BANNER: (i32, i32) = (102, 30);

/// Where the boss's gauge is on the lower screen (right of the little map,
/// under the team's line): x, y, how wide, how high, in pixels of the game.
const GAUGE: (i32, i32, i32, i32) = (100, 183, 148, 5);
/// The strip the gauge lies on, at the foot of the lower screen, under the message box: its top.
const STRIP: i32 = 178;

/// A line cut into pieces of at most so many letters, at spaces.
fn wrapped(line: &str, most: usize) -> Vec<String> {
    let mut pieces: Vec<String> = Vec::new();
    for word in line.split(' ') {
        match pieces.last_mut() {
            Some(last) if last.chars().count() + 1 + word.chars().count() <= most => {
                last.push(' ');
                last.push_str(word);
            }
            _ => pieces.push(word.to_string()),
        }
    }
    pieces
}

/// A boss fight as it goes: what the boss says, whether the fight has begun,
/// whether it grew angry, whether it fell; its life as last seen.
struct BossFight {
    says: rules::own::BossSays,
    begun: bool,
    angered: bool,
    fallen: bool,
    life: (i32, i32),
    /// The phases played (their places in the list).
    played: Vec<usize>,
}

/// A floor as laid out: who stands where, where items lie, where traps are
/// (tiles of the floor).
pub type LaidOut = (Vec<(String, (i32, i32))>, Vec<(i32, i32)>, Vec<(i32, i32)>);

/// A dungeon's text with its fight read in: each `fight: <word>` line becomes
/// the lines of that project's `fights/<word>.fight` (its own, else the
/// shared library's; its `name:` left out: the dungeon keeps its own). A
/// fight not found is left for the rules to tell. Its `tileset: <word>`
/// becomes the number the art of that project knows that picture by
/// (`tilesets/<word>.png`, see `shell::art::own_tileset`), if it is there.
pub fn resolved_for(project: &Project, text: &str) -> String {
    resolved_with(project, text, &|word| project.fight(word), &|word| project.file("tilesets", word, "png"))
}

/// As `resolved`, each fight's file and each tileset's picture where those say.
fn resolved_with(project: &Project, text: &str, fight_file: &dyn Fn(&str) -> std::path::PathBuf, tileset_file: &dyn Fn(&str) -> std::path::PathBuf) -> String {
    text.lines()
        .flat_map(|line| {
            let said = |wanted: &str| {
                line.split('#').next().unwrap_or_default().split_once(':').filter(|(key, _)| key.trim() == wanted).map(|(_, word)| word.trim().to_string())
            };
            if let Some(word) = said("tileset") {
                let path = tileset_file(&word);
                return match path.is_file().then(|| shell::art::own_tileset(project, &path)).flatten() {
                    Some(id) => vec![format!("tileset: {id}")],
                    None => vec![line.to_string()],
                };
            }
            let word = said("fight");
            match word.and_then(|word| std::fs::read_to_string(fight_file(&word)).ok()) {
                Some(fight) => {
                    fight.replace("\r\n", "\n").lines().filter(|line| !line.trim_start().starts_with("name:")).map(str::to_string).collect::<Vec<_>>()
                }
                None => vec![line.to_string()],
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A boss fight played alone, as a dungeon with no floor before it: its
/// name (its `name:`, else its boss's), no floor, its lines.
pub fn fight_alone(text: &str) -> String {
    let named = text.lines().find_map(|line| line.trim().strip_prefix("name:").map(|name| name.trim().to_string()));
    let boss = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("boss:").map(|said| said.trim().rsplit_once(' ').map_or(said.trim(), |(name, _)| name).to_string()));
    let name = named.or(boss).unwrap_or_else(|| "Fight".to_string());
    let lines: Vec<&str> = text.lines().filter(|line| !line.trim_start().starts_with("name:")).collect();
    format!("name: {name}\nfloors: 0\n{}", lines.join("\n"))
}

/// A dungeon's text by its name (else a boss fight's, played alone), and
/// where it was read.
pub(crate) fn text_of(project: &Project, name: &str) -> Result<(String, String), String> {
    let path = project.dungeon(name);
    let fight = project.fight(name);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok((text, path.display().to_string())),
        Err(why) => match std::fs::read_to_string(&fight) {
            Ok(text) => Ok((fight_alone(&text), fight.display().to_string())),
            Err(_) => Err(format!("{}: {why}", path.display())),
        },
    }
}

/// The film's number for an item of its bag (`Golden Apple`: `bag_golden_apple`).
pub fn bag_number(name: &str) -> String {
    format!("bag_{}", film::words::plain(&name.split_whitespace().collect::<Vec<_>>().join("_")))
}

impl Diving {
    /// The dungeon (else the boss fight) of that word in the project, gone
    /// through by the game's own run, lent (at that level if one is said).
    pub fn start_with(data: &str, project: &Project, name: &str, run: Run, level: Option<i32>, at: Option<u16>) -> Result<Self, Refused> {
        match text_of(project, name) {
            Ok((text, path)) => Self::with_run(data, project, &text, &path, run, level, at),
            Err(why) => Err((why, Box::new(run))),
        }
    }

    /// A dungeon from its text (`said`: what to call it in what is wrong),
    /// gone through by that team, carrying that (a game of its own, left
    /// when it ends).
    pub fn of_text(data: &str, project: &Project, text: &str, said: &str, team: Team, carried: &Carried) -> Result<Self, crate::Error> {
        let mut run = Run::new(Arc::new(crate::playthrough::book(data)?), Options::default());
        let known = |name: &str| run.book.tables.items.iter().position(|item| rules::own::plain(item) == rules::own::plain(name)).map(|item| item as u16);
        let items: Vec<(u16, u16)> = carried.items.iter().filter_map(|(name, count)| Some((known(name)?, *count))).collect();
        for (item, count) in items {
            run.give_items(item, count);
        }
        run.add_money(carried.money);
        Self::going(data, project, text, said, run, Some(team), None, None).map_err(|(why, _)| crate::Error::Said(why))
    }

    /// A dungeon from its text, gone through by the game of the film (its
    /// team, its recruits, its bag, its money: `Playthrough`), brought to that
    /// level if one is said; given back by `give_back` once over (or at
    /// once, with why not, if it cannot be).
    /// (`at`: the number the rules already know it by, its table read again there.)
    pub fn with_run(data: &str, project: &Project, text: &str, said: &str, run: Run, level: Option<i32>, at: Option<u16>) -> Result<Self, Refused> {
        Self::going(data, project, text, said, run, None, level, at)
    }

    /// The game lent back once the dungeon is over (none: it went with a game of its own).
    pub fn give_back(self) -> Option<Run> {
        match (self.kept, self.lent) {
            (Some(kept), _) => Some(kept),
            (None, true) => Some(self.run),
            (None, false) => None,
        }
    }

    /// Into the dungeon of that text: by that team (`team`: a game of its
    /// own), else by the game's own, at that level if said.
    #[allow(clippy::too_many_arguments)]
    fn going(
        data: &str,
        project: &Project,
        text: &str,
        said: &str,
        mut run: Run,
        team: Option<Team>,
        level: Option<i32>,
        at: Option<u16>,
    ) -> Result<Self, Refused> {
        let lent = team.is_none();
        let own = match rules::own::read(&resolved_for(project, text), &run.book.tables) {
            Ok(own) => own,
            Err(wrong) => return Err((format!("{said} : {}", wrong.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ; ")), Box::new(run))),
        };
        let floors = own.table.floors.len();
        let mut own = own;
        let boss = own.boss.take().map(|says| BossFight { says, begun: false, angered: false, fallen: false, life: (0, 0), played: Vec::new() });
        // (A fight played alone, by the team it says, if it says one: a game
        // of its own then, the film's kept aside as it was.)
        let alone = floors == 1 && boss.is_some();
        let says = boss.as_ref().map(|fight| &fight.says).filter(|_| alone);
        if lent && says.is_some_and(|says| says.hero.is_some()) {
            let mut fresh = match crate::playthrough::book(data) {
                Ok(book) => Run::new(Arc::new(book), Options::default()),
                Err(why) => return Err((why.to_string(), Box::new(run))),
            };
            // (What the film's team carries, lent: nothing of it comes back.)
            fresh.fill_bag(run.bag().items.clone());
            fresh.add_money(run.bag().money);
            let species = run.team().species;
            let film_s = std::mem::replace(&mut run, fresh);
            let team = Team { hero: species.0, partner: species.1, level: None };
            // (Refused: the film's game given back, not the one made for it.)
            return match Self::going(data, project, text, said, run, Some(team), None, None) {
                Ok(mut diving) => {
                    diving.kept = Some(film_s);
                    Ok(diving)
                }
                Err((why, _)) => Err((why, Box::new(film_s))),
            };
        }
        let Some(book) = Arc::get_mut(&mut run.book) else {
            return Err(("the game's rule book is held elsewhere".to_string(), Box::new(run)));
        };
        let id = match at.filter(|at| (*at as usize) < book.tables.dungeons.len()) {
            Some(at) => {
                book.tables.dungeons[at as usize] = own.table;
                book.tables.dungeon_names[at as usize] = own.name;
                at
            }
            None => rules::own::add(&mut book.tables, own.name, own.table),
        };
        // (The types it gives, its own Fairy among them: the tables it is played with.)
        rules::own::retype(&mut book.tables, &own.species_types, &own.move_types);
        // (Gone to with a job of the team's there: its targets on its floors, paid on the way out.)
        let jobs = lent && run.team().progress.jobs.iter().any(|job| job.dungeon == id);
        match team {
            Some(team) => {
                let (hero, partner) = (
                    says.and_then(|says| says.hero).unwrap_or(team.hero),
                    says.and_then(|says| says.partner.or(says.hero.map(|_| team.partner))).unwrap_or(team.partner),
                );
                run.change_team_at(hero, partner, says.and_then(|says| says.level).or(team.level).or(own.team_level));
            }
            // (The film's game: its level only if one is said, what it has become kept otherwise.)
            None => {
                if let Some(level) = level.or(own.team_level).filter(|level| *level != run.member_level(rules::run::town::Whom::Leader)) {
                    let (hero, partner) = run.team().species;
                    run.change_team_at(hero, partner, Some(level));
                }
            }
        }
        let ui = UiArt::load(data).map(Arc::new);
        let mut frame = Frame::new(data, shell::art::Art::open_in(data, project.clone()), ui, None);
        frame.scratch = true;
        let mut dive = Dive::default();
        let mut done = None;
        dive::enter(&mut Turn { run: &mut run, show: &mut frame, here: &mut dive, next: &mut done }, id, jobs);
        Ok(Diving {
            project: project.clone(),
            run,
            lent,
            kept: None,
            frame,
            dive,
            ended: None,
            left_said: String::new(),
            floors,
            reached: 1,
            boss,
            beaten: false,
            owed: 0.0,
            pending: None,
            banner: None,
            flash: 0,
            shake: 0,
            held_end: None,
        })
    }

    /// So much time gone by (in frames of the game, sixty a second): as many
    /// of its frames played, however fast whoever shows it turns. What was
    /// pressed counts once (kept for the next frame if none is due yet), what
    /// is held each frame.
    pub fn play(&mut self, keys: &Keys, frames: f32) {
        let mut pressed = match self.pending.take() {
            Some(earlier) => Keys {
                enter: earlier.enter || keys.enter,
                up: earlier.up || keys.up,
                down: earlier.down || keys.down,
                left: earlier.left || keys.left,
                right: earlier.right || keys.right,
                escape: earlier.escape || keys.escape,
                tab: earlier.tab || keys.tab,
                named: earlier.named.into_iter().chain(keys.named.iter().cloned()).collect(),
                ..keys.clone()
            },
            None => keys.clone(),
        };
        let still = Keys { toward: keys.toward, run: keys.run, hurry: keys.hurry, held: keys.held.clone(), ..Keys::default() };
        self.owed = (self.owed + frames).min(4.0);
        if self.owed < 1.0 {
            self.pending = Some(pressed.clone());
        }
        while self.owed >= 1.0 && self.ended.is_none() {
            self.owed -= 1.0;
            self.tick(&pressed);
            pressed = still.clone();
        }
    }

    /// A frame of the dungeon: the keys read, the rules played, all drawn.
    pub fn tick(&mut self, keys: &Keys) {
        if self.ended.is_some() {
            return;
        }
        (self.flash, self.shake) = (self.flash.saturating_sub(1), self.shake.saturating_sub(1));
        if let Some((_, _, left)) = &mut self.banner {
            *left = left.saturating_sub(1);
        }
        // (The victory shown a moment before the film goes on: the last picture kept.)
        if let Some((cleared, left)) = &mut self.held_end {
            *left = left.saturating_sub(1);
            if *left == 0 {
                self.ended = Some(*cleared);
            }
            return;
        }
        self.frame.canvas.clear();
        self.frame.world.clear();
        dive::look::prepare(&mut self.frame, &self.run);
        let mut done = None;
        dive::frame(&mut Turn { run: &mut self.run, show: &mut self.frame, here: &mut self.dive, next: &mut done }, keys);
        dive::look::settle(&mut self.frame, keys);
        if let Some(audio) = &mut self.frame.audio {
            audio.tick();
        }
        if let Some((at, _)) = self.floor() {
            self.reached = self.reached.max(at);
        }
        self.boss_fight();
        let ended = match done {
            Some(Done::Left { outcome, message }) => {
                self.left_said = message;
                Some(outcome == Outcome::Cleared)
            }
            Some(Done::ToTitle) => Some(false),
            None => None,
        };
        match ended {
            // (The boss beaten: “VICTORY!”, the victory tune, then on.)
            Some(true) if self.beaten => {
                self.held_end = Some((true, 200));
                let name = self.boss.as_ref().map_or_else(String::new, |fight| fight.says.name.clone());
                self.banner = Some(("VICTORY!".to_string(), format!("{name} is defeated"), 200));
                self.flash = 16;
                if let Some(audio) = self.frame.audio.as_mut() {
                    audio.song(0, Some(VICTORY), 0.8, 0.0);
                }
                self.banner_text();
            }
            _ => self.ended = ended,
        }
    }

    /// The boss fight, on its floor: what the boss says as it begins; each of
    /// its phases, once, as its life falls under its share (what it says, its
    /// anger, its help, its healing, the weather, its moves); as it falls, its
    /// last words. Its name and its life shown at the top while it stands.
    fn boss_fight(&mut self) {
        use rules::run::Side;
        let on_its_floor = self.floor().is_some_and(|(at, of)| at == of);
        let Some(fight) = self.boss.as_mut().filter(|_| on_its_floor) else {
            return;
        };
        let species = fight.says.species;
        let standing = self.run.dungeon().and_then(|dungeon| {
            dungeon
                .monsters
                .iter()
                .find(|monster| monster.side == Side::Foe && monster.species == Some(species) && monster.alive())
                .map(|monster| (monster.stats.hp, monster.stats.max_hp))
        });
        let name = fight.says.name.clone();
        let mut said = Vec::new();
        let mut cries: Vec<String> = Vec::new();
        match standing {
            Some(life) => {
                fight.life = life;
                if !fight.begun {
                    fight.begun = true;
                    self.banner = Some((name.clone(), fight.says.title.clone(), 170));
                    if !fight.says.cry.is_empty() {
                        self.shake = 40;
                    }
                    self.run.teach_foe(species, &fight.says.moves);
                    if !fight.says.cry.is_empty() {
                        cries.push(fight.says.cry.clone());
                    }
                    if fight.says.joins {
                        self.run.boss_may_join(species);
                    }
                    if !fight.says.begins.is_empty() {
                        said.push(format!("{name}: “{}”", fight.says.begins));
                    }
                }
                // (Each phase under its share, the highest first, once.)
                let mut order: Vec<usize> = (0..fight.says.phases.len()).filter(|at| !fight.played.contains(at)).collect();
                order.sort_by_key(|at| std::cmp::Reverse(fight.says.phases[*at].share));
                for at in order {
                    let phase = &fight.says.phases[at];
                    if life.0 * 100 > fight.life.1 * phase.share {
                        continue;
                    }
                    fight.played.push(at);
                    self.flash = 14;
                    if phase.shake {
                        self.shake = 40;
                    }
                    if !phase.says.is_empty() {
                        said.push(format!("{name}: “{}”", phase.says));
                    }
                    if phase.fiercer > 0 && self.run.rouse(species, phase.fiercer) {
                        fight.angered = true;
                        if phase.says.is_empty() {
                            said.push(format!("{name} flies into a black rage!"));
                        }
                        said.push(format!("{name}'s Attack rose!"));
                    }
                    if !phase.help.is_empty() {
                        let came = self.run.summon_near(species, &phase.help);
                        if came > 0 {
                            said.push(format!("{came} Pokémon come to help {name}!"));
                        }
                    }
                    if phase.heal > 0 {
                        let gained = self.run.heal_foe(species, phase.heal);
                        if gained > 0 {
                            said.push(format!("{name} regains its strength! (+{gained} HP)"));
                        }
                    }
                    if let Some(kind) = phase.weather {
                        self.run.set_weather(kind);
                        said.push(match kind {
                            1 => "The sunlight turned harsh!".to_string(),
                            2 => "A sandstorm kicked up!".to_string(),
                            4 => "It started to rain!".to_string(),
                            5 => "It started to hail!".to_string(),
                            6 => "A thick fog fell!".to_string(),
                            _ => "The weather cleared.".to_string(),
                        });
                    }
                    if !phase.cry.is_empty() {
                        cries.push(phase.cry.clone());
                    }
                    if !phase.moves.is_empty() {
                        self.run.teach_foe(species, &phase.moves);
                        said.push(format!("{name} changes the way it fights!"));
                    }
                }
            }
            None if fight.begun && !fight.fallen => {
                (fight.fallen, fight.life.0) = (true, 0);
                self.beaten = true;
                if !fight.says.falls.is_empty() {
                    said.push(format!("{name}: “{}”", fight.says.falls));
                }
                said.push(format!("{name} is defeated!"));
            }
            None => {}
        }
        // (Its cries: sounds of ours, as loud as the rest.)
        for cry in cries {
            if let (Some(audio), Some(stereo)) = (self.frame.audio.as_mut(), shell::own_sounds::read(&self.frame.folder, &self.project, &cry)) {
                audio.samples(stereo, 0.9);
            }
        }
        // (The box holds about forty letters a line: what is longer goes on below.)
        for line in said {
            for piece in wrapped(&line, 40) {
                self.run.say(piece);
            }
        }
        self.banner_text();
        let Some(fight) = self.boss.as_ref() else {
            return;
        };
        // (Its name over its gauge, at the top of the lower screen.)
        if fight.begun
            && !fight.fallen
            && let Some(ui) = self.frame.ui.clone()
        {
            let lower = stage_view::ui::SCREEN_HEIGHT as i32;
            self.frame.canvas.text(&ui, 6, lower + STRIP + 1, &name, [0xF8, 0xE8, 0xA0]);
        }
    }

    /// The banner's lines, on the interface, in the middle of the screen.
    fn banner_text(&mut self) {
        let (Some((first, second, left)), Some(ui)) = (self.banner.clone(), self.frame.ui.clone()) else {
            return;
        };
        if left == 0 {
            return;
        }
        let lower = stage_view::ui::SCREEN_HEIGHT as i32;
        let wide = stage_view::ui::SCREEN_WIDTH as i32;
        let across = |text: &str| (text.chars().count() as i32 * 6).min(wide - 16);
        self.frame.canvas.text(&ui, (wide - across(&first)) / 2, lower + BANNER.0 + 3, &first, [0xF8, 0xD8, 0x50]);
        if !second.is_empty() {
            self.frame.canvas.text(&ui, (wide - across(&second)) / 2, lower + BANNER.0 + 16, &second, [0xF0, 0xF0, 0xF0]);
        }
    }

    /// The boss left with so many hundredths of its life (a fight tried from
    /// its middle, to see what it does then).
    pub fn boss_at(&mut self, hundredths: i32) {
        if let Some(fight) = &self.boss {
            self.run.set_foe_life(fight.says.species, hundredths);
        }
    }

    /// Those standing, as numbers: name, level, life, attack, defense,
    /// special attack and defense (to see how a fight is balanced).
    pub fn stats_said(&self) -> String {
        self.run.dungeon().map_or_else(String::new, |dungeon| {
            dungeon
                .monsters
                .iter()
                .filter(|monster| monster.alive())
                .map(|monster| {
                    let stats = &monster.stats;
                    format!(
                        "{} Lv.{} HP {} Atk {} Def {} SpA {} SpD {}",
                        monster.name, stats.level, stats.max_hp, stats.attack, stats.defense, stats.sp_attack, stats.sp_defense
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    /// Who stands where on the floor (tiles of the floor), and where items
    /// and traps lie (to see a room as it was laid out).
    pub fn laid_out(&self) -> LaidOut {
        self.run.dungeon().map_or_else(Default::default, |dungeon| {
            (
                dungeon.monsters.iter().filter(|monster| monster.alive()).map(|monster| (monster.name.clone(), (monster.at.x, monster.at.y))).collect(),
                dungeon.items.iter().map(|item| (item.at.x, item.at.y)).collect(),
                dungeon.traps.iter().map(|trap| (trap.at.x, trap.at.y)).collect(),
            )
        })
    }

    /// How many of its frames have gone by.
    pub fn clock(&self) -> u64 {
        self.run.frame()
    }

    /// What the dungeon has said so far.
    pub fn said(&self) -> &[String] {
        &self.run.log.lines
    }

    /// The boss's life, while it fights: (now, at most).
    pub fn boss_life(&self) -> Option<(i32, i32)> {
        self.boss.as_ref().filter(|fight| fight.begun && !fight.fallen).map(|fight| fight.life)
    }

    /// The lower screen as the dungeon shows it.
    pub fn draw(&self, world: &mut Screen) {
        let drawn = &self.frame.world;
        if drawn.scale == world.scale {
            world.rgba.copy_from_slice(&drawn.rgba);
        } else {
            // (Another size of window: each pixel taken from the nearest.)
            for y in 0..world.high {
                for x in 0..world.wide {
                    let (from_x, from_y) = (x * drawn.wide / world.wide, y * drawn.high / world.high);
                    let (at, from) = ((y * world.wide + x) * 4, (from_y * drawn.wide + from_x) * 4);
                    world.rgba[at..at + 4].copy_from_slice(&drawn.rgba[from..from + 4]);
                }
            }
        }
        let scale = world.scale as i32;
        // (The strip under the boss.s name and gauge, at the foot of the screen.)
        if self.boss_life().is_some() {
            world.fill(0, STRIP * scale, world.wide as i32, (world.high as i32) - STRIP * scale, [0x08, 0x08, 0x14, 210]);
            world.fill(0, STRIP * scale, world.wide as i32, scale, [0xF8, 0xD8, 0x50, 255]);
        }
        // (The banner's band, under its words: dark, fading in and out.)
        if let Some((_, _, left)) = &self.banner
            && *left > 0
        {
            let alpha = (*left).min(20) * 10;
            world.fill(0, BANNER.0 * scale, world.wide as i32, BANNER.1 * scale, [0x08, 0x08, 0x18, alpha.min(200) as u8]);
            world.fill(0, BANNER.0 * scale, world.wide as i32, scale, [0xF8, 0xD8, 0x50, alpha.min(255) as u8]);
            world.fill(0, (BANNER.0 + BANNER.1 - 1) * scale, world.wide as i32, scale, [0xF8, 0xD8, 0x50, alpha.min(255) as u8]);
        }
        world.over(&self.frame.canvas.data);
        // (The boss's gauge: a frame, its life in red, orange once it is angry.)
        if let Some((now, most)) = self.boss_life() {
            let scale = world.scale as i32;
            let (x, y, wide, high) = GAUGE;
            world.fill(x * scale - scale, y * scale - scale, (wide + 2) * scale, (high + 2) * scale, [0x10, 0x10, 0x18, 255]);
            world.fill(x * scale, y * scale, wide * scale, high * scale, [0x48, 0x18, 0x18, 255]);
            let filled = wide * now.max(0) / most.max(1);
            let angry = self.boss.as_ref().is_some_and(|fight| fight.angered);
            let colour = if angry { [0xF0, 0x80, 0x20, 255] } else { [0xE0, 0x30, 0x30, 255] };
            world.fill(x * scale, y * scale, filled * scale, high * scale, colour);
            world.fill(x * scale, y * scale, filled * scale, scale, [0xF8, 0xA0, 0x90, 255]);
            // (Where its phases come: a mark each on the gauge, dim once played.)
            if let Some(fight) = &self.boss {
                for (at, phase) in fight.says.phases.iter().enumerate() {
                    let mark = x + wide * phase.share / 100;
                    let colour = if fight.played.contains(&at) { [0x50, 0x50, 0x58, 255] } else { [0xF8, 0xF8, 0xF8, 255] };
                    world.fill(mark * scale, (y - 1) * scale, scale, (high + 2) * scale, colour);
                }
            }
        }
        // (A phase comes: a white flash.)
        if self.flash > 0 {
            world.fill(0, 0, world.wide as i32, world.high as i32, [0xFF, 0xFF, 0xFF, (self.flash * 12).min(200) as u8]);
        }
        // (A roar, a quake: the picture shaken from side to side.)
        if self.shake > 0 {
            let push = [3, -3, 2, -2, 1, -1][self.shake as usize % 6] * scale * (self.shake as i32).min(20) / 20;
            let row = world.wide * 4;
            for line in world.rgba.chunks_mut(row) {
                let by = (push.unsigned_abs() as usize * 4).min(row);
                if push > 0 {
                    line.rotate_right(by);
                } else {
                    line.rotate_left(by);
                }
            }
        }
    }

    /// Who goes through: the species of the leader and of its partner, the leader's level.
    pub fn team(&self) -> (u16, u16, i32) {
        let team = self.run.team();
        (team.species.0, team.species.1, team.leader.stats.level)
    }

    /// The foes standing on the floor: their names, and their life (now, at most).
    pub fn foes(&self) -> Vec<(String, i32, i32)> {
        use rules::run::Side;
        self.run
            .dungeon()
            .map(|dungeon| {
                dungeon
                    .monsters
                    .iter()
                    .filter(|monster| monster.side == Side::Foe && monster.alive())
                    .map(|monster| (monster.name.clone(), monster.stats.hp, monster.stats.max_hp))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The floor it is on (from 1), and how many there are.
    pub fn floor(&self) -> Option<(usize, usize)> {
        Some((self.run.dungeon()?.floor_number as usize, self.floors))
    }

    /// On to the next floor at once (a developer's option, `floors`).
    pub fn skip_floor(&mut self) {
        if self.ended.is_none() && self.held_end.is_none() {
            self.run.skip_floor();
        }
    }

    /// The team down at once (see `Run::faint_team`): it faints, and the
    /// dungeon is lost, as when the last blow falls.
    pub fn faint_team(&mut self) {
        if self.ended.is_none() && self.held_end.is_none() {
            self.run.faint_team();
        }
    }

    /// The interface drawn over both screens (for whoever shows both).
    pub fn canvas(&self) -> &Canvas {
        &self.frame.canvas
    }
}
