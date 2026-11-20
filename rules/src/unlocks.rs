//! The dungeons the story does not open itself: those only a job leads to,
//! opened for good once a job there is taken, and those that follow one
//! another, each opened by clearing the one before. (Which ones and from
//! when is from knowing the game, close enough.)

use crate::jobs::GRADUATION;

/// Dungeons only jobs lead to, and the chapter from which the boards offer
/// jobs there: the meadows and forests during the guild's days, the far
/// ones once the team has graduated.
pub const JOB_ONLY: [(u16, i32); 16] = [
    (87, 8),           // Serenity River
    (89, 10),          // Lush Prairie
    (90, 10),          // Tiny Meadow
    (91, 12),          // Labyrinth Cave
    (92, 12),          // Oran Forest
    (93, GRADUATION),  // Lake Afar
    (94, GRADUATION),  // Happy Outlook
    (95, GRADUATION),  // Mt. Mistral
    (96, GRADUATION),  // Shimmer Hill
    (97, GRADUATION),  // Lost Wilderness
    (98, GRADUATION),  // Midnight Forest
    (107, GRADUATION), // Oblivion Forest
    (108, GRADUATION), // Treacherous Waters
    (109, GRADUATION), // Southeastern Islands
    (110, GRADUATION), // Inferno Cave
    (176, GRADUATION), // Armaldo's Shelter
];

/// A dungeon cleared, and the one it opens: the three caves of Aegis Cave
/// and their chambers in turn, the Zero Isle's centre after its south, then
/// the Destiny Tower, the stations of Sky Peak one by one, the
/// Star Cave after the summit.
pub const CHAINS: [(u16, u16); 18] = [
    (54, 55),
    (55, 56),
    (56, 57),
    (57, 58),
    (58, 59),
    (59, 60),
    (102, 103),
    (103, 104),
    (111, 112),
    (112, 113),
    (113, 114),
    (114, 115),
    (115, 116),
    (116, 117),
    (117, 118),
    (118, 119),
    (119, 120),
    (122, 174),
];

/// Dungeons open as soon as another is: Sky Peak's first station with
/// its summit, which the story opens.
pub const WITH: [(u16, u16); 1] = [(122, 111)];

/// What clearing that dungeon opens.
pub fn opened_by_clearing(dungeon: u16) -> Option<u16> {
    CHAINS.iter().find(|(cleared, _)| *cleared == dungeon).map(|(_, next)| *next)
}

/// The dungeons the boards may offer jobs in at that chapter, besides the
/// open ones.
pub fn job_only(chapter: i32) -> impl Iterator<Item = u16> {
    JOB_ONLY.into_iter().filter(move |(_, from)| chapter >= *from).map(|(dungeon, _)| dungeon)
}

/// All the open dungeons: the story's, then those opened by play, then those
/// that come with them; each once.
pub fn all_open(story: &[u16], opened: &[u16]) -> Vec<u16> {
    let mut open: Vec<u16> = Vec::new();
    for &dungeon in story.iter().chain(opened) {
        if !open.contains(&dungeon) {
            open.push(dungeon);
        }
    }
    for (with, also) in WITH {
        if open.contains(&with) && !open.contains(&also) {
            open.push(also);
        }
    }
    open
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dungeons_open_by_jobs_chains_and_company() {
        assert_eq!(opened_by_clearing(102), Some(103));
        assert_eq!(opened_by_clearing(1), None);
        assert!(job_only(3).next().is_none());
        assert!(job_only(GRADUATION).any(|dungeon| dungeon == 107));
        assert_eq!(all_open(&[1, 122], &[1, 87]), [1, 122, 87, 111]);
    }
}
