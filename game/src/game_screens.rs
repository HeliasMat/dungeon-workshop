//! The game's own screens a film one plays opens (`screen shop`, `map
//! …`), over the scene, the film waiting until they are left: the shops,
//! the bank, the storage, the boards of jobs and of outlaws, the Job List
//! (the jobs taken, to give up), Chimecco Assembly (who goes, each member
//! seen, its tactic, a recruit released), the team's name,
//! Xatu's appraisal of the treasure boxes, the café's Gummi juice, Chansey's
//! Day Care (the eggs hatched), the bed (a new day), the world map's ways;
//! Croagunk's Swap Shop, the Link Shop, Luminous Spring, the Recycle Shop
//! and its prize drawing, the bag (`crate::town_shops`); sentry duty,
//! Marowak's Dojo, Pelipper's SOS, the personality test (`crate::guild`). What they do is the rules' (`rules::services`,
//! `rules::jobs`, the `Run` of the film's game, `Playthrough`); here, a key at
//! a time and what the box shows.

use std::collections::BTreeSet;

use rules::jobs::{Job, Listing};
use rules::run::job_floor::{job_details, job_title};
use rules::run::town::{Draw, Whom};
use rules::services::{self, Input, Service};
use shell::Keys;
use stage_view::ui::message::Words;
use stage_view::ui::widgets::input;
use stage_view::ui::{Canvas, ListCursor, Picked, UiArt};

use crate::assembly::{Assembly, Naming};
use crate::guild::{Dojo, Face, QuizScreen, Rescue, SentryDuty};
use crate::playthrough::Playthrough;
use crate::town_shops::{Bag, Evolved, LinkShop, Lottery, Recycle, Spring, SwapShop};

/// A screen open.
pub enum GameScreen {
    /// The bank, the storage, a shop (which: 0 the shop, 1 the wares).
    Service {
        service: Service,
        shop: Option<usize>,
    },
    /// A board of jobs (the outlaws' if it says): those offered, those the
    /// team holds; one read in full, then taken.
    Board(Listing),
    /// The Job List: the jobs held, each read in full, given up.
    JobList(Listing),
    /// Chimecco Assembly: who goes exploring (four at most), each member
    /// seen, its tactic, a recruit released (`crate::assembly`).
    Recruits(Assembly),
    /// The team's name typed.
    Naming(Naming),
    /// Xatu opens the treasure boxes of the bag, one at a time, for a
    /// price: what it found told; the numbers it draws.
    Appraise {
        cursor: ListCursor,
        said: Option<String>,
        draw: Draw,
    },
    /// The café presses a Gummi of the bag into juice for the leader or its
    /// partner: the Gummi chosen (its place in the bag), then who drinks; what it did told.
    Cafe {
        cursor: ListCursor,
        gummi: Option<usize>,
        said: Option<String>,
    },
    /// Chansey's Day Care: the eggs the jobs' clients gave, each hatched
    /// once it has waited its days, the one born joining the
    /// recruits; what it did told.
    DayCare {
        cursor: ListCursor,
        said: Option<String>,
    },
    /// Croagunk's Swap Shop: Exclusive Items swapped for a rarer one.
    Swap(SwapShop),
    /// The Link Shop: moves linked, switched, forgotten, remembered.
    Link(LinkShop),
    /// Luminous Spring: a member evolved.
    Spring(Spring),
    /// The Recycle Shop (Project P), and its prize drawing.
    Recycle(Recycle),
    Lottery(Lottery),
    /// The bag: sorted, an item thrown away.
    Bag(Bag),
    /// Sentry duty: whose footprints, ten visitors; Marowak's Dojo, its
    /// mazes; Pelipper's SOS and A-OK Mail; the personality test.
    Sentry(SentryDuty),
    Dojo(Dojo),
    Rescue(Rescue),
    Quiz(QuizScreen),
    /// The bed: "Go to sleep?", then "Save your adventure?" (as the
    /// original asks before the night); what was answered once left.
    Bed {
        cursor: ListCursor,
        sleeps: bool,
        answer: Option<Slept>,
    },
    /// The world map's ways, by their names; the one chosen once left (from 1; 0: none).
    Map {
        ways: Vec<String>,
        cursor: ListCursor,
        chosen: usize,
    },
    /// The game's menu, M or X in a place: the team, the bag, a save, closed;
    /// what it tells shown; a save asked for (the game writes it).
    Menu {
        cursor: ListCursor,
        said: Option<String>,
        save: bool,
    },
}

/// What the bed was told: to sleep, the game saved first or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slept {
    pub save: bool,
}

impl GameScreen {
    /// The screen of that word (`ways`: the map's), made ready in the film's
    /// game; the boards' jobs in the dungeons of `jobs_in` only, if it says;
    /// a shop's wares those of `stock` if the game has a list (`shop_stock:`);
    /// none if there is no such screen.
    pub fn open(
        word: &str,
        ways: Vec<String>,
        playthrough: &mut Playthrough,
        draw: &mut Draw,
        jobs_in: Option<&BTreeSet<String>>,
        stock: Option<&[String]>,
    ) -> Option<GameScreen> {
        let open = playthrough.open_dungeons(None);
        let for_jobs = playthrough.open_dungeons(jobs_in);
        let stock: Option<Vec<u16>> = stock.map(|names| names.iter().filter_map(|name| playthrough.item_named(name)).collect());
        let run = playthrough.run_mut()?;
        Some(match word {
            "shop" | "market" => {
                let menu = if word == "market" { services::KECLEON_WARES } else { services::KECLEON_SHOP };
                let shop = Service::shop(menu).map(usize::from);
                match (shop, &stock) {
                    (Some(shop), Some(stock)) => run.restock_with(shop, stock),
                    (Some(shop), None) => run.restock(shop, open.len(), draw.below(usize::MAX >> 1) as u32),
                    (None, _) => {}
                }
                GameScreen::Service { service: Service::open(menu)?, shop }
            }
            "bank" => GameScreen::Service { service: Service::open(services::DUSKULL_BANK)?, shop: None },
            "storage" => GameScreen::Service { service: Service::open(services::KANGASKHAN_STORAGE)?, shop: None },
            "jobs" | "outlaws" => {
                let outlaws = word == "outlaws";
                run.fill_board(outlaws, &for_jobs, draw);
                GameScreen::Board(Listing::board(outlaws))
            }
            "joblist" => GameScreen::JobList(Listing::board(false)),
            "cafe" => GameScreen::Cafe { cursor: ListCursor::default(), gummi: None, said: None },
            "recruits" => GameScreen::Recruits(Assembly::default()),
            "name" => GameScreen::Naming(Naming::new(run)),
            "daycare" => GameScreen::DayCare { cursor: ListCursor::default(), said: None },
            "swap" => GameScreen::Swap(SwapShop::default()),
            "link" => GameScreen::Link(LinkShop::default()),
            "evolve" => GameScreen::Spring(Spring::default()),
            "recycle" => GameScreen::Recycle(Recycle::new(Draw(draw.below(usize::MAX >> 1) as u32))),
            "lottery" => GameScreen::Lottery(Lottery::new(Draw(draw.below(usize::MAX >> 1) as u32))),
            "bag" => GameScreen::Bag(Bag::default()),
            "sentry" => GameScreen::Sentry(SentryDuty::new(run, Draw(draw.below(usize::MAX >> 1) as u32))),
            "dojo" => GameScreen::Dojo(Dojo::default()),
            "rescue" => GameScreen::Rescue(Rescue::default()),
            "appraise" => GameScreen::Appraise { cursor: ListCursor::default(), said: None, draw: Draw(draw.below(usize::MAX >> 1) as u32) },
            "bed" => GameScreen::Bed { cursor: ListCursor::default(), sleeps: false, answer: None },
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
                let (offered, held) = run.board(board.outlaws);
                let left = pressed.is_some_and(|pressed| board.press_board(pressed, offered, held));
                let (offered, held) = run.board(board.outlaws);
                let detail = |job: &Job| job_details(job, &book);
                let view = board.view_board(offered, held, |job| job_title(job, &book), detail, |dungeon| book.dungeon_name(dungeon));
                (left, Words::of_menu(view.message, view.options, view.cursor))
            }
            GameScreen::JobList(list) => {
                let (_, held) = run.board(false);
                let left = pressed.is_some_and(|pressed| list.press_held(pressed, held));
                let (_, held) = run.board(false);
                let detail = |job: &Job| job_details(job, &book);
                let view = list.view_held(held, |job| job_title(job, &book), detail, |dungeon| book.dungeon_name(dungeon));
                (left, Words::of_menu(view.message, view.options, view.cursor))
            }
            GameScreen::Recruits(assembly) => {
                let (left, (message, options, at)) = assembly.play(pressed, run);
                (left, Words::of_menu(message, options, at))
            }
            GameScreen::Swap(shop) => shown(shop.play(pressed, run)),
            GameScreen::Link(shop) => shown(shop.play(pressed, run)),
            GameScreen::Spring(spring) => shown(spring.play(pressed, run)),
            GameScreen::Recycle(shop) => shown(shop.play(pressed, run)),
            GameScreen::Lottery(drawing) => shown(drawing.play(pressed, run)),
            GameScreen::Bag(bag) => shown(bag.play(pressed, run, &own)),
            GameScreen::Sentry(duty) => shown(duty.play(pressed, run)),
            GameScreen::Dojo(dojo) => shown(dojo.play(pressed, run)),
            GameScreen::Rescue(rescue) => shown(rescue.play(pressed, run)),
            GameScreen::Quiz(quiz) => shown(quiz.play(pressed, run)),
            GameScreen::Naming(naming) => {
                let (left, message) = naming.play(keys, run);
                (left, Words::of_menu(message, Vec::new(), 0))
            }
            GameScreen::Appraise { cursor, said, draw } => {
                if said.is_some() {
                    if matches!(pressed, Some(Input::Confirm | Input::Cancel)) {
                        *said = None;
                    }
                    return (false, Words::of_menu(said.clone().unwrap_or_default(), Vec::new(), 0));
                }
                let boxes = run.boxes();
                let left = match cursor.press(pressed, boxes.len() + 1) {
                    Picked::Chosen(row) if row < boxes.len() => {
                        *said = Some(run.appraise(boxes[row], draw));
                        false
                    }
                    Picked::Chosen(_) | Picked::Left => true,
                    _ => false,
                };
                if let Some(said) = said {
                    return (left, Words::of_menu(said.clone(), Vec::new(), 0));
                }
                let boxes = run.boxes();
                let options = boxes.iter().map(|&at| book.item_name(run.bag().items[at])).chain(["Done".to_string()]).collect();
                let message = match boxes.is_empty() {
                    true => "You have no treasure box for me to open.".to_string(),
                    false => format!("Which box shall I open? It costs {} Poké.", rules::town::APPRAISAL_PRICE),
                };
                (left, Words::of_menu(message, options, cursor.at))
            }
            GameScreen::Cafe { cursor, gummi, said } => {
                if said.is_some() {
                    if matches!(pressed, Some(Input::Confirm | Input::Cancel)) {
                        *said = None;
                    }
                    return (false, Words::of_menu(said.clone().unwrap_or_default(), Vec::new(), 0));
                }
                let drinkers: Vec<Whom> = run.members().into_iter().filter(|whom| matches!(whom, Whom::Leader | Whom::Partner)).collect();
                let left = match *gummi {
                    // (Who drinks the juice of the Gummi chosen; back to the Gummis.)
                    Some(at) => {
                        match cursor.press(pressed, drinkers.len() + 1) {
                            Picked::Chosen(row) if row < drinkers.len() => {
                                let id = rules::dungeon::stack::id(run.bag().items[at]);
                                *said = Some(run.drink_juice(id, drinkers[row] == Whom::Partner));
                                (*gummi, *cursor) = (None, ListCursor::default());
                            }
                            Picked::Chosen(_) | Picked::Left => (*gummi, *cursor) = (None, ListCursor::default()),
                            _ => {}
                        }
                        false
                    }
                    None => {
                        let gummis = run.gummis();
                        match cursor.press(pressed, gummis.len() + 1) {
                            Picked::Chosen(row) if row < gummis.len() => {
                                (*gummi, *cursor) = (Some(gummis[row]), ListCursor::default());
                                false
                            }
                            Picked::Chosen(_) | Picked::Left => true,
                            _ => false,
                        }
                    }
                };
                if let Some(said) = said {
                    return (left, Words::of_menu(said.clone(), Vec::new(), 0));
                }
                if let Some(at) = *gummi {
                    let juice = book.item_name(run.bag().items[at]);
                    let options = drinkers.iter().map(|&whom| run.member_name(whom)).chain(["Back".to_string()]).collect();
                    return (left, Words::of_menu(format!("{juice} juice, freshly pressed! Who drinks it?"), options, cursor.at));
                }
                let gummis = run.gummis();
                let options = gummis.iter().map(|&at| book.item_name(run.bag().items[at])).chain(["Done".to_string()]).collect();
                let message = match gummis.is_empty() {
                    true => "Bring me a Gummi and I'll press it into juice. It sharpens the mind!",
                    false => "Which Gummi shall I press into juice?",
                };
                (left, Words::of_menu(message.to_string(), options, cursor.at))
            }
            GameScreen::DayCare { cursor, said } => {
                if said.is_some() {
                    if matches!(pressed, Some(Input::Confirm | Input::Cancel)) {
                        *said = None;
                    }
                    return (false, Words::of_menu(said.clone().unwrap_or_default(), Vec::new(), 0));
                }
                let count = run.eggs().len();
                let left = match cursor.press(pressed, count + 1) {
                    Picked::Chosen(row) if row < count => {
                        *said = Some(run.hatch(row));
                        *cursor = ListCursor::default();
                        false
                    }
                    Picked::Chosen(_) | Picked::Left => true,
                    _ => false,
                };
                if let Some(said) = said {
                    return (left, Words::of_menu(said.clone(), Vec::new(), 0));
                }
                let (message, options) = day_care(run.eggs());
                (left, Words::of_menu(message, options, cursor.at))
            }
            GameScreen::Bed { cursor, sleeps, answer } => {
                let left = match cursor.press(pressed, 2) {
                    // (Yes to the night: then the save asked.)
                    Picked::Chosen(0) if !*sleeps => {
                        (*sleeps, *cursor) = (true, ListCursor::default());
                        false
                    }
                    Picked::Chosen(row) if *sleeps => {
                        *answer = Some(Slept { save: row == 0 });
                        true
                    }
                    Picked::Chosen(_) | Picked::Left => true,
                    _ => false,
                };
                let (message, evening) = (bed_question(*sleeps), run.day().1);
                let message = if !*sleeps && evening { format!("It's getting late. {message}") } else { message.to_string() };
                (left, Words::of_menu(message, vec!["Yes".to_string(), "No".to_string()], cursor.at))
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
                const OPTIONS: [&str; 5] = ["Team", "Bag", "Save", "Job List", "Close"];
                let (mut left, mut job_list, mut bag) = (false, false, false);
                if said.is_some() {
                    if matches!(pressed, Some(Input::Confirm | Input::Cancel)) {
                        *said = None;
                    }
                } else {
                    match cursor.press(pressed, OPTIONS.len()) {
                        Picked::Chosen(0) => *said = Some(team_page(run)),
                        Picked::Chosen(1) => bag = true,
                        Picked::Chosen(2) => *save = true,
                        Picked::Chosen(3) => job_list = true,
                        Picked::Chosen(_) | Picked::Left => left = true,
                        _ => {}
                    }
                }
                let words = match said {
                    Some(said) => Words::of_menu(said.clone(), Vec::new(), 0),
                    None => Words::of_menu("What now?".to_string(), OPTIONS.map(str::to_string).to_vec(), cursor.at),
                };
                // (The Job List, in place of the menu.)
                if job_list {
                    *self = GameScreen::JobList(Listing::board(false));
                }
                // (The bag, its items each thrown away or not, sorted.)
                if bag {
                    *self = GameScreen::Bag(Bag::default());
                    return self.play(&Keys::default(), playthrough);
                }
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

    /// What the bed was told, once left: to sleep (saved first or not); none: not now.
    pub fn slept(&self) -> Option<Slept> {
        match self {
            GameScreen::Bed { answer, .. } => *answer,
            _ => None,
        }
    }

    /// The personality test, if the game's data has it.
    pub fn quiz(playthrough: &Playthrough, draw: &mut Draw) -> Option<GameScreen> {
        QuizScreen::new(playthrough.run()?, Draw(draw.below(usize::MAX >> 1) as u32)).map(GameScreen::Quiz)
    }

    /// The portrait beside the box: the visitor of sentry duty (in shadow
    /// until told), the partner on show in the personality test.
    pub fn face(&self) -> Option<Face> {
        match self {
            GameScreen::Sentry(duty) => duty.face(),
            GameScreen::Quiz(quiz) => quiz.face(),
            _ => None,
        }
    }

    /// The Dojo's maze chosen, once left.
    pub fn maze(&self) -> Option<u16> {
        match self {
            GameScreen::Dojo(dojo) => dojo.chosen,
            _ => None,
        }
    }

    /// The hero and its partner the personality test chose, once left.
    pub fn quizzed(&self) -> Option<(u16, u16)> {
        match self {
            GameScreen::Quiz(quiz) => quiz.chosen,
            _ => None,
        }
    }

    /// Who evolved at Luminous Spring, once it is left for the light.
    pub fn evolved(&self) -> Option<Evolved> {
        match self {
            GameScreen::Spring(spring) => spring.evolved.clone(),
            _ => None,
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

/// What a shop of `town_shops` shows, as the box shows it.
fn shown((left, (message, options, at)): (bool, crate::assembly::Shown)) -> (bool, Words) {
    (left, Words::of_menu(message, options, at))
}

/// The menu's Team page: the team's name and rank (the points to the next),
/// who goes exploring, the recruits, the bag's room.
fn team_page(run: &rules::run::Run) -> String {
    let (progress, ranks) = (&run.team().progress, &run.book.tables.jobs);
    let rank = match ranks.next_rank(progress.rank) {
        Some(next) => format!("{}, {} points to {}", ranks.rank_name(progress.rank), next - progress.rank, ranks.rank_name(next)),
        None => ranks.rank_name(progress.rank).to_string(),
    };
    let rank = if ranks.ranks.is_empty() { String::new() } else { format!(": {rank}") };
    let along = run.along();
    let goes = |whom: &Whom| match whom {
        Whom::Partner => !progress.partner_home,
        Whom::Recruit(at) => along.contains(at),
        Whom::Leader => true,
    };
    let team: Vec<String> = run.members().into_iter().filter(goes).map(|whom| format!("{} Lv. {}", run.member_name(whom), run.member_level(whom))).collect();
    let recruits = progress.recruits.len();
    let recruits = if recruits > 0 { format!(" Recruits: {recruits}.") } else { String::new() };
    format!("Team {}{rank}.\nExploring: {}.{recruits}\nBag: {} / {} items.", run.team().team_name(), team.join(", "), run.bag().items.len(), run.bag_room())
}

/// What the bed asks: to sleep, then (yes) to save first.
fn bed_question(sleeps: bool) -> &'static str {
    match sleeps {
        false => "Go to sleep? A new day begins in the morning.",
        true => "Save your adventure before sleeping?",
    }
}

/// What Chansey's Day Care says and offers: each egg (what hatches from it,
/// the days it still needs), then Done.
fn day_care(eggs: &[(u16, u8)]) -> (String, Vec<String>) {
    let egg = |&(_, days): &(u16, u8)| match days {
        0 => "Egg: ready to hatch!".to_string(),
        1 => "Egg: one more day".to_string(),
        days => format!("Egg: {days} more days"),
    };
    let message = match eggs.is_empty() {
        true => "Chansey: Welcome to the Day Care! No Egg is waiting here. A client may give you one for a job well done.",
        false => "Chansey: Welcome to the Day Care! Which Egg would you like to see?",
    };
    (message.to_string(), eggs.iter().map(egg).chain(["Done".to_string()]).collect())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_day_care_shows_each_egg_and_what_it_still_needs() {
        let (message, options) = super::day_care(&[(25, 0), (1, 2)]);
        assert!(message.starts_with("Chansey:"));
        assert_eq!(options, ["Egg: ready to hatch!", "Egg: 2 more days", "Done"]);
        assert!(super::day_care(&[]).0.contains("No Egg"));
    }
}
