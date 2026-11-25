//! The forms a dungeon and a boss fight are written with: read from their
//! files, written back.

use game::dungeon_check::{Foe, Lying, on_floors};
use rules::own::plain;

use super::{BossForm, Form, PhaseForm, Tile, floors_said};

impl PhaseForm {
    /// As a `boss_phase:` line has it (after its `:`).
    pub(super) fn read(value: &str) -> PhaseForm {
        let mut parts = value.split('|').map(str::trim);
        let mut phase = PhaseForm { share: parts.next().map_or(50, |share| share.trim_end_matches('%').trim().parse().unwrap_or(50)), ..PhaseForm::default() };
        for part in parts {
            let (word, rest) = part.split_once(' ').map_or((part, ""), |(word, rest)| (word, rest.trim()));
            match plain(word).as_str() {
                "says" => phase.says = rest.to_string(),
                "anger" => phase.fiercer = rest.trim_start_matches('+').parse().unwrap_or(1),
                "reinforcements" => phase.help = rest.split(',').filter(|one| !one.trim().is_empty()).map(leveled).collect(),
                "heal" => phase.heal = rest.trim_end_matches('%').trim().parse().unwrap_or(0),
                "weather" => phase.weather = plain(rest),
                "moves" => phase.moves = rest.split(',').map(str::trim).filter(|said| !said.is_empty()).map(str::to_string).collect(),
                "cry" => phase.cry = rest.to_string(),
                "shake" => phase.shake = true,
                _ => {}
            }
        }
        phase
    }

    /// Its `boss_phase:` line.
    fn line(&self) -> String {
        let mut parts = vec![format!("{}%", self.share)];
        if !self.says.is_empty() {
            parts.push(format!("says {}", self.says));
        }
        if self.fiercer > 0 {
            parts.push(format!("anger +{}", self.fiercer));
        }
        if !self.help.is_empty() {
            parts.push(format!("reinforcements {}", self.help.iter().map(|(species, level)| format!("{species} {level}")).collect::<Vec<_>>().join(", ")));
        }
        if self.heal > 0 {
            parts.push(format!("heal {}%", self.heal));
        }
        if !self.weather.is_empty() {
            parts.push(format!("weather {}", self.weather));
        }
        if !self.moves.is_empty() {
            parts.push(format!("moves {}", self.moves.join(", ")));
        }
        if !self.cry.is_empty() {
            parts.push(format!("cry {}", self.cry));
        }
        if self.shake {
            parts.push("shake".to_string());
        }
        format!("boss_phase: {}\n", parts.join(" | "))
    }
}

impl BossForm {
    pub(super) fn new() -> BossForm {
        BossForm {
            species: "Spearow".to_string(),
            level: 10,
            guards: Vec::new(),
            allies: Vec::new(),
            hero: String::new(),
            partner: String::new(),
            team_level: None,
            place: None,
            team: None,
            lying: Vec::new(),
            traps: Vec::new(),
            life: 3.0,
            room: (13, 9),
            shape: "room".to_string(),
            music: String::new(),
            begins: String::new(),
            falls: String::new(),
            drop: String::new(),
            moves: Vec::new(),
            phases: vec![PhaseForm { share: 50, fiercer: 1, ..PhaseForm::default() }],
            joins: false,
            cry: String::new(),
            title: String::new(),
            look: String::new(),
            look_floor: 1,
            map: None,
        }
    }

    /// What its arena has on a tile: 0 ground, 1 wall, 2 water.
    pub(super) fn tile(&self, (x, y): Tile) -> u8 {
        self.map.as_ref().and_then(|map| map.get(y)?.get(x).copied()).unwrap_or_else(|| rules::own::arena_tile(&self.shape, self.room, (x, y)))
    }

    /// A tile painted: the whole arena taken from its shape first.
    pub(super) fn paint(&mut self, (x, y): Tile, kind: u8) {
        let (wide, high) = self.room;
        if self.map.as_ref().is_none_or(|map| map.len() != high || map.iter().any(|row| row.len() != wide)) {
            let map = (0..high).map(|y| (0..wide).map(|x| self.tile((x, y))).collect()).collect();
            self.map = Some(map);
        }
        if let Some(map) = self.map.as_mut() {
            map[y][x] = kind;
        }
    }
}

/// `7 3`: a tile.
fn tile_of(said: &str) -> Option<Tile> {
    let (x, y) = said.trim().split_once([',', ' '])?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// ` @5,3` after what is placed, or nothing.
fn at_said(at: Option<Tile>) -> String {
    at.map_or(String::new(), |(x, y)| format!(" @{x},{y}"))
}

/// `Spearow 12`: a species and a level (5 if not said).
fn leveled(said: &str) -> (String, i32) {
    match said.trim().rsplit_once(' ').and_then(|(name, level)| Some((name.trim().to_string(), level.parse::<i32>().ok()?))) {
        Some(found) => found,
        None => (said.trim().to_string(), 5),
    }
}

/// The arena's shapes, as the file says them, and as the editor shows them.
pub(super) const SHAPES: [(&str, &str); 4] = [("room", "a room"), ("pillars", "four pillars"), ("pool", "water down the sides"), ("cross", "a cross")];

impl Form {
    pub(super) fn new(name: &str) -> Form {
        let foe = |species: &str, level, often| Foe { species: species.to_string(), level, often, floors: None };
        let lying = |item: &str, often| Lying { item: item.to_string(), often, floors: None };
        Form {
            head: Vec::new(),
            name: name.to_string(),
            floors: 3,
            look: "Apple Woods".to_string(),
            look_floor: 1,
            tileset: String::new(),
            music: String::new(),
            foes: vec![foe("Caterpie", 3, 3), foe("Pidgey", 4, 2)],
            items: vec![lying("Apple", 3), lying("Oran Berry", 3)],
            recruit: true,
            up: true,
            options: Default::default(),
            retyped: Vec::new(),
            team_level: None,
            rank: String::new(),
            boss: None,
            fight: String::new(),
        }
    }

    /// As its file has it (what it does not know left out).
    pub(super) fn read(text: &str) -> Form {
        let mut form = Form { foes: Vec::new(), items: Vec::new(), look: String::new(), ..Form::new("") };
        let mut begun = false;
        let mut phases: Vec<PhaseForm> = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') && !begun {
                form.head.push(line.to_string());
                continue;
            }
            let Some((key, value)) = trimmed.split('#').next().unwrap_or_default().split_once(':') else {
                continue;
            };
            begun = true;
            let (key, value) = (plain(key), value.trim());
            if form.said(&key, value, trimmed) {
                continue;
            }
            if let Some(phase) = phase_said(&key, value) {
                phases.push(phase);
                continue;
            }
            // (The boss made by the first line that is its own.)
            let had = form.boss.is_some();
            let mut boss = form.boss.take().unwrap_or_else(BossForm::new);
            if boss.said(&key, value) || had {
                form.boss = Some(boss);
            }
        }
        if let Some(boss) = form.boss.as_mut() {
            boss.phases = phases;
        }
        form
    }

    /// A line of the dungeon itself read, whether its key is one.
    fn said(&mut self, key: &str, value: &str, line: &str) -> bool {
        match key {
            "name" => self.name = value.to_string(),
            "floors" => self.floors = value.parse().unwrap_or(3),
            "look" => (self.look, self.look_floor) = look_of(value),
            "tileset" => self.tileset = value.to_string(),
            "music" => self.music = value.to_string(),
            "pokemon" => {
                for part in value.split(',').filter(|part| !part.trim().is_empty()) {
                    let (part, floors) = on_floors(part.trim());
                    let (said, often) = weighed(part);
                    let (species, level) = match said.rsplit_once(' ').and_then(|(name, level)| Some((name.to_string(), level.parse::<i32>().ok()?))) {
                        Some(found) => found,
                        None => (said, 5),
                    };
                    self.foes.push(Foe { species, level, often, floors });
                }
            }
            "items" => {
                for part in value.split(',').filter(|part| !part.trim().is_empty()) {
                    let (part, floors) = on_floors(part.trim());
                    let (item, often) = weighed(part);
                    self.items.push(Lying { item, often, floors });
                }
            }
            "recruiting" => self.recruit = yes(value),
            "upward" => self.up = yes(value),
            "rules" => self.options = self.options.or(rules::rule_options::RuleOptions::read(value).0),
            "pokemontype" | "movetype" => self.retyped.push(line.to_string()),
            "level" => self.team_level = value.parse().ok(),
            "rank" => self.rank = value.to_string(),
            "fight" => self.fight = value.to_string(),
            _ => return false,
        }
        true
    }

    /// Its file's text.
    pub(super) fn text(&self) -> String {
        let mut text: String = self.head.iter().map(|line| format!("{line}\n")).collect();
        let yes = |on: bool| if on { "yes" } else { "no" };
        let weight = |weight: u16| if weight > 1 { format!(" x{weight}") } else { String::new() };
        text.push_str(&format!("name: {}\nfloors: {}\n", self.name, self.floors));
        if !self.look.is_empty() {
            text.push_str(&format!("look: {}{}\n", self.look, if self.look_floor > 1 { format!(" {}", self.look_floor) } else { String::new() }));
        }
        if !self.tileset.is_empty() {
            text.push_str(&format!("tileset: {}\n", self.tileset));
        }
        if !self.music.is_empty() {
            text.push_str(&format!("music: {}\n", self.music));
        }
        let foes: Vec<String> = self.foes.iter().map(|foe| format!("{} {}{}{}", foe.species, foe.level, weight(foe.often), floors_said(foe.floors))).collect();
        text.push_str(&format!("pokemon: {}\n", foes.join(", ")));
        if !self.items.is_empty() {
            let items: Vec<String> = self.items.iter().map(|lying| format!("{}{}{}", lying.item, weight(lying.often), floors_said(lying.floors))).collect();
            text.push_str(&format!("items: {}\n", items.join(", ")));
        }
        text.push_str(&format!("recruiting: {}\nupward: {}\n", yes(self.recruit), yes(self.up)));
        let options = self.options.words();
        if !options.is_empty() {
            text.push_str(&format!("rules: {}\n", options.join(", ")));
        }
        for line in &self.retyped {
            text.push_str(&format!("{line}\n"));
        }
        if let Some(level) = self.team_level {
            text.push_str(&format!("level: {level}\n"));
        }
        if !self.rank.trim().is_empty() {
            text.push_str(&format!("rank: {}\n", self.rank.trim()));
        }
        if !self.fight.is_empty() {
            text.push_str(&format!("fight: {}\n", self.fight));
        }
        if let Some(boss) = &self.boss {
            text.push_str(&boss_text(boss));
        }
        text
    }

    /// A boss fight's own file: its comments, its name, its lines.
    pub(super) fn fight_text(&self) -> String {
        let mut text: String = self.head.iter().map(|line| format!("{line}\n")).collect();
        text.push_str(&format!("name: {}\n", self.name));
        if let Some(boss) = &self.boss {
            text.push_str(&boss_text(boss));
        }
        text
    }
}

/// A boss fight's lines, as a file writes them.
fn boss_text(boss: &BossForm) -> String {
    let mut text = String::new();
    {
        {
            if !boss.look.is_empty() {
                text.push_str(&format!("boss_look: {}{}\n", boss.look, if boss.look_floor > 1 { format!(" {}", boss.look_floor) } else { String::new() }));
            }
            text.push_str(&format!("boss: {} {}\n", boss.species, boss.level));
            if !boss.guards.is_empty() {
                let guards: Vec<String> = boss.guards.iter().map(|(species, level, at)| format!("{species} {level}{}", at_said(*at))).collect();
                text.push_str(&format!("boss_guards: {}\n", guards.join(", ")));
            }
            if !boss.allies.is_empty() {
                let allies: Vec<String> = boss.allies.iter().map(|(species, level, at)| format!("{species} {level}{}", at_said(*at))).collect();
                text.push_str(&format!("boss_allies: {}\n", allies.join(", ")));
            }
            if !boss.hero.is_empty() {
                text.push_str(&format!("boss_hero: {}\n", boss.hero));
            }
            if !boss.partner.is_empty() {
                text.push_str(&format!("boss_partner: {}\n", boss.partner));
            }
            if let Some(level) = boss.team_level {
                text.push_str(&format!("boss_level: {level}\n"));
            }
            if let Some((x, y)) = boss.place {
                text.push_str(&format!("boss_at: {x} {y}\n"));
            }
            if let Some((x, y)) = boss.team {
                text.push_str(&format!("boss_team_at: {x} {y}\n"));
            }
            if !boss.lying.is_empty() {
                let lying: Vec<String> = boss.lying.iter().map(|(item, at)| format!("{item}{}", at_said(Some(*at)))).collect();
                text.push_str(&format!("boss_floor_items: {}\n", lying.join(", ")));
            }
            if !boss.traps.is_empty() {
                let traps: Vec<String> =
                    boss.traps.iter().map(|(kind, at, hidden)| format!("{kind}{}{}", at_said(Some(*at)), if *hidden { " hidden" } else { "" })).collect();
                text.push_str(&format!("boss_traps: {}\n", traps.join(", ")));
            }
            if (boss.life - 1.0).abs() > f32::EPSILON {
                text.push_str(&format!("boss_hp: x{}\n", (boss.life * 10.0).round() / 10.0));
            }
            text.push_str(&format!("boss_arena: {}x{}\n", boss.room.0, boss.room.1));
            if boss.shape != "room" {
                text.push_str(&format!("boss_shape: {}\n", boss.shape));
            }
            if let Some(map) = &boss.map {
                let rows: Vec<String> = map
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|tile| match tile {
                                1 => 'X',
                                2 => '~',
                                _ => '.',
                            })
                            .collect()
                    })
                    .collect();
                text.push_str(&format!("boss_map: {}\n", rows.join("/")));
            }
            if !boss.music.is_empty() {
                text.push_str(&format!("boss_music: {}\n", boss.music));
            }
            if !boss.begins.is_empty() {
                text.push_str(&format!("boss_says: {}\n", boss.begins));
            }
            if !boss.falls.is_empty() {
                text.push_str(&format!("boss_falls: {}\n", boss.falls));
            }
            if !boss.moves.is_empty() {
                text.push_str(&format!("boss_moves: {}\n", boss.moves.join(", ")));
            }
            for phase in &boss.phases {
                text.push_str(&phase.line());
            }
            if boss.joins {
                text.push_str("boss_recruit: yes\n");
            }
            if !boss.cry.is_empty() {
                text.push_str(&format!("boss_cry: {}\n", boss.cry));
            }
            if !boss.title.is_empty() {
                text.push_str(&format!("boss_title: {}\n", boss.title));
            }
            if !boss.drop.is_empty() {
                text.push_str(&format!("boss_item: {}\n", boss.drop));
            }
        }
    }
    text
}

impl BossForm {
    /// A line of the boss read, whether its key is one.
    fn said(&mut self, key: &str, value: &str) -> bool {
        match key {
            "boss" => (self.species, self.level) = leveled(value),
            "bossguards" => self.guards = placed_leveled(value),
            "bossallies" => self.allies = placed_leveled(value),
            "bosshero" => self.hero = value.to_string(),
            "bosspartner" => self.partner = value.to_string(),
            "bosslevel" => self.team_level = value.parse().ok(),
            "bossat" => self.place = tile_of(value),
            "bossteamat" => self.team = tile_of(value),
            "bossflooritems" => {
                self.lying = rules::own::split_placed(value)
                    .iter()
                    .filter_map(|part| {
                        let (said, at) = rules::own::placed_at(part);
                        Some((said.to_string(), at?))
                    })
                    .collect();
            }
            "bosstraps" => {
                self.traps = rules::own::split_placed(value)
                    .iter()
                    .filter_map(|part| {
                        let hidden = part.trim_end().ends_with(" hidden");
                        let (said, at) = rules::own::placed_at(part.trim_end().trim_end_matches(" hidden"));
                        Some((plain(said), at?, hidden))
                    })
                    .collect();
            }
            "bosshp" => self.life = value.trim_start_matches(['x', '×']).trim().parse().unwrap_or(1.0),
            "bossarena" => {
                if let Some((wide, high)) = value.split_once(['x', '×']).and_then(|(wide, high)| Some((wide.trim().parse().ok()?, high.trim().parse().ok()?)))
                {
                    self.room = (wide, high);
                }
            }
            "bossshape" => self.shape = plain(value),
            "bossmap" => self.map = Some(value.split('/').map(map_row).collect()),
            "bossmusic" => self.music = value.to_string(),
            "bosssays" => self.begins = value.to_string(),
            "bossfalls" => self.falls = value.to_string(),
            "bossitem" => self.drop = value.to_string(),
            "bossmoves" => self.moves = value.split(',').map(str::trim).filter(|said| !said.is_empty()).map(str::to_string).collect(),
            "bosscry" => self.cry = value.to_string(),
            "bosstitle" => self.title = value.to_string(),
            "bosslook" => (self.look, self.look_floor) = look_of(value),
            "bossrecruit" => self.joins = yes(value),
            _ => return false,
        }
        true
    }
}

/// A boss's phase, of a `boss_phase:` line or of the lines of before
/// (reinforcements, heal, anger), each a phase.
fn phase_said(key: &str, value: &str) -> Option<PhaseForm> {
    match key {
        "bossreinforcements" => {
            let (share, rest) = value.split_once(' ').unwrap_or((value, ""));
            let (list, said) = rest.split_once(':').unwrap_or((rest, ""));
            let share = share.trim_end_matches('%').parse().unwrap_or(30);
            let help = list.split(',').filter(|part| !part.trim().is_empty()).map(leveled).collect();
            Some(PhaseForm { share, says: said.trim().to_string(), help, ..PhaseForm::default() })
        }
        // (Two shares said, else nothing.)
        "bossheal" => {
            let shares: Vec<i32> = value.split_whitespace().filter_map(|share| share.trim_end_matches('%').parse().ok()).collect();
            match shares[..] {
                [share, heal] => Some(PhaseForm { share, heal, ..PhaseForm::default() }),
                _ => None,
            }
        }
        "bossanger" => {
            let (fiercer, says) = match value.split_once(' ').filter(|(first, _)| first.starts_with(['+', '0'])) {
                Some((first, rest)) => (first.trim_start_matches('+').parse().unwrap_or(1), rest.trim().to_string()),
                None if value.starts_with(['+', '0']) => (value.trim_start_matches('+').parse().unwrap_or(1), String::new()),
                None => (1, value.to_string()),
            };
            Some(PhaseForm { share: 50, says, fiercer, ..PhaseForm::default() })
        }
        "bossphase" => Some(PhaseForm::read(value)),
        _ => None,
    }
}

/// `Apple Woods 3`: a look and its floor (1 if not said).
fn look_of(value: &str) -> (String, usize) {
    match value.rsplit_once(' ').and_then(|(name, floor)| Some((name, floor.parse::<usize>().ok()?))) {
        Some((name, floor)) => (name.to_string(), floor),
        None => (value.to_string(), 1),
    }
}

/// `Apple x3`: a name and how often (1 if not said).
fn weighed(part: &str) -> (String, u16) {
    match part.rsplit_once(" x").and_then(|(name, weight)| Some((name.trim().to_string(), weight.trim().parse::<u16>().ok()?))) {
        Some(found) => found,
        None => (part.trim().to_string(), 1),
    }
}

fn yes(value: &str) -> bool {
    matches!(plain(value).as_str(), "yes" | "true" | "1")
}

/// `Spearow 12 @5,3, Pidgey 8`: species, levels and where.
fn placed_leveled(value: &str) -> Vec<(String, i32, Option<Tile>)> {
    rules::own::split_placed(value)
        .iter()
        .map(|part| {
            let (said, at) = rules::own::placed_at(part);
            let (species, level) = leveled(said);
            (species, level, at)
        })
        .collect()
}

/// A row of a `boss_map:` line: `X` a wall, `~` water, the rest ground.
fn map_row(row: &str) -> Vec<u8> {
    row.trim()
        .chars()
        .map(|tile| match tile {
            'X' | 'x' => 1,
            '~' => 2,
            _ => 0,
        })
        .collect()
}
