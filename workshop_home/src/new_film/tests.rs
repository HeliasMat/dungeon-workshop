//! The films New Film writes: their text, and that they pass the film
//! checker (with the repository's tables; the placing, with the game's files
//! when they are extracted).

use super::*;

fn tables() -> Arc<film::tables::Tables> {
    static READ: std::sync::OnceLock<Arc<film::tables::Tables>> = std::sync::OnceLock::new();
    READ.get_or_init(|| Arc::new(film::tables::Tables::read(&assets::testing::library(), &assets::testing::data()))).clone()
}

fn plan(place: &str, cast: &[&str], start: Start) -> Plan {
    let tables = tables();
    Plan {
        title: "The Big Day".to_string(),
        place: place.to_string(),
        cast: cast.iter().map(|word| (word.to_string(), shown_name(&tables, word))).collect(),
        start,
    }
}

/// What reading the film finds wrong (its remarks left aside).
fn wrong(text: &str) -> Vec<String> {
    film::compile_with(text, tables()).problems.into_iter().filter(|problem| !problem.is_remark()).map(|problem| problem.message).collect()
}

/// Each start, two to six in the cast, put by hand: the film reads with nothing wrong.
#[test]
fn every_start_writes_a_film_that_reads() {
    let casts: [&[&str]; 3] = [&["pikachu", "piplup"], &["pikachu", "piplup", "chimchar"], &["pikachu", "piplup", "chimchar", "munchlax", "pichu", "azurill"]];
    for start in Start::ALL {
        for cast in casts {
            let plan = plan("beach", cast, start);
            let placed: Vec<Placed> = cast
                .iter()
                .enumerate()
                .map(|(at, word)| Placed { word: word.to_string(), at: [300 + 40 * at as i32, 220], facing: "left".to_string() })
                .collect();
            let text = film_text(&plan, Some("beach"), &placed);
            assert_eq!(wrong(&text), Vec::<String>::new(), "{start:?} with {cast:?}:\n{text}");
        }
    }
}

/// The text: its title, the scene's head naming the cast, the music, a put
/// for each one put (none for who arrives), the first moment framed.
#[test]
fn the_text_names_the_place_the_cast_and_puts_them() {
    let plan = plan("town", &["pikachu", "piplup", "chimchar"], Start::Arrival);
    let placed = vec![
        Placed { word: "pikachu".to_string(), at: [330, 212], facing: "right".to_string() },
        Placed { word: "piplup".to_string(), at: [374, 218], facing: "left".to_string() },
    ];
    let text = film_text(&plan, Some("treasure_town"), &placed);
    assert!(text.contains("title: The Big Day\n"), "{text}");
    assert!(
        text.contains("scene: town pikachu piplup chimchar\nmusic treasure_town\npikachu put 330 212 facing right\npiplup put 374 218 facing left\n"),
        "{text}"
    );
    assert!(!text.contains("chimchar put"), "who arrives is not put: {text}");
    assert!(text.contains("moment: waiting\nframe pikachu piplup\n"), "{text}");
    assert!(text.contains("chimchar enter from right"), "{text}");
    assert_eq!(sketch(&plan), "scene: town pikachu piplup\nwait 2s\n");
}

/// A Pokémon's name in the lines: the game's.
#[test]
fn the_lines_say_the_games_names() {
    let tables = tables();
    assert_eq!(shown_name(&tables, "not_a_pokemon"), "Not a pokemon");
    let plan = plan("beach", &["pikachu", "piplup"], Start::Talk);
    let text = film_text(&plan, None, &[]);
    assert!(text.contains(&format!("pikachu (happy): Hey, {}!", plan.cast[1].1)), "{text}");
}

/// With the game's files: the scene's music read, each one put on the
/// place's ground by the scene itself, apart, turned to one another; the
/// whole film reads with nothing wrong.
#[test]
fn a_film_made_whole_places_its_cast_and_reads() {
    let data = assets::testing::data();
    if !assets::testing::extracted(&data) {
        return;
    }
    for (place, start) in [("beach", Start::Talk), ("town", Start::Arrival), ("forest", Start::Chase)] {
        let plan = plan(place, &["pikachu", "piplup", "chimchar"], start);
        let placed = placed(&plan, &data, &assets::testing::library(), tables()).expect("placed");
        let put = put_at_start(&plan);
        assert_eq!(placed.len(), put.len(), "{place}: {placed:?}");
        assert!(placed.windows(2).all(|pair| pair[0].at != pair[1].at), "{place}: apart: {placed:?}");
        let text = made(&plan, &data, &assets::testing::library(), tables());
        assert!(text.contains("\nmusic "), "{place}: the place's music:\n{text}");
        assert_eq!(text.matches(" put ").count(), put.len(), "{text}");
        assert_eq!(wrong(&text), Vec::<String>::new(), "{text}");
        println!("--- {place} {start:?}\n{text}");
    }
}
