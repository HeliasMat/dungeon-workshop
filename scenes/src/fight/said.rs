//! What a fight says, in words a film shares: `said.txt`.

const SAID: &str = include_str!("said.txt");

fn value(word: &str) -> &'static str {
    SAID.lines().find_map(|line| line.strip_prefix(word)?.strip_prefix(' ')).unwrap_or_default()
}

/// Someone reaches a level.
pub fn level_up(name: &str, level: i32) -> String {
    value("level").replace("{who}", name).replace("{level}", &level.to_string())
}

/// How long a line stays in the log of a fight.
pub fn log_frames() -> usize {
    value("log").parse().unwrap_or_default()
}

/// How long a line of the game stays, for as long as it is read.
pub fn read_frames(text: &str) -> usize {
    let numbers: Vec<usize> = value("reading").split(' ').filter_map(|number| number.parse().ok()).collect();
    numbers[0] + text.chars().count() * numbers[1]
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_shared_words_are_read() {
        assert_eq!(super::level_up("Krabby", 12), "Krabby grew to level 12!");
        assert_eq!(super::log_frames(), 150);
        assert_eq!(super::read_frames("abc"), 86);
    }
}
