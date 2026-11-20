//! Quicksave of a dungeon run, in this project's own text format (it is not
//! compatible with the original game's save data).
//!
//! The floor itself is not stored: it is regenerated from the RNG state it
//! was created with, so loading restarts the current floor with the saved
//! leader, bag and counters.

use std::collections::HashMap;

use super::combat::Stats;

const HEADER: &str = "dungeon_save";
const VERSION: &str = "1";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SaveData {
    /// RNG state the current floor was generated from.
    pub floor_seed: u32,
    pub floor_number: u32,
    pub turn: u32,
    pub stats: Stats,
    /// PP left in each move slot.
    pub pp: Vec<i32>,
    pub bag: Vec<u16>,
    /// Belly in 16.16 fixed point.
    pub belly: i32,
    pub sleep_turns: i32,
    pub poisoned: bool,
    /// Ids of the leader's moves, in slot order (empty in older saves: the
    /// species' starting moves are kept).
    pub moves: Vec<u16>,
    /// Experience points (absent in older saves).
    pub exp: Option<i32>,
    /// The partner, when it is in the run.
    pub partner: Option<PartnerSave>,
    /// The others of the team, past the leader and its partner.
    pub extras: Vec<PartnerSave>,
    /// Poké carried (0 in older saves).
    pub money: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PartnerSave {
    pub stats: Stats,
    pub moves: Vec<u16>,
    pub pp: Vec<i32>,
    pub exp: i32,
}

fn stats_lines(prefix: &str, s: &Stats) -> Vec<String> {
    vec![
        format!("{prefix}level={}", s.level),
        format!("{prefix}hp={}", s.hp),
        format!("{prefix}max_hp={}", s.max_hp),
        format!("{prefix}attack={}", s.attack),
        format!("{prefix}defense={}", s.defense),
        format!("{prefix}sp_attack={}", s.sp_attack),
        format!("{prefix}sp_defense={}", s.sp_defense),
        format!("{prefix}types={},{}", s.types.0, s.types.1),
        format!("{prefix}abilities={},{}", s.abilities[0], s.abilities[1]),
        format!("{prefix}held={}", s.held),
        format!("{prefix}iq={},{},{}", s.iq, s.iq_group, s.iq_skills),
        format!("{prefix}female={}", s.female),
    ]
}

fn join<T: ToString>(values: &[T]) -> String {
    values.iter().map(T::to_string).collect::<Vec<_>>().join(",")
}

impl SaveData {
    pub fn to_text(&self) -> String {
        let mut lines = vec![
            format!("{HEADER}={VERSION}"),
            format!("floor_seed={}", self.floor_seed),
            format!("floor_number={}", self.floor_number),
            format!("turn={}", self.turn),
        ];
        lines.extend(stats_lines("", &self.stats));
        lines.extend([
            format!("pp={}", join(&self.pp)),
            format!("bag={}", join(&self.bag)),
            format!("belly={}", self.belly),
            format!("sleep_turns={}", self.sleep_turns),
            format!("poisoned={}", self.poisoned),
            format!("moves={}", join(&self.moves)),
            format!("money={}", self.money),
        ]);
        if let Some(exp) = self.exp {
            lines.push(format!("exp={exp}"));
        }
        if let Some(partner) = &self.partner {
            lines.extend(stats_lines("partner_", &partner.stats));
            lines.extend([
                format!("partner_moves={}", join(&partner.moves)),
                format!("partner_pp={}", join(&partner.pp)),
                format!("partner_exp={}", partner.exp),
            ]);
        }
        for (at, extra) in self.extras.iter().enumerate() {
            lines.extend(stats_lines(&format!("extra{at}_"), &extra.stats));
            lines.extend([
                format!("extra{at}_moves={}", join(&extra.moves)),
                format!("extra{at}_pp={}", join(&extra.pp)),
                format!("extra{at}_exp={}", extra.exp),
            ]);
        }
        lines.join("\n")
    }

    /// `None` for anything that is not a complete save of this version.
    pub fn from_text(text: &str) -> Option<Self> {
        let fields: HashMap<&str, &str> = text.lines().filter_map(|line| line.trim().split_once('=')).collect();
        if *fields.get(HEADER)? != VERSION {
            return None;
        }
        fn number<T: std::str::FromStr>(fields: &HashMap<&str, &str>, key: &str) -> Option<T> {
            fields.get(key)?.parse().ok()
        }
        fn list<T: std::str::FromStr>(fields: &HashMap<&str, &str>, key: &str) -> Option<Vec<T>> {
            let value = fields.get(key)?;
            value.split(',').filter(|part| !part.is_empty()).map(|part| part.parse().ok()).collect()
        }
        fn stats(fields: &HashMap<&str, &str>, prefix: &str) -> Option<Stats> {
            let key = |name: &str| format!("{prefix}{name}");
            let types: Vec<u8> = list(fields, &key("types"))?;
            Some(Stats {
                level: number(fields, &key("level"))?,
                hp: number(fields, &key("hp"))?,
                max_hp: number(fields, &key("max_hp"))?,
                attack: number(fields, &key("attack"))?,
                defense: number(fields, &key("defense"))?,
                // Saves made before the special stats existed lack them.
                sp_attack: number(fields, &key("sp_attack")).or(number(fields, &key("attack")))?,
                sp_defense: number(fields, &key("sp_defense")).or(number(fields, &key("defense")))?,
                types: (*types.first()?, *types.get(1)?),
                // (Saves made before abilities have none.)
                abilities: match list::<u8>(fields, &key("abilities")).as_deref() {
                    Some(&[first, second]) => [first, second],
                    _ => [0, 0],
                },
                // (Saves made before held items and IQ have neither.)
                held: number(fields, &key("held")).unwrap_or(0),
                iq: list::<u128>(fields, &key("iq")).and_then(|iq| Some(*iq.first()? as i32)).unwrap_or(crate::dungeon::iq::STARTING_IQ),
                iq_group: list::<u128>(fields, &key("iq")).and_then(|iq| Some(*iq.get(1)? as u8)).unwrap_or(0),
                iq_skills: list::<u128>(fields, &key("iq")).and_then(|iq| iq.get(2).copied()).unwrap_or(0),
                // (Saves made before it have no gender: not female.)
                female: number(fields, &key("female")).unwrap_or(false),
            })
        }
        // A save without a partner has none of its lines.
        let partner = stats(&fields, "partner_").and_then(|stats| {
            Some(PartnerSave { stats, moves: list(&fields, "partner_moves")?, pp: list(&fields, "partner_pp")?, exp: number(&fields, "partner_exp")? })
        });
        let extra = |at: usize| {
            Some(PartnerSave {
                stats: stats(&fields, &format!("extra{at}_"))?,
                moves: list(&fields, &format!("extra{at}_moves"))?,
                pp: list(&fields, &format!("extra{at}_pp"))?,
                exp: number(&fields, &format!("extra{at}_exp"))?,
            })
        };
        Some(Self {
            extras: (0..4).map_while(extra).collect(),
            floor_seed: number(&fields, "floor_seed")?,
            floor_number: number(&fields, "floor_number")?,
            turn: number(&fields, "turn")?,
            stats: stats(&fields, "")?,
            pp: list(&fields, "pp")?,
            bag: list(&fields, "bag")?,
            belly: number(&fields, "belly")?,
            sleep_turns: number(&fields, "sleep_turns")?,
            poisoned: number(&fields, "poisoned")?,
            moves: list(&fields, "moves").unwrap_or_default(),
            exp: number(&fields, "exp"),
            partner,
            money: number(&fields, "money").unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SaveData {
        SaveData {
            floor_seed: 0xDEAD_BEEF,
            floor_number: 3,
            turn: 412,
            stats: Stats {
                level: 7,
                max_hp: 34,
                hp: 12,
                attack: 15,
                defense: 11,
                sp_attack: 17,
                sp_defense: 9,
                types: (4, 8),
                abilities: [0, 0],
                held: 0,
                iq: 1,
                iq_group: 0,
                iq_skills: 0,
                female: false,
            },
            pp: vec![10, 0, 17],
            bag: vec![70, 109, 109],
            belly: 55 << 16,
            sleep_turns: 2,
            poisoned: true,
            moves: vec![33, 45, 73],
            exp: Some(431),
            money: 1250,
            partner: Some(PartnerSave {
                stats: Stats {
                    level: 6,
                    max_hp: 30,
                    hp: 30,
                    attack: 12,
                    defense: 8,
                    sp_attack: 14,
                    sp_defense: 10,
                    types: (5, 0),
                    abilities: [0, 0],
                    held: 0,
                    iq: 1,
                    iq_group: 0,
                    iq_skills: 0,
                    female: false,
                },
                moves: vec![84, 39],
                pp: vec![3, 20],
                exp: 250,
            }),
            extras: Vec::new(),
        }
    }

    #[test]
    fn round_trips() {
        let save = sample();
        assert_eq!(SaveData::from_text(&save.to_text()), Some(save));
        // Empty lists survive too.
        let empty = SaveData { pp: vec![], bag: vec![], ..sample() };
        assert_eq!(SaveData::from_text(&empty.to_text()), Some(empty));
        // A leader alone, and a save from before moves and experience were
        // kept.
        let alone = SaveData { partner: None, extras: Vec::new(), ..sample() };
        let text = alone.to_text();
        assert_eq!(SaveData::from_text(&text), Some(alone));
        let old: Vec<&str> = text.lines().filter(|line| !line.starts_with("moves=") && !line.starts_with("exp=")).collect();
        let old = SaveData::from_text(&old.join("\n")).unwrap();
        assert_eq!((old.moves, old.exp, old.partner), (vec![], None, None));
    }

    #[test]
    fn rejects_damaged_or_foreign_files() {
        let text = sample().to_text();
        assert_eq!(SaveData::from_text(""), None);
        assert_eq!(SaveData::from_text(&text.replace("turn=412", "turn=soon")), None);
        assert_eq!(SaveData::from_text(&text.replace("belly=", "bely=")), None);
        assert_eq!(SaveData::from_text(&text.replace("dungeon_save=1", "dungeon_save=2")), None);
    }
}
