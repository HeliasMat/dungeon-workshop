//! Lines that made the compiler panic, or ask for more memory than there is:
//! each is now read, and what is wrong in it told.

use super::compile;

/// The film's problems, as said.
fn told(line: &str) -> Vec<String> {
    let written = compile(&format!("scene: town pikachu piplup\n{line}\n"));
    written.problems.iter().map(ToString::to_string).collect()
}

#[test]
fn lines_that_fell_over_are_read() {
    // (Each of these panicked: an index past the end, an overflow, a
    // number that is none.)
    for line in [
        "regroup pikachu",
        "pikachu move",
        "pikachu move é",
        "twinkle pikachu 1e40 nan",
        "gulls pikachu 1e40 nan",
        "gulls pikachu 2147483647 5",
        "dig pikachu piplup 2147483647",
        "burst_out pikachu 2147483647 2147483647",
        "gulls pikachu 2147483647 2147483647",
    ] {
        told(line);
    }
}

#[test]
fn too_many_is_told_not_attempted() {
    assert!(told("crowd pikachu 99999999999999").iter().any(|said| said.contains("at most")), "{:?}", told("crowd pikachu 99999999999999"));
}

#[test]
fn a_number_is_digits_not_nan_nor_too_far() {
    assert_eq!(film_core::grammar::number_in("nan"), None);
    assert_eq!(film_core::grammar::number_in("inf"), None);
    assert_eq!(film_core::grammar::number_in("1e40"), None);
    assert_eq!(film_core::grammar::coordinate_in("1e30"), None);
    assert_eq!(film_core::grammar::coordinate_in("-12.5"), Some(-12));
}
