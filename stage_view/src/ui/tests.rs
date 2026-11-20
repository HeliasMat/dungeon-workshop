use super::*;
use shell::Keys;

#[test]
fn text_is_written_out_up_to_its_waits() {
    let line = "[CS:N]Bob[CR]: hi,[K]
you![K]";
    assert_eq!(reveal(line, 0), "[CS:N]");
    assert_eq!(reveal(line, 4), "[CS:N]Bob[CR]:");
    assert_eq!(reveal(line, 100), line);
    // Waits after "Bob: hi," and at the end (line breaks do not count).
    assert_eq!(key_stops(line), [8, 12]);
    assert_eq!(key_stops("no wait"), [0usize; 0]);
}

#[test]
fn pauses_take_the_time_of_so_many_characters() {
    let line = "Erm...[W:20] no.";
    assert_eq!(reveal(line, 6), "Erm...");
    assert_eq!(reveal(line, 20), "Erm...");
    assert_eq!(reveal(line, 26), "Erm...[W:20]");
    assert_eq!(reveal(line, 27), "Erm...[W:20] ");
    assert_eq!(pause_units(line), 20);
    assert_eq!(key_stops("a[W:5]b[K]"), [7]);
    assert!(!pausing(line, 6) && pausing(line, 10) && !pausing(line, 26) && !pausing(line, 100));
}

#[test]
fn colour_tags_mark_what_they_name() {
    let art = art();
    assert_eq!(code_color(&art, "N"), color::YELLOW);
    assert_eq!(code_color(&art, "M"), color::GREEN);
    assert_eq!(code_color(&art, "W"), color::RED);
    assert_eq!(code_color(&art, "l"), color::BLUE);
    assert_eq!(code_color(&art, "?"), color::WHITE);
    assert_eq!(code_color(&art, ""), color::WHITE);
}

fn art() -> UiArt {
    // An empty font and a frame whose tiles are plain: red corners,
    // green edges.
    let font = Font::default();
    let mut frame = vec![0u8; FRAME_WIDTH * 32 * 4];
    for ty in 0..3 {
        for tx in 0..3 {
            let color = match (tx, ty) {
                (1, 1) => continue,
                (1, _) | (_, 1) => [0, 255, 0, 255],
                _ => [255, 0, 0, 255],
            };
            for dy in 0..TILE {
                for dx in 0..TILE {
                    let at = ((ty * TILE + dy) * FRAME_WIDTH + tx * TILE + dx) * 4;
                    frame[at..at + 4].copy_from_slice(&color);
                }
            }
        }
    }
    UiArt {
        font,
        frame,
        frame_green: Vec::new(),
        frame_pink: Vec::new(),
        frame_types: Vec::new(),
        title_logo: Vec::new(),
        title_sprites: Vec::new(),
        menu_art: Vec::new(),
        menu_backdrop: Vec::new(),
        title_art: Vec::new(),
        marks: vec![(2, 2, vec![9; 16])],
        hud: Vec::new(),
        backdrop: Vec::new(),
        text_colors: Vec::new(),
        // A cursor of one white pixel.
        cursor: (1, 1, vec![248, 248, 248, 255]),
        more: (0, 0, Vec::new()),
        strings: Arc::new(UiStrings { menu: vec!["Belly:[CLUM_SET:168][digits:0]".to_string()], ..UiStrings::default() }),
    }
}

fn pixel(canvas: &Canvas, x: usize, y: usize) -> [u8; 4] {
    let at = (y * SCREEN_WIDTH + x) * 4;
    [canvas.data[at], canvas.data[at + 1], canvas.data[at + 2], canvas.data[at + 3]]
}

#[test]
fn windows_are_framed_around_their_inside() {
    let art = art();
    let mut canvas = Canvas::default();
    // On the touch screen: see-through grey.
    canvas.window(&art, 2, BOTTOM + 2, 7, 12);
    let top = SCREEN_HEIGHT;
    assert_eq!(pixel(&canvas, 16, top + 16), [32, 32, 32, 128]);
    assert_eq!(pixel(&canvas, 71, top + 111), [32, 32, 32, 128]);
    // Corners and edges sit one tile outside.
    assert_eq!(pixel(&canvas, 8, top + 8), [255, 0, 0, 255]);
    assert_eq!(pixel(&canvas, 72, top + 112), [255, 0, 0, 255]);
    assert_eq!(pixel(&canvas, 40, top + 8), [0, 255, 0, 255]);
    assert_eq!(pixel(&canvas, 8, top + 60), [0, 255, 0, 255]);
    // Nothing further out.
    assert_eq!(pixel(&canvas, 7, top + 60)[3], 0);
    assert_eq!(pixel(&canvas, 100, top + 60)[3], 0);
    // On the top screen the grey is plain.
    canvas.window(&art, 2, 2, 28, 20);
    assert_eq!(pixel(&canvas, 100, 100), [32, 32, 32, 255]);
}

#[test]
fn labels_and_tags() {
    let art = art();
    assert_eq!(art.label(0), "Belly:");
    assert_eq!(art.label(5), "");
    assert_eq!(strip_tags("[CN]Controls[M:B3]+[M:B8]"), "Controls+");
}

#[test]
fn titled_menus_follow_the_measured_layout() {
    let art = art();
    let mut canvas = Canvas::default();
    let rows = [("a", color::WHITE), ("b", color::WHITE)];
    let lines = canvas.titled_menu(&art, (2, 2, 12), ("t", color::WHITE), &rows, 0, Some(1), 12);
    // Entries start 18 pixels down, 16 apart; the window is the title,
    // the entries and the foot: 8 tiles.
    assert_eq!(lines, vec![34, 50]);
    assert_eq!(pixel(&canvas, 20, 29), [248, 248, 248, 255]);
    assert_eq!(pixel(&canvas, 20, 30), [0, 0, 0, 255]);
    assert_eq!(pixel(&canvas, 20, 64), [248, 248, 248, 255]);
    assert_eq!(pixel(&canvas, 20, 80), [0, 255, 0, 255]);
    // The cursor, on the second entry: the art's picture, a column
    // right of the window's edge.
    assert_eq!(pixel(&canvas, 17, 50), [248, 248, 248, 255]);
    assert_eq!(pixel(&canvas, 17, 36)[3], 255);
    assert_eq!(canvas.cursors, vec![(16, 50)]);
    assert!(canvas.cursor_moved());
    // None while it blinks off where it already was.
    let mut hidden = Canvas { cursor_hidden: true, cursors_before: vec![(16, 50)], ..Canvas::default() };
    hidden.titled_menu(&art, (2, 2, 12), ("t", color::WHITE), &rows, 0, Some(1), 12);
    assert_ne!(pixel(&hidden, 17, 50), [248, 248, 248, 255]);
    assert!(!hidden.cursor_moved());
    // One that just moved there shows whatever the blinking.
    let mut moved = Canvas { cursor_hidden: true, cursors_before: vec![(16, 34)], ..Canvas::default() };
    moved.titled_menu(&art, (2, 2, 12), ("t", color::WHITE), &rows, 0, Some(1), 12);
    assert_eq!(pixel(&moved, 17, 50), [248, 248, 248, 255]);
    assert!(moved.cursor_moved());
}

#[test]
fn the_backdrop_is_a_lit_checkerboard() {
    let mut canvas = Canvas::default();
    canvas.backdrop(&art());
    assert_eq!(pixel(&canvas, 0, 0), [144, 128, 56, 255]);
    assert_eq!(pixel(&canvas, 1, 1), [120, 104, 48, 255]);
    assert_eq!(pixel(&canvas, 4, 0), [72, 56, 24, 255]);
    assert_eq!(pixel(&canvas, 5, 1), [96, 80, 40, 255]);
    assert_eq!(pixel(&canvas, 4, 4), [144, 128, 56, 255]);
    // Only the top screen.
    assert_eq!(pixel(&canvas, 0, SCREEN_HEIGHT)[3], 0);
}

#[test]
fn a_box_is_filled_as_its_pixels_one_by_one() {
    // (Over pixels that are there, half there and not there, cut by every edge.)
    let mut by_box = Canvas::default();
    for (at, part) in by_box.data.iter_mut().enumerate() {
        *part = (at * 7 % 256) as u8;
    }
    let mut by_pixel = Canvas { data: by_box.data.clone(), ..Canvas::default() };
    for (x, y, wide, high, colour) in
        [(-5, -3, 20, 10, [10, 200, 30, 128]), (250, 180, 30, 30, [255, 0, 0, 255]), (40, 50, 0, 9, [1, 2, 3, 255]), (9, 9, 3, 3, [0, 0, 0, 0])]
    {
        by_box.fill(x, y, wide, high, colour);
        for row in y..y + high as i32 {
            for column in x..x + wide as i32 {
                by_pixel.put(column, row, colour);
            }
        }
    }
    assert!(by_box.data == by_pixel.data);
}

fn keys(f: impl FnOnce(&mut Keys)) -> Keys {
    let mut keys = Keys::default();
    f(&mut keys);
    keys
}

#[test]
fn a_list_cursor_wraps_round_and_chooses() {
    let mut list = ListCursor::default();
    assert_eq!(list.keys(&keys(|keys| keys.up = true), 3), Picked::Moved);
    assert_eq!(list.at, 2, "up from the first: the last");
    assert_eq!(list.keys(&keys(|keys| keys.down = true), 3), Picked::Moved);
    assert_eq!(list.at, 0, "down from the last: the first");
    assert_eq!(list.keys(&keys(|keys| (keys.down, keys.up) = (true, true)), 3), Picked::Moved);
    assert_eq!(list.at, 0, "both at once: back where it was");
    assert_eq!(list.keys(&keys(|keys| (keys.down, keys.enter) = (true, true)), 3), Picked::Chosen(1));
    assert_eq!(list.keys(&keys(|keys| keys.escape = true), 3), Picked::Left);
    assert_eq!(list.keys(&Keys::default(), 3), Picked::Nothing);
    // (Fewer rows than before: on the last; none: on the first.)
    list.at = 9;
    assert_eq!(list.keys(&Keys::default(), 4), Picked::Nothing);
    assert_eq!(list.at, 3);
    assert_eq!(list.keys(&keys(|keys| keys.down = true), 0), Picked::Moved);
    assert_eq!(list.at, 0);
}

#[test]
fn a_list_cursor_reads_one_input_a_frame() {
    use rules::services::Input;
    let mut list = ListCursor::at(1);
    assert_eq!(list.press(Some(Input::Down), 2), Picked::Moved);
    assert_eq!(list.at, 0);
    assert_eq!(list.press(Some(Input::Up), 2), Picked::Moved);
    assert_eq!(list.at, 1);
    assert_eq!(list.press(Some(Input::Left), 2), Picked::Nothing);
    assert_eq!(list.press(Some(Input::Confirm), 2), Picked::Chosen(1));
    assert_eq!(list.press(Some(Input::Cancel), 2), Picked::Left);
    assert_eq!(list.press(None, 2), Picked::Nothing);
    // (The arrows first: Enter with an arrow is the arrow.)
    assert_eq!(widgets::input(&keys(|keys| (keys.up, keys.enter) = (true, true))), Some(Input::Up));
    assert_eq!(widgets::input(&keys(|keys| keys.named = vec!["r".to_string()])), Some(Input::Sort));
}

#[test]
fn a_long_list_scrolls_with_its_cursor() {
    let list = |at| ListCursor::at(at);
    // (Following the cursor: it is the last seen once past the first rows.)
    assert_eq!(list(0).first(8), 0);
    assert_eq!(list(7).first(8), 0);
    assert_eq!(list(8).first(8), 1);
    assert_eq!(list(20).first(8), 13);
    // (By pages.)
    assert_eq!(list(7).page(8), 0);
    assert_eq!(list(8).page(8), 8);
    assert_eq!(list(17).page(8), 16);
    assert_eq!(list(3).first(0), 3);
}

#[test]
fn a_yes_or_no_turns_with_any_arrow_and_escape_says_no() {
    let mut answer = YesNo::default();
    assert_eq!((answer.yes, answer.cursor()), (true, 0));
    assert_eq!(answer.press(&keys(|keys| keys.right = true)), None);
    assert_eq!((answer.yes, answer.cursor()), (false, 1));
    assert_eq!(answer.press(&keys(|keys| keys.up = true)), None);
    assert_eq!(answer.press(&keys(|keys| keys.enter = true)), Some(true));
    assert_eq!(answer.press(&keys(|keys| keys.escape = true)), Some(false), "cancelled: no");
    answer.yes = false;
    assert_eq!(answer.press(&keys(|keys| keys.enter = true)), Some(false));
}

#[test]
fn rows_and_their_second_column_line_up() {
    let art = art();
    let mut canvas = Canvas::default();
    let rows = [("a", color::WHITE), ("b", color::WHITE), ("c", color::WHITE)];
    let lines = canvas.rows(&art, Rows { left: 16, top: 40, pitch: 13, indent: 20 }, &rows, Some(2));
    assert_eq!(lines, vec![40, 53, 66]);
    assert_eq!(canvas.cursors, vec![(16, 66)], "the cursor on its row, at the list's left");
    let none = canvas.rows(&art, Rows { left: 16, top: 40, pitch: 13, indent: 20 }, &rows, None);
    assert_eq!((none.len(), canvas.cursors.len()), (3, 1));
}

#[test]
fn a_choice_box_sits_above_the_dialogue_box_and_scrolls() {
    let art = art();
    let mut canvas = Canvas::default();
    let options: Vec<String> = (0..10).map(|at| format!("option {at}")).collect();
    canvas.choice_box(&art, &options, ListCursor::at(9), false);
    // (Eight rows of 14 pixels: the window is 14 tiles high, ending at tile 15, from tile 1; the cursor on the last row seen.)
    let window_top = SCREEN_HEIGHT as i32 + TILE as i32;
    assert_eq!(canvas.cursors.iter().map(|&(_, y)| y).collect::<Vec<_>>(), vec![window_top + 7 * 14]);
    // (No choices: nothing drawn.)
    let mut empty = Canvas::default();
    empty.choice_box(&art, &[], ListCursor::default(), false);
    assert!(empty.data.iter().all(|&part| part == 0));
}
