//! One catalog of every Pokémon the project can draw: the game's, the
//! community's imported from SpriteCollab (the species past the game's,
//! 2000 and on, and their forms, 5000 and on), the variants painted by
//! hand, and SpriteCollab's folders not imported yet. Each species once, in
//! the Pokédex's order, its forms under it (the story's own, Mama under
//! Wigglytuff, its variants); sought by name (accents and `_` left out), by
//! Pokédex number, by type, by generation. The Resources' Pokémon shelf and
//! the Film's Assets panel both show it.

use std::collections::{BTreeMap, HashMap, HashSet};

use film::words::plain;

/// The last of each generation, in the Pokédex.
const LAST_OF_EACH: [i64; 9] = [151, 251, 386, 493, 649, 721, 809, 905, 1025];
/// The game's second gender: its monsters this many further on.
const SECOND: i64 = 600;
/// The first number of the species past the game's, of the variants.
const COMMUNITY: i64 = 2000;
const VARIANTS: i64 = 5000;

/// Where a Pokémon's pictures come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    /// The game's own (its sprites, perhaps with the community's animations added).
    Game,
    /// Imported from SpriteCollab.
    Community,
    /// Painted by hand, or imported from a SpriteBot folder.
    Hand,
    /// On SpriteCollab, not imported yet: imported when first used.
    Missing,
}

impl Source {
    /// Its badge on a card (none for the game's own).
    pub fn badge(self) -> Option<&'static str> {
        match self {
            Source::Game => None,
            Source::Community => Some("community"),
            Source::Hand => Some("hand-made"),
            Source::Missing => Some("community · not imported"),
        }
    }
}

/// A Pokémon a film can name: a species itself or one of its forms.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// What a film writes (`eevee`, `eevee_alternate_two`, `mama`).
    pub word: String,
    /// Its name shown: the species' (`Eevee`), or the species' and the form's (`Eevee · Alternate Two`).
    pub name: String,
    /// The form's own name (empty: the species itself).
    pub form: String,
    /// Its monster number (its pictures' files); -1: not imported yet.
    pub number: i64,
    pub source: Source,
    /// SpriteCollab's folder, if it comes from one not imported yet.
    pub unit: Option<(u32, u32)>,
}

/// A species and its forms, the species first.
#[derive(Clone, Debug, PartialEq)]
pub struct Species {
    /// Its Pokédex number (0: one of the game's that is no Pokémon, a decoy, a statue).
    pub dex: i64,
    pub name: String,
    /// Its generation (0: none).
    pub generation: u8,
    /// Its types, by the game's numbers (`rules::dungeon::TYPE_NAMES`), where the game's tables say.
    pub types: Vec<u8>,
    pub entries: Vec<Entry>,
}

impl Species {
    /// Its forms: those after the species itself.
    pub fn forms(&self) -> usize {
        self.entries.len().saturating_sub(1)
    }

    /// Its types' names (`Fire / Flying`).
    pub fn types_said(&self) -> String {
        self.types.iter().map(|kind| rules::dungeon::TYPE_NAMES.get(usize::from(*kind)).copied().unwrap_or("?")).collect::<Vec<_>>().join(" / ")
    }
}

/// What the catalog is made of, read from the project's files (`of_session`) or written by a test.
#[derive(Clone, Debug, Default)]
pub struct Input {
    /// Each monster drawn (with a sprite or a portrait): its words, its name.
    pub monsters: Vec<(i64, Vec<String>, String)>,
    /// The variants: number, of which monster, word, and whether painted by hand.
    pub variants: Vec<(i64, i64, String, bool)>,
    /// Pokédex numbers, by monster.
    pub dex: HashMap<i64, i64>,
    /// Types, by monster.
    pub types: HashMap<i64, (u8, u8)>,
    /// SpriteCollab's folders not imported yet.
    pub missing: Vec<spritecollab::Missing>,
}

/// Every Pokémon, by species.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Catalog {
    pub species: Vec<Species>,
    /// Where each word is: its species, its place among them.
    at: HashMap<String, (usize, usize)>,
}

/// A word made comparable: small letters, no accents, no `_`, `-` nor spaces.
pub fn key(said: &str) -> String {
    plain(said).replace(['_', '-'], "")
}

/// A form's name of its word, past its species' word (`eevee_alternate_two`: `Alternate Two`).
fn form_of_word(word: &str, species_word: &str) -> String {
    let rest = word.strip_prefix(species_word).map_or(word, |rest| rest.trim_start_matches('_'));
    let rest = if rest.is_empty() { word } else { rest };
    rest.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut letters = part.chars();
            letters.next().map(|first| first.to_uppercase().chain(letters).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The word a monster is best named by: one not ending in its number (`dialga_primal`, not `dialga_552`), the shortest.
fn best_word(words: &[String]) -> Option<String> {
    words.iter().min_by_key(|word| (word.ends_with(|letter: char| letter.is_ascii_digit()), word.len(), (*word).clone())).cloned()
}

/// A Pokédex number's generation.
pub fn generation_of(dex: i64) -> u8 {
    if dex <= 0 {
        return 0;
    }
    LAST_OF_EACH.iter().position(|last| dex <= *last).map_or(LAST_OF_EACH.len() as u8 + 1, |at| at as u8 + 1)
}

impl Catalog {
    /// The catalog of what is drawn.
    pub fn build(input: &Input) -> Catalog {
        let dex_of =
            |number: i64| -> i64 { input.dex.get(&number).copied().unwrap_or(if (COMMUNITY..VARIANTS).contains(&number) { number - COMMUNITY } else { 0 }) };
        // (Each group: a Pokédex number, or one of the game's that is no Pokémon, by its own number below zero.)
        let mut groups: BTreeMap<i64, Vec<Entry>> = BTreeMap::new();
        let variant_numbers: HashSet<i64> = input.variants.iter().map(|(number, ..)| *number).collect();
        for (number, words, name) in &input.monsters {
            if variant_numbers.contains(number) || *number >= VARIANTS || name.is_empty() || name.starts_with('?') || name.starts_with("reserve") {
                continue;
            }
            // (A word ending in a number a film reads as a second of the same kind: those (the game's second genders,
            // its story's doubles) cannot be named.)
            let Some(word) = best_word(words).filter(|word| !word.ends_with(|letter: char| letter.is_ascii_digit())) else { continue };
            let dex = dex_of(*number);
            let source = if *number >= COMMUNITY { Source::Community } else { Source::Game };
            let group = if dex > 0 { dex } else { -*number };
            groups.entry(group).or_default().push(Entry { word, name: name.clone(), form: String::new(), number: *number, source, unit: None });
        }
        for (number, of, word, by_hand) in &input.variants {
            let dex = dex_of(*of);
            let group = if dex > 0 { dex } else { -*of };
            let source = if *by_hand { Source::Hand } else { Source::Community };
            groups.entry(group).or_default().push(Entry { word: word.clone(), name: String::new(), form: String::new(), number: *number, source, unit: None });
        }
        for missing in &input.missing {
            let dex = i64::from(missing.unit.0);
            let entry = if missing.unit.1 == 0 {
                // (A species of the game's already there: its folder only adds animations to it.)
                if groups.contains_key(&dex) {
                    continue;
                }
                Entry {
                    word: plain(&missing.species),
                    name: missing.species.clone(),
                    form: String::new(),
                    number: -1,
                    source: Source::Missing,
                    unit: Some(missing.unit),
                }
            } else {
                let form = missing.form.replace('_', " ");
                Entry { word: missing.word.clone(), name: String::new(), form, number: -1, source: Source::Missing, unit: Some(missing.unit) }
            };
            groups.entry(dex).or_default().push(entry);
        }
        let mut species: Vec<Species> = Vec::new();
        // (Those with a Pokédex number first, in its order; the game's others after, by their numbers.)
        let order: Vec<i64> = groups.keys().filter(|group| **group > 0).chain(groups.keys().filter(|group| **group <= 0).rev()).copied().collect();
        for group in order {
            let Some(entries) = groups.remove(&group) else { continue };
            species.push(Self::species_of(group.max(0), entries, &input.types));
        }
        let mut catalog = Catalog { species, at: HashMap::new() };
        catalog.index();
        catalog
    }

    /// A species of its entries: the species itself first (the first of the game's, else of the community's), its forms named.
    fn species_of(dex: i64, mut entries: Vec<Entry>, types: &HashMap<i64, (u8, u8)>) -> Species {
        let head_rank = |entry: &Entry| {
            let own = entry.form.is_empty() && !entry.name.is_empty();
            let plain_number = entry.number >= 0 && (entry.number < SECOND || (COMMUNITY..VARIANTS).contains(&entry.number));
            (!own, !plain_number, entry.source, entry.number)
        };
        entries.sort_by_key(head_rank);
        let head_name = entries.first().map(|entry| if entry.name.is_empty() { entry.form.clone() } else { entry.name.clone() }).unwrap_or_default();
        let head_word = entries.first().map(|entry| entry.word.clone()).unwrap_or_default();
        for (at, entry) in entries.iter_mut().enumerate() {
            if at == 0 {
                entry.name = head_name.clone();
                entry.form.clear();
                continue;
            }
            if entry.form.is_empty() {
                entry.form = if entry.number >= VARIANTS || entry.name.is_empty() {
                    form_of_word(&entry.word, &head_word)
                } else if key(&entry.name) != key(&head_name) {
                    entry.name.clone()
                } else {
                    form_of_word(&entry.word, &head_word)
                };
            }
            entry.name = format!("{head_name} · {}", entry.form);
        }
        // (The species itself, then the game's forms, the community's, the hand-made, those not imported; by number.)
        let head = entries.remove(0);
        entries.sort_by_key(|entry| (entry.source, entry.number < 0, entry.number, entry.word.clone()));
        entries.insert(0, head);
        let kinds = entries.iter().filter_map(|entry| types.get(&entry.number)).find(|kinds| kinds.0 != 0);
        let mut types: Vec<u8> = kinds.map_or(Vec::new(), |(first, second)| [*first, *second].into_iter().filter(|kind| *kind != 0).collect());
        types.dedup();
        Species { dex, name: head_name, generation: generation_of(dex), types, entries }
    }

    fn index(&mut self) {
        self.at.clear();
        for (species_at, species) in self.species.iter().enumerate() {
            for (entry_at, entry) in species.entries.iter().enumerate() {
                self.at.entry(entry.word.clone()).or_insert((species_at, entry_at));
            }
        }
    }

    /// Where a word is: its species, its place there.
    pub fn find(&self, word: &str) -> Option<(usize, usize)> {
        self.at.get(word).copied()
    }

    pub fn entry(&self, word: &str) -> Option<(&Species, &Entry)> {
        let (species, entry) = self.find(word)?;
        let species = &self.species[species];
        Some((species, &species.entries[entry]))
    }

    /// How many entries in all.
    pub fn len(&self) -> usize {
        self.species.iter().map(|species| species.entries.len()).sum()
    }

    /// What answers a search, the closest first: each species found, the
    /// entries of it that answer (all of them when the species itself does).
    pub fn search(&self, said: &str) -> Vec<Found> {
        let query = Query::of(said);
        let mut found: Vec<Found> = Vec::new();
        for (at, species) in self.species.iter().enumerate() {
            if !query.fits_species(species) {
                continue;
            }
            let entries: Vec<usize> = (0..species.entries.len()).filter(|entry| query.fits_names(species, &species.entries[*entry])).collect();
            if entries.is_empty() {
                continue;
            }
            let whole = entries.first() == Some(&0);
            let rank = query.rank(species);
            found.push(Found { species: at, entries: if whole { (0..species.entries.len()).collect() } else { entries }, whole, rank });
        }
        found.sort_by_key(|found| found.rank);
        found
    }
}

/// A species found by a search: which, the entries of it found, whether the species itself answers, how close.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub species: usize,
    pub entries: Vec<usize>,
    pub whole: bool,
    pub rank: u8,
}

/// A search read: its words, a Pokédex number, a generation, types.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Query {
    pub words: Vec<String>,
    pub dex: Option<i64>,
    pub generation: Option<u8>,
    pub types: Vec<u8>,
}

impl Query {
    pub fn of(said: &str) -> Query {
        let mut query = Query::default();
        let pieces: Vec<String> = said.split([' ', ',', ';']).map(|piece| piece.trim().to_lowercase()).filter(|piece| !piece.is_empty()).collect();
        let mut at = 0;
        while at < pieces.len() {
            let piece = pieces[at].trim_start_matches('#');
            let generation = ["generation", "gen.", "gen", "g"].iter().find_map(|head| piece.strip_prefix(head)).map(|rest| rest.trim_start_matches('.'));
            match generation {
                // (`gen 8`, `gen8`, `g8`.)
                Some("") if pieces.get(at + 1).is_some_and(|next| next.parse::<u8>().is_ok()) && piece != "g" => {
                    query.generation = pieces[at + 1].parse().ok();
                    at += 1;
                }
                Some(number) if !number.is_empty() && number.parse::<u8>().is_ok() => query.generation = number.parse().ok(),
                _ if !piece.is_empty() && piece.chars().all(|letter| letter.is_ascii_digit()) => query.dex = piece.parse().ok(),
                _ => match rules::dungeon::TYPE_NAMES.iter().position(|name| key(name) == key(piece)).filter(|kind| *kind != 0 && *kind != 18) {
                    Some(kind) => query.types.push(kind as u8),
                    None => {
                        let word = key(piece);
                        if !word.is_empty() {
                            query.words.push(word);
                        }
                    }
                },
            }
            at += 1;
        }
        query
    }

    /// Whether a species answers the number, the generation and the types asked.
    fn fits_species(&self, species: &Species) -> bool {
        self.dex.is_none_or(|dex| species.dex == dex)
            && self.generation.is_none_or(|generation| species.generation == generation)
            && self.types.iter().all(|kind| species.types.contains(kind))
    }

    /// Whether an entry answers the words asked (its name, its word, its species').
    fn fits_names(&self, species: &Species, entry: &Entry) -> bool {
        let said = [key(&entry.name), key(&entry.word), key(&species.name)];
        self.words.iter().all(|word| said.iter().any(|name| name.contains(word.as_str())))
    }

    /// How close a species is: its name the word asked (0), beginning with it (1), else (2).
    fn rank(&self, species: &Species) -> u8 {
        let Some(first) = self.words.first() else { return 0 };
        let name = key(&species.name);
        if name == *first {
            0
        } else if name.starts_with(first.as_str()) {
            1
        } else {
            2
        }
    }
}

/// What an entry is, said in a few words (`Pokédex No. 133 · Gen. 1 · Normal`, `a form of Eevee · community`).
pub fn about(species: &Species, entry: &Entry, at: usize) -> String {
    let mut said = if at == 0 {
        if species.dex > 0 { format!("Pokédex No. {} · Gen. {}", species.dex, species.generation) } else { format!("the game's, No. {}", entry.number) }
    } else {
        format!("a form of {}", species.name)
    };
    if at == 0 && !species.types.is_empty() {
        said.push_str(&format!(" · {}", species.types_said()));
    }
    if let Some(badge) = entry.source.badge() {
        said.push_str(&format!(" · {badge}"));
    }
    said
}

/// The catalog as the shelves' things: each species, then its forms, in its order.
pub fn items(catalog: &Catalog) -> Vec<super::Item> {
    catalog
        .species
        .iter()
        .enumerate()
        .flat_map(|(species_at, species)| {
            species.entries.iter().enumerate().map(move |(at, entry)| super::Item {
                shelf: super::Shelf::Pokemon,
                word: entry.word.clone(),
                label: entry.name.clone(),
                about: about(species, entry, at),
                number: entry.number,
                file: None,
                order: (species_at * 1000 + at) as i64,
            })
        })
        .collect()
}

/// The catalog of the project's files; `missing`: SpriteCollab's folders not imported yet.
pub fn of_session(session: &workshop_base::session::Session, missing: &[spritecollab::Missing]) -> Catalog {
    let tables = session.tables();
    let data = session.data();
    let sprites = super::pictures::sprite_sheets(session);
    let mut drawn: HashSet<i64> = sprites.keys().map(|(number, _)| i64::from(*number)).collect();
    drawn.extend(tables.faces_alone.keys().filter_map(|word| tables.monsters.get(word)?.0.parse::<i64>().ok()));
    drawn.extend(tables.portraits.iter().filter(|(_, faces)| faces.contains(&0)).filter_map(|(number, _)| number.parse::<i64>().ok()));
    let mut words: BTreeMap<i64, (Vec<String>, String)> = BTreeMap::new();
    let variant_words: HashSet<String> = assets::tables::variants(data).into_iter().map(|variant| plain(&variant.word)).collect();
    for (word, (number, _)) in &tables.monsters {
        let Ok(number) = number.parse::<i64>() else { continue };
        if drawn.contains(&number) && !variant_words.contains(word) {
            let kept = words.entry(number).or_insert_with(|| (Vec::new(), String::new()));
            kept.0.push(word.clone());
        }
    }
    // (The names as monsters.txt writes them: the tables give a variant its species' name.)
    for (number, name) in assets::tables::numbered(data, "monsters.txt") {
        if let Some(kept) = words.get_mut(&i64::from(number)) {
            kept.1 = name;
        }
    }
    let hand = std::path::Path::new(data).join("hand");
    let variants = assets::tables::variants(data)
        .into_iter()
        .map(|variant| (i64::from(variant.number), i64::from(variant.of), plain(&variant.word), hand.join(&variant.word).is_dir()))
        .collect();
    let dex: HashMap<i64, i64> = assets::data::text(data, "dex.txt")
        .lines()
        .filter_map(|line| {
            let (number, at) = line.split_once('\t')?;
            Some((number.trim().parse().ok()?, at.trim().parse().ok()?))
        })
        .collect();
    let types = tables.kinds.iter().filter_map(|(number, kind)| Some((number.parse().ok()?, kind.types))).collect();
    let input =
        Input { monsters: words.into_iter().map(|(number, (words, name))| (number, words, name)).collect(), variants, dex, types, missing: missing.to_vec() };
    Catalog::build(&input)
}
