//! A shop on the floor: its goods laid out before its keeper, bought or
//! stolen.

use super::*;

/// How the goods of a shop lie before its keeper: so many a row, so far
/// apart, so far below him.
pub(super) const SHELF: (usize, i32, i32) = (3, 26, 34);

/// `shop kecleon Apple 50 Oran Berry 30 Escape Orb 200`: a shop on the
/// floor: its goods laid out in rows before its keeper, each with its price.
pub(super) fn shop(film: &mut Film, line: &Line) {
    // (`pikachu shop buy Apple`: he buys it, as `buy`.)
    if line.args.has("buy") {
        buys(film, line);
        return;
    }
    let keeper = film_core::verbs::who_said(line);
    let keeper = keeper.as_str();
    let mut goods: Vec<(film_core::names::things::Thing, i32)> = Vec::new();
    let mut words: Vec<String> = Vec::new();
    for word in line.args.words("goods") {
        match word.parse::<i32>() {
            Ok(price) if !words.is_empty() => {
                match known_thing(film, &words) {
                    Some(thing) => goods.push((thing, price)),
                    None => film.wrong(line.number, format!("unknown item in the shop: {}", words.join(" "))),
                }
                words.clear();
            }
            _ => words.push(word.clone()),
        }
    }
    if goods.is_empty() || !words.is_empty() {
        film.wrong(line.number, "shop <shopkeeper> <item> <price> <item> <price>...".to_string());
        return;
    }
    let mut laid = Vec::new();
    for (at, (thing, price)) in goods.iter().enumerate() {
        let (row, column) = (at / SHELF.0, at % SHELF.0);
        let dx = (column as i32 - (SHELF.0 as i32 - 1) / 2) * SHELF.1;
        let Some(spot) = film.spot_by(keeper, dx as f32, (SHELF.2 + row as i32 * SHELF.1) as f32) else {
            return;
        };
        let code = thing_out(film, thing);
        let at = found(film, &code, spot, &["exact"]);
        film.put_point_at(&code, at);
        arrives(film, &code, thing);
        laid.push((thing.key.clone(), code, *price));
    }
    film.kept::<DungeonState>().shop = Some((keeper.to_string(), laid));
    let name = film.called(keeper);
    logged(film, &format!("{name}: Welcome to my shop!"));
}

/// The thing a line names among the shop's goods, and its price.
pub(super) fn on_sale(film: &mut Film, line: &Line) -> Option<(String, String, i32)> {
    let thing = Some(line.args.words("thing")).filter(|words| !words.is_empty()).and_then(|words| known_thing(film, words));
    let found = thing.as_ref().and_then(|thing| film.kept::<DungeonState>().shop.as_ref()?.1.iter().find(|(key, ..)| *key == thing.key).cloned());
    if found.is_none() {
        film.wrong(line.number, "not in the shop: shop <shopkeeper> <item> <price>... first".to_string());
    }
    found
}

/// `buy a Apple`: he goes to it, pays, picks it up; the keeper is glad.
pub(super) fn buys(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let who = who.as_str();
    let Some((key, code, price)) = on_sale(film, line) else {
        return;
    };
    let keeper = film.kept::<DungeonState>().shop.as_ref().map(|(keeper, _)| keeper.clone()).unwrap_or_default();
    if let Some(at) = film.things.stuff.iter().position(|stuff| stuff.code == code) {
        take_up(film, who, at);
    }
    film.on(who, |who| emit::popup(who, emit::colour(engine::Colour::Yellow), format!("-{price} P")));
    film.beat(emit::jingle(7));
    film.emote(&keeper, "joy", Some(40.0));
    let thing = known_thing(film, std::slice::from_ref(&key)).map_or(key, |thing| thing.name);
    let name = film.called(who);
    logged(film, &format!("{name} bought the {thing} for {price} Poké!"));
    if let Some((_, goods)) = film.kept::<DungeonState>().shop.as_mut() {
        goods.retain(|(_, laid, _)| *laid != code);
    }
    film.wait_frames(40, line.wait);
}

/// `steal a Apple`: he takes it without paying: the keeper, furious, cries
/// thief; the ground shakes.
pub(super) fn steals(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let who = who.as_str();
    let Some((_, code, _)) = on_sale(film, line) else {
        return;
    };
    let keeper = film.kept::<DungeonState>().shop.as_ref().map(|(keeper, _)| keeper.clone()).unwrap_or_default();
    if let Some(at) = film.things.stuff.iter().position(|stuff| stuff.code == code) {
        take_up(film, who, at);
    }
    film.wait(20);
    film.face(&keeper, who);
    film.emote(&keeper, "anger", Some(60.0));
    film.sound("alert");
    film.beat(emit::quake(3.0, 40.0));
    film.on(&keeper, |who| emit::tremble(who, 40.0, 3.0));
    film.on(&keeper, |who| emit::popup(who, emit::colour(engine::Colour::Red), "Thief!"));
    let name = film.called(&keeper);
    logged(film, &format!("{name}: Thief! Give that back!"));
    film.wait_frames(60, line.wait);
}
