//! The tests of the Inspector's sums: a kind named, what is said of a thing renamed or put away.

use super::super::Shelf;
use super::about::kind_name;
use super::actions::{renamed_said, still_written};

/// Each shelf's thing is called by its kind.
#[test]
fn a_kind_is_named() {
    assert_eq!(kind_name(Shelf::Pokemon), "Character");
    assert_eq!(kind_name(Shelf::Pieces), "Decor Element");
    assert_eq!(kind_name(Shelf::Drawn), "Object");
}

/// The films changed are counted, one or more.
#[test]
fn a_renaming_is_told() {
    assert_eq!(renamed_said("flare", 0), "renamed “flare”");
    assert_eq!(renamed_said("flare", 1), "renamed “flare”, and in 1 film");
    assert_eq!(renamed_said("flare", 3), "renamed “flare”, and in 3 films");
    assert_eq!(still_written(0), "");
    assert_eq!(still_written(1), " (1 film still writes it)");
    assert_eq!(still_written(2), " (2 films still write it)");
}
