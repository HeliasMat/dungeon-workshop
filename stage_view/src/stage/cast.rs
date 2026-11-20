//! Who is who on a stage: the scripts' characters as monsters, their names,
//! their pictures. The same for a film and for the game; only the team
//! differs.

use std::collections::HashMap;
use std::rc::Rc;

use engine::{Who, names_in};
use rules::tables::Tables;
use scenes::vocabulary::POKEMON;

use shell::art::Art;
use shell::images::{Frame, Sprite};

/// The hero and the partner when no team is chosen yet.
const TEAM: (u16, u16) = (1, 25);

/// A character's own sprite sheet (`character`): the file, a cell's size, how
/// many ways, the animations.
type Sheet = (String, (u16, u16), u8, Vec<shell::sheet::Cells>);

#[derive(Clone, Default)]
pub struct Cast {
    /// The scripts' characters: name and monster (0: the hero or the
    /// partner, by the name).
    entities: Vec<(String, u16)>,
    monsters: HashMap<u16, String>,
    items: HashMap<u16, String>,
    sprites: HashMap<Who, Option<Rc<Sprite>>>,
    /// The pictures of a film's own: their files, how many pictures in each, their pace.
    drawings: HashMap<u16, (String, u16, Vec<u32>)>,
    /// The characters of a film's own, from their sprite sheets (`character`):
    /// the file, a cell's size, how many ways, the animations.
    sheets: HashMap<Who, Sheet>,
    /// The team's monsters and the names given to them and to it (empty:
    /// their species', "Poképals").
    team: (u16, u16),
    names: (String, String, String),
}

impl Cast {
    /// The lists of the folder.
    pub fn open(folder: &str) -> Self {
        Cast {
            entities: assets::tables::entities(folder),
            monsters: assets::tables::numbered(folder, "monsters.txt").into_iter().collect(),
            items: assets::tables::numbered(folder, "items.txt").into_iter().collect(),
            sprites: HashMap::new(),
            drawings: HashMap::new(),
            sheets: HashMap::new(),
            team: TEAM,
            names: Default::default(),
        }
    }

    /// The team as it is now. (Those who were it change picture.)
    pub fn set_team(&mut self, team: (u16, u16), names: &(String, String, String)) {
        let team = if team.0 == 0 || team.1 == 0 { TEAM } else { team };
        if self.team != team {
            self.sprites.clear();
        }
        self.team = team;
        if self.names != *names {
            self.names = names.clone();
        }
    }

    /// Whether that character of the scripts is the hero or the partner.
    pub fn of_the_team(&self, entity: u16) -> bool {
        self.entities.get(entity as usize).is_some_and(|entity| entity.1 == 0)
    }

    /// The name of a monster.
    pub fn species_name(&self, monster: u16) -> String {
        self.monsters.get(&monster).cloned().unwrap_or_default()
    }

    /// The monster someone is, and its name.
    pub fn who(&self, who: Who) -> (u16, String) {
        let species = |species: u16| (species, self.species_name(species));
        let entity = match who {
            Who::Character(entity) => entity,
            Who::Monster(number) | Who::Another { species: number, .. } => return species(number),
            _ => return (0, String::new()),
        };
        let called = |given: &String, monster: u16| match given.is_empty() {
            true => self.species_name(monster),
            false => given.clone(),
        };
        match self.entities.get(entity as usize) {
            Some((name, 0)) if name.contains("PARTNER") || name.starts_with("ATTENDANT") || name == "ADVENTURE_NPC1" => {
                (self.team.1, called(&self.names.1, self.team.1))
            }
            Some((_, 0)) => (self.team.0, called(&self.names.0, self.team.0)),
            Some((_, monster)) => species(*monster),
            None => (0, String::new()),
        }
    }

    /// A text with the names of those it names put in (`{name:m25}`), and
    /// of the items (`{item:109}`).
    pub fn names_in(&self, text: &str) -> String {
        let team = if self.names.2.is_empty() { "Poképals" } else { &self.names.2 };
        let text =
            text.trim_start().replace("{hero}", &self.who(Who::Character(0)).1).replace("{partner}", &self.who(Who::Character(10)).1).replace("[team:]", team);
        names_in(&text, &POKEMON, |who| self.who(who).1, |item| self.items.get(&item).cloned().unwrap_or_default())
    }

    /// The picture of someone or something of the stage, shown as `looks`.
    pub fn sprite(&mut self, art: &Art, tables: &Tables, looks: Who) -> Option<Rc<Sprite>> {
        if let Some(known) = self.sprites.get(&looks) {
            return known.clone();
        }
        if let Some((file, cell, ways, anims)) = self.sheets.get(&looks) {
            let sprite = art.sheet(file, *cell, *ways, anims);
            self.sprites.insert(looks, sprite.clone());
            return sprite;
        }
        let sprite = match looks {
            Who::Item { item, .. } => tables.item_art.get(item as usize).filter(|rgba| !rgba.is_empty()).map(|rgba| {
                let still = || vec![Frame::still(16, 16, (8, 14), rgba.clone())];
                Rc::new(Sprite::whole(vec![(0..8).map(|_| still()).collect(), (0..8).map(|_| still()).collect()]))
            }),
            Who::Thing { object, .. } => art.thing(object),
            Who::Drawn { picture, .. } => self.drawings.get(&picture).and_then(|(file, cells, ticks)| art.drawn(file, *cells, ticks)),
            Who::Character(_) | Who::Monster(_) | Who::Another { .. } => art.monster(self.who(looks).0),
            _ => None,
        };
        self.sprites.insert(looks, sprite.clone());
        sprite
    }

    /// The picture drawn for showing a thing large: `lettre_grand.png` for
    /// `lettre.png`.
    pub fn large(&self, art: &Art, who: Who) -> Option<Rc<Sprite>> {
        let Who::Drawn { picture, .. } = who else {
            return None;
        };
        let (file, _, _) = self.drawings.get(&picture)?;
        let file = file.replace(".png", "_grand.png");
        art.drawn(&file, 1, &[])
    }

    /// A picture of a film's own, by its number there.
    pub fn draws(&mut self, who: Who, file: String, cells: u16, ticks: Vec<u32>) {
        if let Who::Drawn { picture, .. } = who {
            self.drawings.insert(picture, (file, cells, ticks));
        }
    }

    /// A character of a film's own, from its sprite sheet: its sprite made
    /// again from now on.
    pub fn sheet(&mut self, who: Who, file: String, cell: (u16, u16), ways: u8, anims: &[engine::SheetAnim]) {
        let anims = anims
            .iter()
            .map(|anim| shell::sheet::Cells { animation: anim.animation as usize, cells: anim.cells.clone(), ticks: anim.ticks.clone(), row: anim.row })
            .collect();
        self.sheets.insert(who, (file, cell, ways, anims));
        self.sprites.remove(&who);
    }

    /// The file of a picture of a film's own, by its number there.
    pub fn drawing_file(&self, picture: u16) -> Option<&str> {
        self.drawings.get(&picture).map(|(file, ..)| file.as_str())
    }

    /// The sprite sheet of a character of a film's own: its file, a cell's
    /// size, how many ways.
    pub fn sheet_of(&self, who: Who) -> Option<(&str, (u16, u16), u8)> {
        self.sheets.get(&who).map(|(file, cell, ways, _)| (file.as_str(), *cell, *ways))
    }

    /// A picture of a film's own, by its number there: its file, how many
    /// pictures side by side, their pace.
    pub fn drawing(&self, picture: u16) -> Option<(&str, u16, &[u32])> {
        self.drawings.get(&picture).map(|(file, cells, ticks)| (file.as_str(), *cells, ticks.as_slice()))
    }

    /// The animations of a character of a film's own, from its sprite sheet.
    pub fn sheet_cells(&self, who: Who) -> Option<&[shell::sheet::Cells]> {
        self.sheets.get(&who).map(|(.., anims)| anims.as_slice())
    }

    /// Whether it is drawn from a sprite sheet of a film's own.
    pub fn has_sheet(&self, who: Who) -> bool {
        self.sheets.contains_key(&who)
    }

    /// Lets go of the pictures no one shows (at each new place).
    pub fn forget(&mut self, art: &Art) {
        self.sprites.clear();
        art.forget_sprites();
    }
}
