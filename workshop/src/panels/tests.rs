//! The tests of the Decor's panels: the Hierarchy's rows, what a click
//! chooses, where an arrow goes, the name of a new group, the layers.

use std::collections::HashSet;

use super::hierarchy::{Row, chosen_after_click, new_group_name, row_after_arrow, tree_rows};
use super::layers::{layer_meaning, placeable};

/// The rows: a group's head before its first line; its lines hidden when it is folded, unless something is sought.
#[test]
fn the_tree_has_its_groups_and_their_lines() {
    let groups = vec![("trees".to_string(), 2, 6)];
    let lines = [(9, None), (5, Some(0)), (3, Some(0)), (1, None)];
    let open = tree_rows(&lines, &groups, &HashSet::new(), false);
    assert_eq!(open, vec![Row::Line(9), Row::Group(0), Row::Line(5), Row::Line(3), Row::Line(1)]);
    let folded = HashSet::from(["trees".to_string()]);
    assert_eq!(tree_rows(&lines, &groups, &folded, false), vec![Row::Line(9), Row::Group(0), Row::Line(1)]);
    assert_eq!(tree_rows(&lines, &groups, &folded, true), open, "searching: nothing folded");
}

/// A click chooses the row's things; with Ctrl or Shift, adds them, or takes them out if they were all chosen.
#[test]
fn a_click_chooses_or_adds() {
    let selection = vec![(1, 0), (2, 0)];
    assert_eq!(chosen_after_click(&selection, &[(3, 0)], false, false), vec![(3, 0)]);
    assert_eq!(chosen_after_click(&selection, &[(3, 0)], false, true), vec![(1, 0), (2, 0), (3, 0)]);
    assert_eq!(chosen_after_click(&selection, &[(2, 0)], true, true), vec![(1, 0)]);
}

/// The arrows go from row to row, stopping at the ends; a new group takes the first name free; the layers things go on.
#[test]
fn arrows_names_and_layers() {
    assert_eq!(row_after_arrow(Some(0), true, 4), 0);
    assert_eq!(row_after_arrow(Some(3), false, 4), 3);
    assert_eq!(row_after_arrow(Some(1), false, 4), 2);
    assert_eq!(row_after_arrow(None, true, 4), 0);
    assert_eq!(new_group_name(&[("group1".to_string(), 1, 3), ("group3".to_string(), 4, 6)]), "group2");
    assert!(placeable("objects") && !placeable("residents"));
    assert_eq!(layer_meaning("anything else"), "where no one walks, painted by hand");
}

/// A piece's sums: a point on the grid, its layer, a spot's word, its line moved, a resident's talk.
#[test]
fn a_piece_is_changed() {
    use super::piece::{decors_moved, layer_now, on_grid, spot_word};
    use super::resident::talk_word;
    assert_eq!(on_grid([13.0, -3.0]), [16.0, -0.0]);
    assert_eq!((layer_now(Some("over")), layer_now(Some("ground")), layer_now(None), layer_now(Some("x"))), ("over", "ground", "objects", "objects"));
    assert_eq!(spot_word("  the old  well "), Some("the_old_well".to_string()));
    assert_eq!(spot_word("   "), None);
    let text = "decor: beach\nput a 1 2\nput b 3 4\n";
    assert_eq!(decors_moved(text, 2, true), Some("decor: beach\nput b 3 4\nput a 1 2\n".to_string()));
    assert_eq!(decors_moved(text, 2, false), None, "not before the decor's line");
    assert_eq!(decors_moved(text, 3, true), None, "the last");
    assert_eq!((talk_word(None), talk_word(Some("film a_film")), talk_word(Some("shop"))), ("only speaks", "film", "shop"));
}
