//! The wiki searched: by a word's name first (the very word, then those it
//! begins, then those with a part of it: `rain` finds `makes_rain`), then by
//! its family, then by what it does; a name that only hides it (`train`) last.

use super::Page;

/// The words of a text as they are compared: small letters, each word apart (`Run in!` is `run`, `in`).
fn words_of(text: &str) -> Vec<String> {
    text.to_lowercase().split(|letter: char| !(letter.is_alphanumeric() || letter == '_')).filter(|word| !word.is_empty()).map(str::to_string).collect()
}

/// How well a page answers what is sought (0: not at all).
pub fn score(page: &Page, family: &str, sought: &str) -> u32 {
    let tokens = words_of(sought);
    if tokens.is_empty() {
        return 1;
    }
    let squeezed = tokens.join("_");
    let word = page.word.as_str();
    if word == squeezed {
        return 1000;
    }
    if word.starts_with(&squeezed) {
        return 800 - (word.len() - squeezed.len()).min(99) as u32;
    }
    // (A part of its name: `makes_rain`, `rocket_rain` for `rain`.)
    let parts: Vec<&str> = word.split('_').collect();
    if parts.windows(tokens.len()).any(|run| run.iter().zip(&tokens).all(|(part, token)| part == token)) {
        return 700;
    }
    if parts.iter().any(|part| part.starts_with(tokens[0].as_str())) && word.contains(&squeezed) {
        return 650 - (word.len() - squeezed.len()).min(99) as u32;
    }
    let family = words_of(family);
    if tokens.iter().all(|token| family.iter().any(|said| said.starts_with(token.as_str()))) {
        return 400;
    }
    let said = words_of(&page.explained);
    let begins = |token: &String| said.iter().any(|said| said.starts_with(token.as_str()));
    let found = tokens.iter().filter(|token| begins(token)).count() as u32;
    if found as usize == tokens.len() {
        return 200 + found;
    }
    // (Hidden in its name, not a part of it: `train` for `rain`.)
    if word.contains(&squeezed) {
        return 150;
    }
    if found > 0 {
        return 100 + found;
    }
    let forms = page.forms.iter().any(|form| form.shown.to_lowercase().contains(&squeezed));
    if forms { 50 } else { 0 }
}

/// The pages that answer what is sought, the best first (in a tie, by
/// family and name); all of them, by family and name, when nothing is.
pub fn ranked<'a>(pages: &'a [Page], families: &[&str], sought: &str) -> Vec<&'a Page> {
    let mut found: Vec<(u32, &Page)> =
        pages.iter().map(|page| (score(page, families.get(page.family).copied().unwrap_or_default(), sought), page)).filter(|(score, _)| *score > 0).collect();
    found.sort_by(|(a, one), (b, other)| b.cmp(a).then((one.family, &one.word).cmp(&(other.family, &other.word))));
    found.into_iter().map(|(_, page)| page).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(word: &str, family: usize, explained: &str) -> Page {
        Page { word: word.to_string(), family, explained: explained.to_string(), ..Page::default() }
    }

    #[test]
    fn the_name_first_then_the_family_then_what_it_does() {
        let pages = vec![
            page("run_in", 1, "Someone runs in, out of breath."),
            page("walk", 1, "Someone walks to a place."),
            page("run", 1, "Someone runs to a place."),
            page("jump", 2, "A little hop."),
            page("sky", 3, "The weather of the sky: rain, snow."),
            page("trek", 4, "The team in file behind the leader, who runs ahead."),
            page("makes_rain", 2, "Someone's move brings the clouds."),
            page("train", 1, "They walk in single file."),
        ];
        let families = ["Speech", "Movement", "Body", "Effects & Weather", "Dungeon"];
        let words = |sought: &str| ranked(&pages, &families, sought).iter().map(|page| page.word.clone()).collect::<Vec<_>>();
        assert_eq!(words("run"), ["run", "run_in", "trek"], "the very word, one it begins, then one that says it");
        assert_eq!(words("run in").first().map(String::as_str), Some("run_in"));
        assert_eq!(words("weather"), ["sky"], "by its family");
        assert_eq!(words("snow"), ["sky"], "by what it does");
        assert_eq!(words("rain"), ["makes_rain", "sky", "train"], "a part of its name, what it does, then a name that hides it");
        assert_eq!(words("hop"), ["jump"]);
        assert_eq!(words("").len(), pages.len(), "nothing sought: all of them");
        assert_eq!(words("").first().map(String::as_str), Some("run"), "by family, then name");
        assert!(words("zzz").is_empty());
    }
}
