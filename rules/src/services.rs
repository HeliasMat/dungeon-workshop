//! The town's services scripts open through the game's own menus: Duskull
//! Bank, Kangaskhan Storage and the two Kecleon shops.
//!
//! What they do is the original's (money and items kept safe from a
//! defeat in a dungeon, items bought and sold), and they look like it: a
//! first choice beside the box, then a titled list read by pages with the
//! money carried beside it, and in the box what the item under the cursor
//! does (the original's "Info").
// (Lists of items take several at once, marked first, and are sorted by
// item; an amount may be the most at once.)

/// The menus of `message_Menu` this module provides.
pub const KANGASKHAN_STORAGE: u16 = 12;
pub const DUSKULL_BANK: u16 = 15;
pub const KECLEON_SHOP: u16 = 18;
pub const KECLEON_WARES: u16 = 19;

/// Most items the bag holds, and most money carried or banked, as in the
/// original.
pub const BAG_CAPACITY: usize = 48;
pub(crate) const MONEY_LIMIT: i32 = 9_999_999;
/// Digits of an amount.
const DIGITS: u32 = 7;
/// Items a page of a list shows.
pub const LIST_PAGE: usize = 6;
/// The keys of a list of items, told under it.
const KEYS: &str = "Space: mark   R: sort";

/// A key, as the services read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Cancel,
    /// An item of a list marked (or not any more), to take several at
    /// once; an amount, the most it may be.
    Mark,
    /// A list of items sorted by item.
    Sort,
}

/// What the team owns: the bag and money it carries, what is kept safe.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Goods {
    pub bag: Vec<u16>,
    pub money: i32,
    pub storage: Vec<u16>,
    pub bank: i32,
    /// What the shop being visited has left to sell.
    pub stock: Vec<u16>,
}

/// What an item costs, and what it sells for (0: the shops do not take
/// it).
pub(crate) type Prices<'a> = &'a dyn Fn(u16) -> (i32, i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// The service's first choice.
    Menu,
    /// Choosing an amount, digit by digit (bank).
    Amount { deposit: bool, amount: i32, digit: u32 },
    /// Choosing items to put away, or to take back (storage); to sell,
    /// or to buy (shops).
    Items { store: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Bank,
    Storage,
    Shop,
}

/// A service being used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Service {
    kind: Kind,
    step: Step,
    cursor: usize,
    /// The items of the list marked, by their places in it.
    marked: Vec<usize>,
}

/// What a service shows: a message, choices under it, and the one the
/// cursor is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub message: String,
    pub options: Vec<String>,
    pub cursor: usize,
}

impl Service {
    /// The service of a `message_Menu` number, if it is one.
    pub fn open(menu: u16) -> Option<Self> {
        let kind = match menu {
            DUSKULL_BANK => Kind::Bank,
            KANGASKHAN_STORAGE => Kind::Storage,
            KECLEON_SHOP | KECLEON_WARES => Kind::Shop,
            _ => return None,
        };
        Some(Self { kind, step: Step::Menu, cursor: 0, marked: Vec::new() })
    }

    /// How many choices the current step has.
    fn choices(&self, goods: &Goods) -> usize {
        match self.step {
            Step::Menu => 3,
            Step::Amount { .. } => 0,
            Step::Items { store: true } => goods.bag.len(),
            Step::Items { store: false } if self.kind == Kind::Shop => goods.stock.len(),
            Step::Items { store: false } => goods.storage.len(),
        }
    }

    /// Whether the menu is one of the shops, and which (`true`: Kecleon
    /// Wares).
    pub fn shop(menu: u16) -> Option<bool> {
        match menu {
            KECLEON_SHOP => Some(false),
            KECLEON_WARES => Some(true),
            _ => None,
        }
    }

    /// What the items of the list being read are chosen for: the list's
    /// title. None when the step is not a list of items.
    pub fn title(&self) -> Option<&'static str> {
        match (self.step, self.kind) {
            (Step::Items { store: false }, Kind::Shop) => Some("Acheter"),
            (Step::Items { store: true }, Kind::Shop) => Some("Vendre"),
            (Step::Items { store: true }, _) => Some("Deposit"),
            (Step::Items { store: false }, _) => Some("Retirer"),
            _ => None,
        }
    }

    /// An amount being entered: it, the digit being changed (0: the
    /// units), and whether it is a deposit; and the most it may be.
    pub fn amount(&self) -> Option<(i32, u32, bool)> {
        match self.step {
            Step::Amount { deposit, amount, digit } => Some((amount, digit, deposit)),
            _ => None,
        }
    }

    /// Digits an amount has.
    pub const DIGITS: u32 = DIGITS;

    /// Takes a key; true once the service is left.
    pub fn press(&mut self, input: Input, goods: &mut Goods, prices: Prices) -> bool {
        let count = self.choices(goods);
        // (Marks are for the list they were made in.)
        if matches!(input, Input::Cancel) || matches!(self.step, Step::Menu) {
            self.marked.clear();
        }
        match (self.step, input) {
            // The item under the cursor marked or not; on to the next.
            (Step::Items { .. }, Input::Mark) if count > 0 => {
                match self.marked.iter().position(|&marked| marked == self.cursor) {
                    Some(at) => _ = self.marked.remove(at),
                    None => self.marked.push(self.cursor),
                }
                self.cursor = (self.cursor + 1) % count;
            }
            (Step::Items { store }, Input::Sort) => {
                let list = match (store, self.kind) {
                    (true, _) => &mut goods.bag,
                    (false, Kind::Shop) => &mut goods.stock,
                    (false, _) => &mut goods.storage,
                };
                list.sort_by_key(|&item| (crate::dungeon::stack::id(item), item));
                (self.cursor, self.marked) = (0, Vec::new());
            }
            // A list of items is read by pages too.
            (Step::Items { .. }, Input::Right) if count > 0 => {
                self.cursor = (self.cursor + LIST_PAGE).min(count - 1);
            }
            (Step::Items { .. }, Input::Left) => self.cursor = self.cursor.saturating_sub(LIST_PAGE),
            (Step::Menu | Step::Items { .. }, Input::Down) if count > 0 => {
                self.cursor = (self.cursor + 1) % count;
            }
            (Step::Menu | Step::Items { .. }, Input::Up) if count > 0 => {
                self.cursor = (self.cursor + count - 1) % count;
            }
            (Step::Menu, Input::Cancel) => return true,
            (Step::Menu, Input::Confirm) => {
                let first = self.cursor == 0;
                self.step = match (self.cursor, self.kind) {
                    (2.., _) => return true,
                    (_, Kind::Bank) => Step::Amount { deposit: first, amount: 0, digit: 0 },
                    (_, Kind::Storage) => Step::Items { store: first },
                    // A shop offers to buy first, then to sell.
                    (_, Kind::Shop) => Step::Items { store: !first },
                };
                self.cursor = 0;
            }
            (Step::Amount { deposit, amount, digit }, input) => {
                // No more than there is, nor than the other side can take.
                let (from, to) = match deposit {
                    true => (goods.money, goods.bank),
                    false => (goods.bank, goods.money),
                };
                let most = from.min(MONEY_LIMIT - to).max(0);
                let unit = 10_i32.pow(digit);
                let (amount, digit) = match input {
                    Input::Mark => (most, digit),
                    Input::Sort => (amount, digit),
                    Input::Up => ((amount + unit).min(most), digit),
                    Input::Down => ((amount - unit).max(0), digit),
                    Input::Left => (amount, (digit + 1).min(DIGITS - 1)),
                    Input::Right => (amount, digit.saturating_sub(1)),
                    Input::Confirm => {
                        let moved = if deposit { amount } else { -amount };
                        goods.money -= moved;
                        goods.bank += moved;
                        (self.step, self.cursor) = (Step::Menu, 0);
                        return false;
                    }
                    Input::Cancel => {
                        (self.step, self.cursor) = (Step::Menu, 0);
                        return false;
                    }
                };
                self.step = Step::Amount { deposit, amount, digit };
            }
            (Step::Items { .. }, Input::Cancel) => (self.step, self.cursor) = (Step::Menu, 0),
            (Step::Items { store }, Input::Confirm) if count > 0 => {
                // (The items marked, the last first so the others keep
                // their places; else the one under the cursor.)
                let mut chosen = std::mem::take(&mut self.marked);
                if chosen.is_empty() {
                    chosen.push(self.cursor);
                }
                chosen.sort_unstable_by(|a, b| b.cmp(a));
                for at in chosen.into_iter().filter(|&at| at < count) {
                    self.take(store, at, goods, prices);
                }
                // Nothing left to move: back to the first choice.
                match self.choices(goods) {
                    0 => (self.step, self.cursor) = (Step::Menu, 0),
                    left => self.cursor = self.cursor.min(left - 1),
                }
            }
            _ => {}
        }
        false
    }

    /// One item of the list moved: sold, bought, put away or taken back,
    /// if it may be.
    fn take(&self, store: bool, at: usize, goods: &mut Goods, prices: Prices) {
        let room = goods.bag.len() < BAG_CAPACITY;
        match (store, self.kind) {
            // Selling: what the shop gives for it, if it takes it.
            (true, Kind::Shop) => {
                let (_, gain) = prices(goods.bag[at]);
                if gain > 0 {
                    goods.bag.remove(at);
                    goods.money = (goods.money + gain).min(MONEY_LIMIT);
                }
            }
            // Buying: with the money for it, and room in the bag.
            (false, Kind::Shop) => {
                let (cost, _) = prices(goods.stock[at]);
                if room && goods.money >= cost {
                    goods.money -= cost;
                    goods.bag.push(goods.stock.remove(at));
                }
            }
            (true, _) => goods.storage.push(goods.bag.remove(at)),
            (false, _) if room => goods.bag.push(goods.storage.remove(at)),
            (false, _) => {}
        }
    }

    /// The item under the cursor of a list.
    fn under_cursor(&self, goods: &Goods) -> Option<u16> {
        let list = match (self.step, self.kind) {
            (Step::Items { store: false }, Kind::Shop) => &goods.stock,
            (Step::Items { store: true }, _) => &goods.bag,
            (Step::Items { store: false }, _) => &goods.storage,
            _ => return None,
        };
        list.get(self.cursor).copied()
    }

    /// What to show; `item` names an item, `about` tells what it does.
    pub fn view(&self, goods: &Goods, item: impl Fn(u16) -> String, about: impl Fn(u16) -> String, prices: Prices) -> View {
        // (An item marked: a sign before it.)
        let item = |at: usize, id: u16| match self.marked.contains(&at) {
            true => format!("+ {}", item(id)),
            false => item(id),
        };
        let names = |items: &[u16]| items.iter().enumerate().map(|(at, &id)| item(at, id)).collect::<Vec<_>>();
        // In a shop, items come with what they cost or sell for.
        let priced = |items: &[u16], selling: bool| -> Vec<String> {
            let line = |(at, &id): (usize, &u16)| match (prices(id), selling) {
                ((_, 0), true) => format!("{}  [CS:E]-[CR]", item(at, id)),
                ((_, gain), true) => format!("{}  [CS:G]{gain}[CR]", item(at, id)),
                ((cost, _), false) => format!("{}  [CS:G]{cost}[CR]", item(at, id)),
            };
            items.iter().enumerate().map(line).collect()
        };
        let options = |list: [&str; 3]| list.iter().map(|option| option.to_string()).collect();
        let pocket = format!("Poké on hand: [CS:G]{}[CR]", goods.money);
        // (What the item under the cursor does, or what is asked.)
        let told = |asked: &str| self.under_cursor(goods).map(&about).filter(|about| !about.is_empty()).unwrap_or(asked.to_string());
        let (message, options) = match (self.step, self.kind) {
            (Step::Menu, Kind::Shop) => (pocket, options(["Buy", "Sell", "Quit"])),
            (Step::Items { store: false }, Kind::Shop) => {
                let message = match (goods.stock.is_empty(), goods.bag.len() < BAG_CAPACITY) {
                    (true, _) => "Everything is sold!".to_string(),
                    (_, false) => "The bag is full.".to_string(),
                    _ => told("Buy which item?"),
                };
                (format!("{message}\n{KEYS}"), priced(&goods.stock, false))
            }
            (Step::Items { store: true }, Kind::Shop) => (format!("{}\n{KEYS}", told("Sell which item?")), priced(&goods.bag, true)),
            (Step::Menu, Kind::Bank) => {
                (format!("Poké on hand: [CS:G]{}[CR]\nPoké in the bank: [CS:G]{}[CR]", goods.money, goods.bank), options(["Deposit", "Withdraw", "Quit"]))
            }
            (Step::Menu, Kind::Storage) => (
                format!("Items in the bag: {} / {BAG_CAPACITY}\nItems in storage: {}", goods.bag.len(), goods.storage.len()),
                options(["Deposit items", "Withdraw items", "Quit"]),
            ),
            (Step::Amount { deposit, amount, digit }, _) => {
                // (The amount is shown in its own window, digit by digit:
                // see `amount`.)
                let _ = (amount, digit);
                let verb = if deposit { "Deposit" } else { "Withdraw" };
                (format!("{verb} how many Poké?\nUp/Down: digit   Left/Right: position\nSpace: all"), Vec::new())
            }
            (Step::Items { store: true }, _) => (format!("{}\n{KEYS}", told("Deposit which item?")), names(&goods.bag)),
            (Step::Items { store: false }, _) => match goods.bag.len() < BAG_CAPACITY {
                true => (format!("{}\n{KEYS}", told("Withdraw which item?")), names(&goods.storage)),
                false => (format!("The bag is full.\n{KEYS}"), names(&goods.storage)),
            },
        };
        View { message, options, cursor: self.cursor }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_of_items_have_a_title_and_pages() {
        let prices: Prices = &|_| (10, 5);
        let mut goods = Goods { bag: (1..=20).collect(), ..Default::default() };
        let mut storage = Service::open(KANGASKHAN_STORAGE).unwrap();
        assert_eq!(storage.title(), None);
        storage.press(Input::Confirm, &mut goods, prices);
        assert_eq!(storage.title(), Some("Deposit"));
        // A page on, as far as the last item, and back.
        for expected in [6, 12, 18, 19, 13] {
            let key = if expected == 13 { Input::Left } else { Input::Right };
            storage.press(key, &mut goods, prices);
            assert_eq!(storage.view(&goods, |item| item.to_string(), |_| String::new(), prices).cursor, expected);
        }
        let mut shop = Service::open(KECLEON_SHOP).unwrap();
        shop.press(Input::Confirm, &mut goods, prices);
        assert_eq!(shop.title(), Some("Acheter"));
    }

    fn goods() -> Goods {
        Goods { bag: vec![1, 2, 3], money: 350, storage: vec![9], bank: 1000, stock: vec![20, 30, 40] }
    }

    /// Items cost ten times their number and sell for their number; item 2
    /// cannot be sold.
    fn prices(id: u16) -> (i32, i32) {
        (id as i32 * 10, if id == 2 { 0 } else { id as i32 })
    }

    fn presses(service: &mut Service, goods: &mut Goods, inputs: &[Input]) -> bool {
        inputs.iter().any(|&input| service.press(input, goods, &prices))
    }

    #[test]
    fn a_shop_sells_what_can_be_paid_and_buys_what_it_takes() {
        use Input::*;
        let (mut service, mut goods) = (Service::open(KECLEON_SHOP).unwrap(), goods());
        // Buy: item 20 costs 200 of the 350; then 30 is too dear.
        presses(&mut service, &mut goods, &[Confirm]);
        let view = service.view(&goods, |id| format!("item {id}"), |_| String::new(), &prices);
        assert_eq!(view.options[0], "item 20  [CS:G]200[CR]");
        presses(&mut service, &mut goods, &[Confirm, Confirm]);
        assert_eq!((goods.bag.clone(), goods.money, goods.stock.clone()), (vec![1, 2, 3, 20], 150, vec![30, 40]));
        // Sell: item 1 for 1; item 2 is not taken.
        presses(&mut service, &mut goods, &[Cancel, Down, Confirm, Confirm, Confirm]);
        assert_eq!((goods.bag.clone(), goods.money), (vec![2, 3, 20], 151));
        assert!(presses(&mut service, &mut goods, &[Cancel, Cancel]));
    }

    #[test]
    fn the_bank_takes_and_gives_no_more_than_there_is() {
        use Input::*;
        let (mut service, mut goods) = (Service::open(DUSKULL_BANK).unwrap(), goods());
        // Deposit: 120 chosen digit by digit.
        presses(&mut service, &mut goods, &[Confirm, Left, Up, Up, Left, Up]);
        assert_eq!(service.amount(), Some((120, 2, true)));
        assert!(!presses(&mut service, &mut goods, &[Confirm]));
        assert_eq!((goods.money, goods.bank), (230, 1120));
        // Withdraw: asking for 9000 gives what the bank holds.
        presses(&mut service, &mut goods, &[Down, Confirm, Left, Left, Left]);
        presses(&mut service, &mut goods, &[Up; 9]);
        presses(&mut service, &mut goods, &[Confirm]);
        assert_eq!((goods.money, goods.bank), (1350, 0));
        // Cancelling an amount changes nothing; the last choice leaves.
        presses(&mut service, &mut goods, &[Confirm, Up, Cancel]);
        assert_eq!((goods.money, goods.bank), (1350, 0));
        assert!(presses(&mut service, &mut goods, &[Down, Down, Confirm]));
    }

    #[test]
    fn the_storage_moves_items_both_ways() {
        use Input::*;
        let (mut service, mut goods) = (Service::open(KANGASKHAN_STORAGE).unwrap(), goods());
        let view = service.view(&goods, |id| format!("item {id}"), |_| String::new(), &prices);
        assert!(view.message.contains("3 / 48") && view.options.len() == 3);
        // Store the second item, then the last.
        presses(&mut service, &mut goods, &[Confirm, Down, Confirm, Confirm]);
        assert_eq!((goods.bag.clone(), goods.storage.clone()), (vec![1], vec![9, 2, 3]));
        let view = service.view(&goods, |id| format!("item {id}"), |_| String::new(), &prices);
        assert_eq!((view.options, view.cursor), (vec!["item 1".to_string()], 0));
        // Take the first stored item back.
        presses(&mut service, &mut goods, &[Cancel, Down, Confirm, Confirm]);
        assert_eq!((goods.bag.clone(), goods.storage.clone()), (vec![1, 9], vec![2, 3]));
        // A full bag takes nothing more.
        goods.bag = vec![5; BAG_CAPACITY];
        presses(&mut service, &mut goods, &[Confirm]);
        assert_eq!(goods.storage, [2, 3]);
        assert!(presses(&mut service, &mut goods, &[Cancel, Cancel]));
    }

    #[test]
    fn several_items_at_once_and_lists_sorted() {
        let mut goods = goods();
        let mut storage = Service::open(KANGASKHAN_STORAGE).unwrap();
        let bag = goods.bag.clone();
        // Put away: the first and the third, marked.
        presses(&mut storage, &mut goods, &[Input::Confirm, Input::Mark, Input::Down, Input::Mark, Input::Confirm]);
        assert_eq!(goods.bag.len(), bag.len() - 2);
        assert!(goods.storage.ends_with(&[bag[2], bag[0]]), "{:?}", goods.storage);
        // Sorted by item.
        presses(&mut storage, &mut goods, &[Input::Sort]);
        let mut sorted = goods.bag.clone();
        sorted.sort_by_key(|&item| (crate::dungeon::stack::id(item), item));
        assert_eq!(goods.bag, sorted);
        // The bank: Mark, the most there is.
        let mut bank = Service::open(DUSKULL_BANK).unwrap();
        let (money, banked) = (goods.money, goods.bank);
        presses(&mut bank, &mut goods, &[Input::Confirm, Input::Mark, Input::Confirm]);
        assert_eq!((goods.money, goods.bank), (0, banked + money));
    }

    #[test]
    fn other_menus_are_not_services() {
        assert_eq!(Service::open(16), None);
    }
}
