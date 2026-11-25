//! The story's guests: monsters that join the team for a dungeon of the
//! story (Bidoof, Chatot, Grovyle...), and the rules some of those
//! dungeons have, as `guests.txt` lists them (`rom_formats::guests`).
//!
//! As the game, they join on a visit the story makes (`enter_story_dungeon`),
//! never on one the team chose; Hidden Land's rules (no change of leader)
//! hold there then, or always where the game keeps them after the dungeon
//! is cleared.
// (The guests that join only while the story's `SIDE01_BOSS2ND` is 0: the
// rules do not know the story's variables, they join every story visit.)

/// A guest: a monster as the story makes it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Guest {
    pub monster: u16,
    pub level: i32,
    pub hp: i32,
    pub attack: i32,
    pub sp_attack: i32,
    pub defense: i32,
    pub sp_defense: i32,
    pub iq: i32,
    /// The moves it knows (none left out: 0).
    pub moves: Vec<u16>,
}

/// What a dungeon of the story does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoryDungeon {
    /// The guests who join, by their place in the list.
    pub guests: Vec<usize>,
    /// Hidden Land's rules on the story's visit, and on any visit after.
    pub hidden_land: bool,
    pub hidden_land_cleared: bool,
    /// No recruiting on the story's visit.
    pub no_recruiting: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Guests {
    pub guests: Vec<Guest>,
    /// By dungeon, those that do something.
    pub dungeons: Vec<(u16, StoryDungeon)>,
}

impl Guests {
    /// From the text of `guests.txt` (lines it cannot read are skipped).
    pub fn read(text: &str) -> Self {
        let mut read = Self::default();
        for line in text.lines().filter(|line| !line.starts_with('#')) {
            let fields: Vec<&str> = line.split('\t').collect();
            let number = |at: usize| fields.get(at).and_then(|field| field.parse::<i32>().ok());
            match fields.first().copied() {
                Some("guest") if fields.len() >= 13 => {
                    let Some(stats) = (4..12).map(number).collect::<Option<Vec<i32>>>() else {
                        continue;
                    };
                    let index = number(1).unwrap_or(-1);
                    if index != read.guests.len() as i32 {
                        continue;
                    }
                    let moves = fields[12].split(',').filter_map(|id| id.parse::<u16>().ok()).filter(|&id| id != 0).collect();
                    read.guests.push(Guest {
                        monster: number(2).unwrap_or(0) as u16,
                        level: stats[0],
                        hp: stats[1],
                        attack: stats[2],
                        sp_attack: stats[3],
                        defense: stats[4],
                        sp_defense: stats[5],
                        iq: stats[6],
                        moves,
                    });
                }
                Some("dungeon") if fields.len() >= 5 => {
                    let Some(number) = number(1) else {
                        continue;
                    };
                    let flags: Vec<&str> = fields[4].split(',').collect();
                    let guests = fields[2..4].iter().filter_map(|field| field.parse::<usize>().ok()).collect();
                    read.dungeons.push((
                        number as u16,
                        StoryDungeon {
                            guests,
                            hidden_land: flags.contains(&"hidden_land"),
                            hidden_land_cleared: flags.contains(&"hidden_land_cleared"),
                            no_recruiting: flags.contains(&"no_recruiting"),
                        },
                    ));
                }
                _ => {}
            }
        }
        read
    }

    /// What a dungeon of the story does, if anything.
    pub fn dungeon(&self, id: u16) -> Option<&StoryDungeon> {
        self.dungeons.iter().find(|(number, _)| *number == id).map(|(_, dungeon)| dungeon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guests_and_their_dungeons_are_read() {
        let text = "# guest <index> ...\n\
            guest\t0\t399\tBidoof\t9\t36\t13\t10\t11\t9\t1\t0\t33,39,0,0\t8\t1\n\
            guest\t1\t252\tGrovyle\t42\t120\t40\t33\t30\t28\t100\t0\t1,2,3,4\t62\t1\n\
            dungeon\t8\t0\t-\t-\n\
            dungeon\t62\t1\t-\thidden_land,no_recruiting\n";
        let guests = Guests::read(text);
        assert_eq!(guests.guests.len(), 2);
        let bidoof = &guests.guests[0];
        assert_eq!((bidoof.monster, bidoof.level, bidoof.hp, bidoof.sp_defense, bidoof.iq), (399, 9, 36, 9, 1));
        assert_eq!(bidoof.moves, [33, 39]);
        assert_eq!(guests.dungeon(8).map(|dungeon| dungeon.guests.clone()), Some(vec![0]));
        let hidden = guests.dungeon(62).expect("Hidden Land");
        assert!(hidden.hidden_land && hidden.no_recruiting && !hidden.hidden_land_cleared);
        assert_eq!(guests.dungeon(9), None);
    }
}
