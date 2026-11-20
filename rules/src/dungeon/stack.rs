//! Items that pile up: the thrown ones (sticks, thorns, spikes, rocks).
//! A pile is one item of the bag or of the floor, its number in the high
//! bits of its value (item numbers stay under 2048): so the bag, the
//! storage and the saves keep holding plain numbers.

/// The bits of an item's number, and the most a pile holds.
pub(crate) const ID_BITS: u32 = 11;
const ID_MASK: u16 = (1 << ID_BITS) - 1;
pub const MOST: u16 = 31;
/// The last of the items that pile up (`status::item::LAST_PROJECTILE`).
const LAST_PILING: u16 = 10;

/// A sticky item (one that does not pile up: the bits a pile's count
/// would take): it cannot be used, thrown, put down or given.
const STICKY: u16 = 1 << ID_BITS;

/// Whether the item is sticky.
pub(crate) fn sticky(value: u16) -> bool {
    !piles(value) && value & STICKY != 0
}

/// The item made sticky (a pile stays as it is), or cleaned.
pub fn stuck(value: u16, sticky: bool) -> u16 {
    match (piles(value), sticky) {
        (true, _) => value,
        (false, true) => value | STICKY,
        (false, false) => value & !STICKY,
    }
}

/// A Used TM (`ITEM_TM_USED_TM`) keeps the TM it was, as the original's
/// keeps it in its quantity: its number, with this bit.
const USED: u16 = 1 << (ID_BITS + 1);
pub(crate) const USED_TM: u16 = 187;

/// The item a value stands for.
pub fn id(value: u16) -> u16 {
    let raw = value & ID_MASK;
    match !(1..=LAST_PILING).contains(&raw) && value & USED != 0 {
        true => USED_TM,
        false => raw,
    }
}

/// The Used TM a TM leaves.
pub(crate) fn used_tm(tm: u16) -> u16 {
    id(tm) | USED
}

/// The TM a Used TM was (Recycle makes it again).
pub(crate) fn recycled(value: u16) -> Option<u16> {
    (id(value) == USED_TM && value & USED != 0).then_some(value & ID_MASK)
}

/// How many a pile holds (1 for an item that does not pile up).
pub fn count(value: u16) -> u16 {
    (value >> ID_BITS).max(1)
}

/// Whether the item piles up.
pub fn piles(item: u16) -> bool {
    (1..=LAST_PILING).contains(&id(item))
}

/// A pile of `count` of the item.
pub fn pile(item: u16, count: u16) -> u16 {
    match piles(item) {
        true => id(item) | count.clamp(1, MOST) << ID_BITS,
        false => id(item),
    }
}

/// Puts a pile in the bag: with a pile of the same item there, as long as
/// it holds no more than [`MOST`].
pub fn add(bag: &mut Vec<u16>, value: u16) {
    if piles(value)
        && let Some(slot) = bag.iter_mut().find(|slot| id(**slot) == id(value) && count(**slot) + count(value) <= MOST)
    {
        *slot = pile(value, count(*slot) + count(value));
        return;
    }
    bag.push(value);
}

/// Takes one of the pile at `index` of the bag (the whole slot when it was
/// the last): the item taken.
pub(crate) fn take_one(bag: &mut Vec<u16>, index: usize) -> u16 {
    let value = bag[index];
    match count(value) {
        1 => {
            bag.remove(index);
        }
        left => bag[index] = pile(value, left - 1),
    }
    id(value)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_used_tm_remembers_its_tm() {
        use super::*;
        let used = used_tm(200);
        assert_eq!(id(used), USED_TM);
        assert_eq!(recycled(used), Some(200));
        assert_eq!(recycled(USED_TM), None);
        let stuck_used = stuck(used, true);
        assert!(sticky(stuck_used) && id(stuck_used) == USED_TM);
        assert_eq!(stuck(stuck_used, false), used);
    }

    use super::*;

    #[test]
    fn piles_merge_and_are_thrown_one_by_one() {
        const IRON_THORN: u16 = 2;
        const APPLE: u16 = 109;
        let mut bag = vec![APPLE];
        add(&mut bag, pile(IRON_THORN, 5));
        add(&mut bag, pile(IRON_THORN, 3));
        assert_eq!(bag.len(), 2);
        assert_eq!((id(bag[1]), count(bag[1])), (IRON_THORN, 8));
        assert_eq!(take_one(&mut bag, 1), IRON_THORN);
        assert_eq!(count(bag[1]), 7);
        assert_eq!(take_one(&mut bag, 0), APPLE);
        assert_eq!(bag.len(), 1);
        // An apple does not pile up.
        add(&mut bag, APPLE);
        add(&mut bag, APPLE);
        assert_eq!(bag.len(), 3);
    }

    #[test]
    fn sticky_items_keep_their_number() {
        const APPLE: u16 = 109;
        const IRON_THORN: u16 = 2;
        let apple = stuck(APPLE, true);
        assert!(sticky(apple) && id(apple) == APPLE && count(apple) == 1);
        assert_eq!(stuck(apple, false), APPLE);
        // A pile is never sticky.
        assert!(!sticky(stuck(pile(IRON_THORN, 3), true)));
    }
}
