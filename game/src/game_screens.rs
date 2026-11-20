//! The game's own screens a film one plays opens (`screen shop`, `map
//! …`), over the scene, the film waiting until they are left: the shops,
//! the bank, the storage, the board of jobs, the recruits who come along,
//! the world map's ways. What they do is the rules' (`rules::services`,
//! `rules::jobs`, the `Run` of the film's game, `Playthrough`); here, a key at
//! a time and what the box shows.

use rules::jobs::Listing;
use rules::run::Recruit;
use rules::run::job_floor::job_title;
use rules::run::town::Draw;
use rules::services::{self, Input, Service};
use shell::Keys;
use stage_view::ui::message::Words;
use stage_view::ui::widgets::input;
use stage_view::ui::{Canvas, ListCursor, Picked, UiArt};

use crate::playthrough::Playthrough;

/// A screen open.
pub enum GameScreen {
    /// The bank, the storage, a shop (which: 0 the shop, 1 the wares).
    Service { service: Service, shop: Option<usize> },
    /// The board of jobs: those offered, those the team holds.
    Board(Listing),
    /// Who of the recruits comes along (two at most).
    Recruits { cursor: ListCursor },
    /// The world map's ways, by their names; the one chosen once left (from 1; 0: none).
    Map { ways: Vec<String>, cursor: ListCursor, chosen: usize },
    /// The game's menu, M or X in a place: the team, the bag, a save, closed;
    /// what it tells shown; a save asked for (the game writes it).
    Menu { cursor: ListCursor, said: Option<String>, save: bool },
}

/// The screens by the words a film opens them with.
pub const SCREENS: &[(&str, &str)] = &[
    ("shop", "the shop: buy, sell (what it has for sale changes after each dungeon)"),
    ("market", "the market: other items to buy, to sell"),
    ("bank", "the bank: deposit money, withdraw it (money deposited is not lost in a dungeon)"),
    ("storage", "storage: deposit items, take them back (they are not lost in a dungeon)"),
    ("jobs", "the job board: take one (in a dungeon of one's own), see those taken"),
    ("recruits", "the recruits: who comes into a dungeon with the team (two at most)"),
];

impl GameScreen {
    /// The screen of that word (`ways`: the map's), made ready in the film's game; none if there is no such screen.
    pub fn open(word: &str, ways: Vec<String>, playthrough: &mut Playthrough, draw: &mut Draw) -> Option<GameScreen> {
        let open = playthrough.open_dungeons();
        let run = playthrough.run_mut()?;
        Some(match word {
            "shop" | "market" => {
                let menu = if word == "market" { services::KECLEON_WARES } else { services::KECLEON_SHOP };
                let shop = Service::shop(menu).map(usize::from);
                if let Some(shop) = shop {
                    run.restock(shop, open.len(), draw.below(usize::MAX >> 1) as u32);
                }
                GameScreen::Service { service: Service::open(menu)?, shop }
            }
            "bank" => GameScreen::Service { service: Service::open(services::DUSKULL_BANK)?, shop: None },
            "storage" => GameScreen::Service { service: Service::open(services::KANGASKHAN_STORAGE)?, shop: None },
            "jobs" => {
                run.fill_board(false, &open, draw);
                // (None on a floor laid out by hand: a boss's arena.)
                let book = run.book.clone();
                let (offered, _) = run.board(false);
                offered.retain(|job| {
                    let floors = book.tables.dungeons.get(job.dungeon as usize).map(|dungeon| &dungeon.floors);
                    floors.and_then(|floors| floors.get((job.floor as usize).saturating_sub(1))).is_some_and(|floor| floor.fixed.is_none())
                });
                GameScreen::Board(Listing::board(false))
            }
            "recruits" => GameScreen::Recruits { cursor: ListCursor::default() },
            "map" => GameScreen::Map { ways, cursor: ListCursor::default(), chosen: 0 },
            "menu" => GameScreen::Menu { cursor: ListCursor::default(), said: None, save: false },
            _ => return None,
        })
    }

    /// A frame of it: the keys read, what the box shows; whether it is left.
    pub fn play(&mut self, keys: &Keys, playthrough: &mut Playthrough) -> (bool, Words) {
        let pressed = input(keys);
        let own = playthrough.own.clone();
        let Some(run) = playthrough.run_mut() else {
            return (true, Words::default());
        };
        let book = run.book.clone();
        match self {
            GameScreen::Service { service, shop } => {
                let prices = |id: u16| book.item_prices(id);
                let (left, view) = run.through_service(*shop, |goods| {
                    let left = pressed.is_some_and(|pressed| service.press(pressed, goods, &prices));
                    let about = |id: u16| book.tables.item_descriptions.get(rules::dungeon::stack::id(id) as usize).cloned().unwrap_or_default();
                    (left, service.view(goods, |id| book.item_name(id), about, &prices))
                });
                (left, Words::of_menu(view.message, view.options, view.cursor))
            }
            GameScreen::Board(board) => {
                let (offered, held) = run.board(false);
                let left = pressed.is_some_and(|pressed| board.press_board(pressed, offered, held));
                let (offered, held) = run.board(false);
                let view = board.view_board(offered, held, |job| job_title(job, &book), |dungeon| book.dungeon_name(dungeon));
                (left, Words::of_menu(view.message, view.options, view.cursor))
            }
            GameScreen::Recruits { cursor } => {
                let count = run.team().progress.recruits.len();
                let left = match cursor.press(pressed, count + 1) {
                    Picked::Chosen(row) if row < count => {
                        run.pick_along(row);
                        false
                    }
                    Picked::Chosen(_) | Picked::Left => true,
                    _ => false,
                };
                let along = run.along();
                let row = |(at, recruit): (usize, &Recruit)| {
                    format!("{} {}  Lv. {}", if along.contains(&at) { "[CS:G]+[CR]" } else { "  " }, recruit.name, recruit.stats[0])
                };
                let options = run.team().progress.recruits.iter().enumerate().map(row).chain(["Done".to_string()]).collect();
                let message = match count {
                    0 => "Nobody has joined the team yet.",
                    _ => "Who goes exploring with you? (two at most)",
                };
                (left, Words::of_menu(message.to_string(), options, cursor.at))
            }
            GameScreen::Map { ways, cursor, chosen } => {
                let left = match cursor.press(pressed, ways.len() + 1) {
                    Picked::Chosen(row) => {
                        *chosen = if row < ways.len() { row + 1 } else { 0 };
                        true
                    }
                    Picked::Left => true,
                    _ => false,
                };
                let options = ways.iter().cloned().chain(["Stay here".to_string()]).collect();
                (left, Words::of_menu("Where to?".to_string(), options, cursor.at))
            }
            GameScreen::Menu { cursor, said, save } => {
                const OPTIONS: [&str; 4] = ["Team", "Bag", "Save", "Close"];
                let mut left = false;
                if said.is_some() {
                    if matches!(pressed, Some(Input::Confirm | Input::Cancel)) {
                        *said = None;
                    }
                } else {
                    match cursor.press(pressed, OPTIONS.len()) {
                        Picked::Chosen(0) => {
                            let team: Vec<String> =
                                run.members().into_iter().map(|whom| format!("{} Lv. {}", run.member_name(whom), run.member_level(whom))).collect();
                            let recruits = run.team().progress.recruits.len();
                            *said = Some(format!("Team: {}.{}", team.join(", "), if recruits > 0 { format!(" Recruits: {recruits}.") } else { String::new() }));
                        }
                        Picked::Chosen(1) => {
                            let mut things: Vec<String> = crate::playthrough::bag_of(run).into_iter().map(|(name, count)| format!("{name} x{count}")).collect();
                            things.extend(own.iter().map(|(name, count)| format!("{name} x{count}")));
                            let money = run.bag().money;
                            *said = Some(match things.is_empty() {
                                true => format!("The bag is empty. {money} Poké."),
                                false => format!("Bag: {}. {money} Poké.", things.join(", ")),
                            });
                        }
                        Picked::Chosen(2) => *save = true,
                        Picked::Chosen(_) | Picked::Left => left = true,
                        _ => {}
                    }
                }
                let words = match said {
                    Some(said) => Words::of_menu(said.clone(), Vec::new(), 0),
                    None => Words::of_menu("What now?".to_string(), OPTIONS.map(str::to_string).to_vec(), cursor.at),
                };
                (left, words)
            }
        }
    }

    /// The box and its choices, as the game shows this screen: a service's list by pages with the money beside it, the amount at the bank.
    pub fn draw(&self, canvas: &mut Canvas, art: &UiArt, words: &Words, playthrough: &Playthrough) {
        words.draw(canvas, art, false);
        let service = match self {
            GameScreen::Service { service, .. } => Some(service),
            _ => None,
        };
        let (money, bank) = playthrough.run().map_or((0, 0), |run| (run.bag().money, run.bank()));
        if let Some(amount) = service.and_then(Service::amount) {
            stage_view::ui::services::draw_amount(canvas, art, amount, (money, bank));
        }
        match service.and_then(Service::title).filter(|_| !words.options.is_empty()) {
            Some(title) => stage_view::ui::services::draw_list(canvas, art, title, words, money),
            None => words.draw_choices(canvas, art, false),
        }
    }

    /// The way chosen on the map (from 1; 0: none).
    pub fn chosen(&self) -> usize {
        match self {
            GameScreen::Map { chosen, .. } => *chosen,
            _ => 0,
        }
    }
}
