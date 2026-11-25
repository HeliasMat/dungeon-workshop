//! The town's other shops, as the original has them: Croagunk's Swap Shop
//! (`swap`: Exclusive Items in the cauldron, a rarer one out), the Link
//! Shop (`link`: moves linked, delinked, switched, forgotten, remembered),
//! Luminous Spring (`evolve`: a member evolves once it meets its
//! condition), the Recycle Shop and its prize drawing (`recycle`,
//! `lottery`, Project P), and the bag (`bag`: sorted, an item thrown
//! away). What they do is the rules' (`rules::run::Run`, `rules::town`);
//! here, a key at a time and what is shown. Who keeps each shop is the
//! game's own resident: the words name none.

use rules::run::Run;
use rules::run::town::{Draw, LINK_PRICE, SwapOffer, Whom};
use rules::services::Input;
use rules::town::swap;
use stage_view::ui::{ListCursor, Picked};

use crate::assembly::Shown;

/// What a key did to a list of `rows` and a last one (Done, Back): the
/// row chosen (`Some(Some(row))`), left (`Some(None)`), or nothing yet.
fn pick(cursor: &mut ListCursor, pressed: Option<Input>, rows: usize) -> Option<Option<usize>> {
    match cursor.press(pressed, rows + 1) {
        Picked::Chosen(row) if row < rows => Some(Some(row)),
        Picked::Chosen(_) | Picked::Left => Some(None),
        _ => None,
    }
}

/// Yes or No: `Some(true)` for Yes, `Some(false)` for No or left.
fn yes(cursor: &mut ListCursor, pressed: Option<Input>) -> Option<bool> {
    match cursor.press(pressed, 2) {
        Picked::Chosen(row) => Some(row == 0),
        Picked::Left => Some(false),
        _ => None,
    }
}

fn yes_no() -> Vec<String> {
    vec!["Yes".to_string(), "No".to_string()]
}

/// What was told, shown till a key (that key taken by it).
fn told(said: &mut Option<String>, pressed: Option<Input>) -> Option<Shown> {
    let shown = said.clone()?;
    if matches!(pressed, Some(Input::Confirm | Input::Cancel)) {
        *said = None;
    }
    Some((shown, Vec::new(), 0))
}

/// Its stars, as the items' info marks them.
fn stars(item: u16) -> String {
    "*".repeat(swap::stars(item) as usize)
}

/// Croagunk's Swap Shop open: the offers, one asked about, what was told.
#[derive(Default)]
pub struct SwapShop {
    cursor: ListCursor,
    asked: Option<(SwapOffer, ListCursor)>,
    said: Option<String>,
}

impl SwapShop {
    pub fn play(&mut self, pressed: Option<Input>, run: &mut Run) -> (bool, Shown) {
        if let Some(shown) = told(&mut self.said, pressed) {
            return (false, shown);
        }
        let mut left = false;
        match &mut self.asked {
            Some((offer, cursor)) => match yes(cursor, pressed) {
                Some(true) => {
                    self.said = Some(run.swap(offer).unwrap_or_else(|why| why));
                    self.asked = None;
                }
                Some(false) => self.asked = None,
                None => {}
            },
            None => {
                let offers = run.swap_offers();
                match pick(&mut self.cursor, pressed, offers.len()) {
                    Some(Some(row)) => match (run.bag().money >= swap::PRICE, run.swap_takes(&offers[row])) {
                        (false, _) => self.said = Some(format!("No Poké, no swap! It's {} Poké a swap.", swap::PRICE)),
                        (true, None) => self.said = Some(lacking(&offers[row], run)),
                        (true, Some(_)) => self.asked = Some((offers[row].clone(), ListCursor::default())),
                    },
                    Some(None) => left = true,
                    None => {}
                }
            }
        }
        match &self.said {
            Some(said) => (left, (said.clone(), Vec::new(), 0)),
            None => (left, self.shown(run)),
        }
    }

    fn shown(&self, run: &Run) -> Shown {
        let name = |item: u16| run.book.item_name(item);
        if let Some((offer, cursor)) = &self.asked {
            let taken: Vec<String> = run.swap_takes(offer).unwrap_or_default().into_iter().map(name).collect();
            let what = match &offer.set {
                Some(_) => taken.join(", "),
                None => format!("{} Exclusive Items ({})", taken.len(), taken.join(", ")),
            };
            let message = format!("Swap your {what} for the {}? It costs {} Poké.", name(offer.item), swap::PRICE);
            return (message, yes_no(), cursor.at);
        }
        let offers = run.swap_offers();
        let offer = |offer: &SwapOffer| match &offer.set {
            Some(_) => format!("{} {}  its set", name(offer.item), stars(offer.item)),
            None => format!("{} {}  any {}", name(offer.item), stars(offer.item), swap::to_trade(offer.item)),
        };
        let options = offers.iter().map(offer).chain(["Done".to_string()]).collect();
        let message = match offers.is_empty() {
            true => "Nobody has anything to swap with you today. Come back tomorrow!".to_string(),
            false => format!(
                "The Swap Shop! Exclusive Items in the cauldron, a rarer one out: a set of the same kind, or so many of any kind. {} Poké a swap; you have {}.",
                swap::PRICE,
                run.bag().money
            ),
        };
        (message, options, self.cursor.at)
    }
}

/// What a swap lacks, said.
fn lacking(offer: &SwapOffer, run: &Run) -> String {
    match &offer.set {
        Some(set) => {
            let set: Vec<String> = set.iter().map(|&item| run.book.item_name(item)).collect();
            format!("You don't have the set for that one: {}.", set.join(", "))
        }
        None => format!("That one takes {} Exclusive Items, from the bag or the storage.", swap::to_trade(offer.item)),
    }
}

/// The Link Shop open.
#[derive(Default)]
pub struct LinkShop {
    cursor: ListCursor,
    page: LinkPage,
    /// The session paid for this visit (its links, moves forgotten and remembered).
    paid: bool,
    said: Option<String>,
}

#[derive(Default)]
enum LinkPage {
    /// Whose moves: the leader's, the partner's.
    #[default]
    Members,
    /// A member's moves, each linked or not, then Remember a move.
    Moves { partner: bool, cursor: ListCursor },
    /// What may be done with one of them.
    Move { partner: bool, slot: usize, cursor: ListCursor },
    /// The moves it forgot, to remember one.
    Remember { partner: bool, cursor: ListCursor },
    /// Four moves known: the one forgotten in place of the move remembered.
    Instead { partner: bool, id: u16, cursor: ListCursor },
}

/// What may be done with a move at that slot of so many.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MoveChoice {
    Link,
    Delink,
    Up,
    Down,
    Forget,
    Back,
}

fn move_choices(slot: usize, moves: usize, linked: bool) -> Vec<MoveChoice> {
    let mut choices = Vec::new();
    if slot > 0 {
        choices.push(if linked { MoveChoice::Delink } else { MoveChoice::Link });
        choices.push(MoveChoice::Up);
    }
    if slot + 1 < moves {
        choices.push(MoveChoice::Down);
    }
    choices.extend([MoveChoice::Forget, MoveChoice::Back]);
    choices
}

impl LinkShop {
    pub fn play(&mut self, pressed: Option<Input>, run: &mut Run) -> (bool, Shown) {
        if let Some(shown) = told(&mut self.said, pressed) {
            return (false, shown);
        }
        let left = self.press(pressed, run);
        match &self.said {
            Some(said) => (left, (said.clone(), Vec::new(), 0)),
            None => (left, self.shown(run)),
        }
    }

    /// The session paid, now if not yet: whether it is (else told why not).
    fn session(&mut self, run: &mut Run) -> bool {
        self.paid |= run.pay_link_session();
        if !self.paid {
            self.said = Some(format!("You don't have enough Poké. A session is {LINK_PRICE} Poké."));
        }
        self.paid
    }

    fn press(&mut self, pressed: Option<Input>, run: &mut Run) -> bool {
        let members = link_members(run);
        match &mut self.page {
            LinkPage::Members => match pick(&mut self.cursor, pressed, members.len()) {
                Some(Some(row)) => self.page = LinkPage::Moves { partner: members[row] == Whom::Partner, cursor: ListCursor::default() },
                Some(None) => return true,
                None => {}
            },
            LinkPage::Moves { partner, cursor } => {
                let (partner, moves) = (*partner, run.linked_moves(*partner));
                match pick(cursor, pressed, moves.len() + 1) {
                    Some(Some(row)) if row < moves.len() => self.page = LinkPage::Move { partner, slot: row, cursor: ListCursor::default() },
                    Some(Some(_)) if run.forgotten_moves(partner).is_empty() => {
                        self.said = Some(format!("{} hasn't forgotten any move: there is nothing to remember.", member_name(run, partner)));
                    }
                    Some(Some(_)) => self.page = LinkPage::Remember { partner, cursor: ListCursor::default() },
                    Some(None) => self.page = LinkPage::Members,
                    None => {}
                }
            }
            LinkPage::Move { partner, slot, cursor } => {
                let (partner, slot, moves) = (*partner, *slot, run.linked_moves(*partner));
                let linked = moves.get(slot).is_some_and(|(_, linked)| *linked);
                let choices = move_choices(slot, moves.len(), linked);
                let back = LinkPage::Moves { partner, cursor: ListCursor::at(slot) };
                match cursor.press(pressed, choices.len()) {
                    Picked::Chosen(row) => match choices[row] {
                        MoveChoice::Link | MoveChoice::Delink if self.session(run) => {
                            run.link_move(partner, slot);
                            self.page = back;
                        }
                        MoveChoice::Up => {
                            run.reorder_move(partner, slot, slot - 1);
                            self.page = LinkPage::Moves { partner, cursor: ListCursor::at(slot - 1) };
                        }
                        MoveChoice::Down => {
                            run.reorder_move(partner, slot, slot + 1);
                            self.page = LinkPage::Moves { partner, cursor: ListCursor::at(slot + 1) };
                        }
                        MoveChoice::Forget if self.session(run) => {
                            self.said = Some(run.forget_move(partner, slot));
                            self.page = LinkPage::Moves { partner, cursor: ListCursor::default() };
                        }
                        MoveChoice::Back => self.page = back,
                        _ => {}
                    },
                    Picked::Left => self.page = back,
                    _ => {}
                }
            }
            LinkPage::Remember { partner, cursor } => {
                let (partner, forgotten) = (*partner, run.forgotten_moves(*partner));
                match pick(cursor, pressed, forgotten.len()) {
                    Some(Some(row)) if run.linked_moves(partner).len() >= 4 => {
                        self.page = LinkPage::Instead { partner, id: forgotten[row], cursor: ListCursor::default() };
                    }
                    Some(Some(row)) if self.session(run) => {
                        self.said = Some(run.remember_move(partner, forgotten[row], None));
                        self.page = LinkPage::Moves { partner, cursor: ListCursor::default() };
                    }
                    Some(Some(_)) => {}
                    Some(None) => self.page = LinkPage::Moves { partner, cursor: ListCursor::default() },
                    None => {}
                }
            }
            LinkPage::Instead { partner, id, cursor } => {
                let (partner, id, moves) = (*partner, *id, run.linked_moves(*partner));
                match pick(cursor, pressed, moves.len()) {
                    Some(Some(row)) if self.session(run) => {
                        self.said = Some(run.remember_move(partner, id, Some(row)));
                        self.page = LinkPage::Moves { partner, cursor: ListCursor::default() };
                    }
                    Some(Some(_)) => {}
                    Some(None) => self.page = LinkPage::Remember { partner, cursor: ListCursor::default() },
                    None => {}
                }
            }
        }
        false
    }

    fn shown(&self, run: &Run) -> Shown {
        let move_name = |id: u16| run.book.move_name(id);
        let moves_of = |partner: bool| -> Vec<String> {
            run.linked_moves(partner).into_iter().map(|(id, linked)| format!("{}{}", if linked { "+ " } else { "" }, move_name(id))).collect()
        };
        match &self.page {
            LinkPage::Members => {
                let options = link_members(run).into_iter().map(|whom| run.member_name(whom)).chain(["Done".to_string()]).collect();
                let paid = if self.paid { " (paid for this visit)" } else { "" };
                let message =
                    format!("The Link Shop! Linked moves are used one after the other in a single turn. {LINK_PRICE} Poké a session{paid}. Whose moves?");
                (message, options, self.cursor.at)
            }
            LinkPage::Moves { partner, cursor } => {
                let options = moves_of(*partner).into_iter().chain(["Remember a move".to_string(), "Back".to_string()]).collect();
                let message = format!("{}'s moves (+: linked to the one above, used with it, top to bottom).", member_name(run, *partner));
                (message, options, cursor.at)
            }
            LinkPage::Move { partner, slot, cursor } => {
                let moves = run.linked_moves(*partner);
                let (id, linked) = moves.get(*slot).copied().unwrap_or_default();
                let option = |choice: &MoveChoice| match choice {
                    MoveChoice::Link => "Link to the one above",
                    MoveChoice::Delink => "Delink",
                    MoveChoice::Up => "Switch up",
                    MoveChoice::Down => "Switch down",
                    MoveChoice::Forget => "Forget",
                    MoveChoice::Back => "Back",
                };
                let options = move_choices(*slot, moves.len(), linked).iter().map(|choice| option(choice).to_string()).collect();
                (format!("{}: what shall it be? (Forgetting it forgets the moves linked to it.)", move_name(id)), options, cursor.at)
            }
            LinkPage::Remember { partner, cursor } => {
                let options = run.forgotten_moves(*partner).into_iter().map(move_name).chain(["Back".to_string()]).collect();
                (format!("Which forgotten move shall {} remember?", member_name(run, *partner)), options, cursor.at)
            }
            LinkPage::Instead { partner, id, cursor } => {
                let options = moves_of(*partner).into_iter().chain(["Back".to_string()]).collect();
                (format!("{} knows four moves. Which one does it forget for {}?", member_name(run, *partner), move_name(*id)), options, cursor.at)
            }
        }
    }
}

/// Who the Link Shop links the moves of: the leader and its partner.
fn link_members(run: &Run) -> Vec<Whom> {
    run.members().into_iter().filter(|whom| matches!(whom, Whom::Leader | Whom::Partner)).collect()
}

fn member_name(run: &Run, partner: bool) -> String {
    run.member_name(if partner { Whom::Partner } else { Whom::Leader })
}

/// A member evolved at the spring: who, from what into what (the rules'
/// species), its name before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evolved {
    pub whom: Whom,
    pub from: u16,
    pub into: u16,
    pub name: String,
}

/// Luminous Spring open.
#[derive(Default)]
pub struct Spring {
    cursor: ListCursor,
    page: SpringPage,
    /// The leader and its partner may evolve: the team has graduated (the
    /// original lets them once its story is over).
    pub free: bool,
    /// The one evolved, once it has: the screen is left for its light.
    pub evolved: Option<Evolved>,
    said: Option<String>,
}

#[derive(Default)]
enum SpringPage {
    #[default]
    Members,
    /// What a member may evolve into.
    Into { whom: Whom, cursor: ListCursor },
    /// "Evolve … into …?"
    Ask { whom: Whom, into: u16, cursor: ListCursor },
}

impl Spring {
    pub fn play(&mut self, pressed: Option<Input>, run: &mut Run) -> (bool, Shown) {
        if let Some(shown) = told(&mut self.said, pressed) {
            return (false, shown);
        }
        let left = self.press(pressed, run) || self.evolved.is_some();
        match &self.said {
            Some(said) => (left, (said.clone(), Vec::new(), 0)),
            None => (left, self.shown(run)),
        }
    }

    /// Whether that member may come into the spring now.
    fn may(&self, whom: Whom) -> bool {
        self.free || matches!(whom, Whom::Recruit(_))
    }

    fn press(&mut self, pressed: Option<Input>, run: &mut Run) -> bool {
        let members = run.members();
        match &mut self.page {
            SpringPage::Members => match pick(&mut self.cursor, pressed, members.len()) {
                Some(Some(row)) if !self.may(members[row]) => {
                    self.said = Some(format!(
                        "The spring's light doesn't reach {} yet: the leader and its partner evolve once their training is over.",
                        run.member_name(members[row])
                    ));
                }
                Some(Some(row)) if run.evolution_offers(members[row]).is_empty() => {
                    self.said = Some(format!("The spring is quiet... {} doesn't evolve.", run.member_name(members[row])));
                }
                Some(Some(row)) => self.page = SpringPage::Into { whom: members[row], cursor: ListCursor::default() },
                Some(None) => return true,
                None => {}
            },
            SpringPage::Into { whom, cursor } => {
                let (whom, offers) = (*whom, run.evolution_offers(*whom));
                match pick(cursor, pressed, offers.len()) {
                    Some(Some(row)) => match &offers[row].1 {
                        Ok(()) => self.page = SpringPage::Ask { whom, into: offers[row].0, cursor: ListCursor::default() },
                        Err(lacks) => self.said = Some(format!("Not yet... {lacks}")),
                    },
                    Some(None) => self.page = SpringPage::Members,
                    None => {}
                }
            }
            SpringPage::Ask { whom, into, cursor } => {
                let (whom, into) = (*whom, *into);
                match yes(cursor, pressed) {
                    Some(true) => {
                        let (from, name) = (run.member_species(whom), run.member_name(whom));
                        match run.evolve(whom, into) {
                            Ok(()) => self.evolved = Some(Evolved { whom, from, into, name }),
                            Err(why) => self.said = Some(why),
                        }
                        self.page = SpringPage::Members;
                    }
                    Some(false) => self.page = SpringPage::Into { whom, cursor: ListCursor::default() },
                    None => {}
                }
            }
        }
        false
    }

    fn shown(&self, run: &Run) -> Shown {
        let species = |id: u16| run.book.species_name(id);
        match &self.page {
            SpringPage::Members => {
                let row = |whom: Whom| {
                    let ready = self.may(whom) && run.evolution_offers(whom).iter().any(|(_, may)| may.is_ok());
                    format!("{}  Lv. {}{}", run.member_name(whom), run.member_level(whom), if ready { "  [CS:G]ready[CR]" } else { "" })
                };
                let options = run.members().into_iter().map(row).chain(["Done".to_string()]).collect();
                ("Luminous Spring. Its light shines on those ready to evolve. Who steps into the water?".to_string(), options, self.cursor.at)
            }
            SpringPage::Into { whom, cursor } => {
                let offer = |(into, may): &(u16, Result<(), String>)| format!("{}{}", species(*into), if may.is_ok() { "  [CS:G]ready[CR]" } else { "" });
                let options = run.evolution_offers(*whom).iter().map(offer).chain(["Back".to_string()]).collect();
                (format!("What does {} evolve into?", run.member_name(*whom)), options, cursor.at)
            }
            SpringPage::Ask { whom, into, cursor } => {
                (format!("Evolve {} into {}? It cannot be undone.", run.member_name(*whom), species(*into)), yes_no(), cursor.at)
            }
        }
    }
}

/// The Recycle Shop open: its trades, one asked about; the numbers it draws (its bonus tickets).
pub struct Recycle {
    cursor: ListCursor,
    asked: Option<(usize, ListCursor)>,
    said: Option<String>,
    pub draw: Draw,
}

impl Recycle {
    pub fn new(draw: Draw) -> Self {
        Recycle { cursor: ListCursor::default(), asked: None, said: None, draw }
    }

    pub fn play(&mut self, pressed: Option<Input>, run: &mut Run) -> (bool, Shown) {
        if let Some(shown) = told(&mut self.said, pressed) {
            return (false, shown);
        }
        let mut left = false;
        match &mut self.asked {
            Some((trade, cursor)) => match yes(cursor, pressed) {
                Some(true) => {
                    self.said = Some(run.recycle(*trade, &mut self.draw));
                    self.asked = None;
                }
                Some(false) => self.asked = None,
                None => {}
            },
            None => {
                let offers = run.recycle_offers();
                match pick(&mut self.cursor, pressed, offers.len()) {
                    Some(Some(row)) => self.asked = Some((offers[row], ListCursor::default())),
                    Some(None) => left = true,
                    None => {}
                }
            }
        }
        match &self.said {
            Some(said) => (left, (said.clone(), Vec::new(), 0)),
            None => (left, self.shown(run)),
        }
    }

    fn shown(&self, run: &Run) -> Shown {
        let trades = &run.book.tables.recycle;
        let name = |item: u16| run.book.item_name(item);
        if let Some((trade, cursor)) = &self.asked {
            let trade = &trades[*trade];
            return (format!("Recycle {} for a {}?", costs(trade, &name), name(trade.item)), yes_no(), cursor.at);
        }
        let offers = run.recycle_offers();
        let options = offers.iter().map(|&at| format!("{}  {}", name(trades[at].item), costs(&trades[at], &name))).chain(["Done".to_string()]).collect();
        let made = run.team().progress.recycled;
        let message = match offers.is_empty() {
            true => "The Recycle Shop has no trade to offer yet.".to_string(),
            false => format!("Welcome to the Recycle Shop! The items you don't need, traded for others. Recycled so far: {made} (Project P)."),
        };
        (message, options, self.cursor.at)
    }
}

/// What a recycle takes: so many of an item (any item, for 0).
fn costs(trade: &rules::town::RecycleTrade, name: &dyn Fn(u16) -> String) -> String {
    let cost = |&(item, count): &(u16, u16)| {
        let what = if item == 0 { "any item".to_string() } else { name(item) };
        if count > 1 { format!("{count} x {what}") } else { what }
    };
    trade.costs.iter().map(cost).collect::<Vec<_>>().join(" + ")
}

/// The prize drawing open: a ticket of the bag played.
pub struct Lottery {
    cursor: ListCursor,
    said: Option<String>,
    pub draw: Draw,
}

impl Lottery {
    pub fn new(draw: Draw) -> Self {
        Lottery { cursor: ListCursor::default(), said: None, draw }
    }

    pub fn play(&mut self, pressed: Option<Input>, run: &mut Run) -> (bool, Shown) {
        if let Some(shown) = told(&mut self.said, pressed) {
            return (false, shown);
        }
        let tickets = run.tickets();
        let left = match pick(&mut self.cursor, pressed, tickets.len()) {
            Some(Some(row)) => {
                let ticket = rules::dungeon::stack::id(run.bag().items[tickets[row]]);
                self.said = Some(format!("The drum turns... clack! {}", run.play_ticket(ticket, &mut self.draw)));
                self.cursor = ListCursor::default();
                false
            }
            Some(None) => true,
            None => false,
        };
        if let Some(said) = &self.said {
            return (left, (said.clone(), Vec::new(), 0));
        }
        let tickets = run.tickets();
        let options = tickets.iter().map(|&at| run.book.item_name(run.bag().items[at])).chain(["Done".to_string()]).collect();
        let message = match tickets.is_empty() {
            true => "The prize drawing! You have no ticket. A recycle sometimes comes with one.",
            false => "The prize drawing! The better the ticket, the better the prize. Which ticket will you play?",
        };
        (left, (message.to_string(), options, self.cursor.at))
    }
}

/// The bag open: its items, each maybe thrown away, the bag sorted.
#[derive(Default)]
pub struct Bag {
    cursor: ListCursor,
    /// An item asked about ("Throw away …?"), by its place.
    asked: Option<(usize, ListCursor)>,
    said: Option<String>,
}

impl Bag {
    /// `own`: the game's own things the team carries (told, never thrown).
    pub fn play(&mut self, pressed: Option<Input>, run: &mut Run, own: &[(String, u16)]) -> (bool, Shown) {
        if let Some(shown) = told(&mut self.said, pressed) {
            return (false, shown);
        }
        let count = run.bag().items.len();
        let mut left = false;
        match &mut self.asked {
            Some((at, cursor)) => match yes(cursor, pressed) {
                Some(true) => {
                    let name = run.book.item_name(run.bag().items[*at]);
                    run.throw_away(*at);
                    self.said = Some(format!("The {name} was thrown away."));
                    self.asked = None;
                }
                Some(false) => self.asked = None,
                None => {}
            },
            // (The items, then Sort, then Done.)
            None => match pick(&mut self.cursor, pressed, count + 1) {
                Some(Some(row)) if row < count => self.asked = Some((row, ListCursor::at(1))),
                Some(Some(_)) => {
                    run.sort_bag();
                    self.said = Some("The bag is sorted.".to_string());
                }
                Some(None) => left = true,
                None => {}
            },
        }
        if let Some(said) = &self.said {
            return (left, (said.clone(), Vec::new(), 0));
        }
        if let Some((at, cursor)) = &self.asked {
            return (left, (format!("Throw away the {}?", run.book.item_name(run.bag().items[*at])), yes_no(), cursor.at));
        }
        let items = run.bag().items.iter().map(|&value| run.book.item_name(value));
        let options = items.chain(["Sort".to_string(), "Done".to_string()]).collect();
        (left, (bag_message(run, own), options, self.cursor.at))
    }
}

/// What the bag's page says: its room, the money, the game's own things.
fn bag_message(run: &Run, own: &[(String, u16)]) -> String {
    let things: Vec<String> = own.iter().map(|(name, count)| format!("{name} x{count}")).collect();
    let things = if things.is_empty() { String::new() } else { format!("\nAlso: {}.", things.join(", ")) };
    format!("The bag: {} / {} items. {} Poké.{things}", run.bag().items.len(), run.bag_room(), run.bag().money)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_move_offers_its_link_and_its_switches_as_its_place_allows() {
        use MoveChoice::*;
        assert_eq!(move_choices(0, 3, false), [Down, Forget, Back]);
        assert_eq!(move_choices(1, 3, false), [Link, Up, Down, Forget, Back]);
        assert_eq!(move_choices(2, 3, true), [Delink, Up, Forget, Back]);
        assert_eq!(stars(577), "***");
    }

    #[test]
    fn a_recycle_tells_what_it_takes() {
        let trade = rules::town::RecycleTrade { item: 1, costs: vec![(5, 1), (0, 2)], ..Default::default() };
        assert_eq!(costs(&trade, &|item| format!("Item{item}")), "Item5 + 2 x any item");
    }
}
