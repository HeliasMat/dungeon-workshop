//! Words as they are compared: small letters, no accents (one table for
//! every block), and the names nearest to one not known.

/// A letter without its accent (`é` → `e`), the others as they are.
pub fn unaccented(letter: char) -> char {
    match letter {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
        'ç' => 'c',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' | 'ì' => 'i',
        'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ÿ' => 'y',
        'ñ' => 'n',
        other => other,
    }
}

/// What a word keeps besides its letters, small and without accents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    /// Its letters and digits, nothing else, `♂` and `♀` as `m` and `f`:
    /// `Nidoran♀` is `nidoranf`, `Spinda's Café` `spindascafe` (the rules'
    /// names, a dungeon's keys).
    Letters,
    /// Its letters, digits, `_` and `-`, as a film writes its words (the
    /// rest, signs and letters of other alphabets, left out: `Nidoran♀` is
    /// `nidoran`).
    Words,
    /// Its letters and digits, `_` for the rest, none at either end:
    /// `Treasure Town!` is `treasure_town` (the names of places and pieces).
    Joined,
}

/// A word as it is compared: small letters, no accents, what else it
/// keeps by `kept`.
pub fn normalise(word: &str, kept: Kept) -> String {
    let letters = word.chars().flat_map(char::to_lowercase).map(unaccented);
    match kept {
        Kept::Letters => letters
            .filter_map(|letter| match letter {
                '♂' => Some('m'),
                '♀' => Some('f'),
                letter if letter.is_alphanumeric() => Some(letter),
                _ => None,
            })
            .collect(),
        Kept::Words => letters.filter(|letter| letter.is_ascii_lowercase() || letter.is_ascii_digit() || *letter == '_' || *letter == '-').collect(),
        Kept::Joined => {
            let joined: String = letters.map(|letter| if letter.is_alphanumeric() { letter } else { '_' }).collect();
            joined.trim_matches('_').to_string()
        }
    }
}

/// How many letters to change, add or rub out to go from a word to another.
pub fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (at, letter) in a.chars().enumerate() {
        let mut before = row[0];
        row[0] = at + 1;
        for (place, other) in b.iter().enumerate() {
            let changed = before + usize::from(letter != *other);
            before = row[place + 1];
            row[place + 1] = changed.min(row[place] + 1).min(row[place + 1] + 1);
        }
    }
    row[b.len()]
}

/// How near two words are, from 0 to 1 (the same): the letters kept of the
/// longest, by [`distance`].
pub fn similarity(a: &str, b: &str) -> f64 {
    let longest = a.chars().count().max(b.chars().count()).max(1);
    1.0 - distance(a, b) as f64 / longest as f64
}

/// How alike two words are, from 0 to 1, as difflib's ratio has it, about:
/// the letters of their longest pieces in common.
pub fn alike(a: &str, b: &str) -> f64 {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    fn matched(a: &[char], b: &[char]) -> usize {
        let (mut best, mut at_a, mut at_b) = (0, 0, 0);
        for i in 0..a.len() {
            for j in 0..b.len() {
                let mut long = 0;
                while i + long < a.len() && j + long < b.len() && a[i + long] == b[j + long] {
                    long += 1;
                }
                if long > best {
                    (best, at_a, at_b) = (long, i, j);
                }
            }
        }
        if best == 0 {
            return 0;
        }
        best + matched(&a[..at_a], &b[..at_b]) + matched(&a[at_a + best..], &b[at_b + best..])
    }
    let total = a.len() + b.len();
    if total == 0 { 1.0 } else { 2.0 * matched(&a, &b) as f64 / total as f64 }
}

/// The names nearest, by how near `rank` says each is (higher nearer; none:
/// not near enough), the nearest first, in their order among the equally
/// near; each once, `most` at most.
pub fn nearest<'a>(names: impl IntoIterator<Item = &'a str>, most: usize, rank: impl Fn(&str) -> Option<f64>) -> Vec<&'a str> {
    let mut found: Vec<(f64, &str)> = names.into_iter().filter_map(|name| rank(name).map(|near| (near, name))).collect();
    found.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut kept: Vec<&str> = Vec::new();
    for (_, name) in found {
        if kept.len() < most && !kept.contains(&name) {
            kept.push(name);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_accent_of_the_old_tables_is_taken_off() {
        // (Those of the films' and the places' tables, a superset of the rules'.)
        for (written, plain) in
            [("àâäáãå", "aaaaaa"), ("ç", "c"), ("éèêë", "eeee"), ("îïíì", "iiii"), ("ôöóòõ", "ooooo"), ("ùûüú", "uuuu"), ("ÿ", "y"), ("ñ", "n")]
        {
            for kept in [Kept::Letters, Kept::Words, Kept::Joined] {
                assert_eq!(normalise(written, kept), plain, "{written} {kept:?}");
                assert_eq!(normalise(&written.to_uppercase(), kept), plain, "{written} {kept:?} in capitals");
            }
        }
    }

    #[test]
    fn names_match_as_before() {
        // (The rules': letters and digits alone, the signs of the Nidoran.)
        assert_eq!(normalise("Spinda's Café", Kept::Letters), "spindascafe");
        assert_eq!(normalise("Firefly Forest", Kept::Letters), "fireflyforest");
        assert_eq!(normalise("Nidoran♀", Kept::Letters), "nidoranf");
        assert_eq!(normalise("Nidoran♂", Kept::Letters), "nidoranm");
        // (The films': `_` and `-` kept, the signs left out, so “nidoran” is still the one of the films.)
        assert_eq!(normalise("Nidoran♀", Kept::Words), "nidoran");
        assert_eq!(normalise("Front_of-Me!", Kept::Words), "front_of-me");
        assert_eq!(normalise("Mr. Mime", Kept::Words), "mrmime");
        assert_eq!(normalise("Café", Kept::Words), "cafe");
        // (The places': `_` for the rest, none at the ends.)
        assert_eq!(normalise(" Treasure Town! ", Kept::Joined), "treasure_town");
        assert_eq!(normalise("Plage-Crique", Kept::Joined), "plage_crique");
        assert_eq!(normalise("Été", Kept::Joined), "ete");
    }

    #[test]
    fn words_near_one_another() {
        assert_eq!(distance("wlak", "walk"), 2);
        assert_eq!(distance("walk", "walk"), 0);
        assert_eq!(distance("", "ab"), 2);
        assert!((similarity("walk", "walk") - 1.0).abs() < 1e-9);
        assert!(alike("forest", "forets") > 0.5);
        let names = ["walk", "talk", "jump", "walk"];
        assert_eq!(nearest(names, 3, |name| Some(similarity("wakl", name)).filter(|near| *near >= 0.5)), ["walk"]);
        assert_eq!(nearest(names, 1, |name| Some(-(distance("tak", name) as f64))), ["talk"]);
    }
}
