//! What team members say by personality (`talk.txt`) and the guests of the
//! story's dungeons (`guests.txt`).

use super::rom_data::{LANGUAGE, RomData};
use super::{text, wrong};

/// `talk.txt`: `monster <number> <name> <group>` for each monster,
/// `special <index> <group>` for the story's own characters (see
/// `rom_formats::talk`), `say <group> <kind> <number> <string> <text>` for each
/// dialogue (kind: `rom_formats::talk::KINDS`; string: its index in the text
/// files). How many monsters and dialogues were written.
pub fn talk(data: &RomData, folder: &str) -> std::io::Result<(usize, usize)> {
    let talk = rom_formats::talk::Talk::parse(data.rom.file("MESSAGE/tbl_talk.tlk").ok_or_else(|| wrong("MESSAGE/tbl_talk.tlk is missing"))?).map_err(wrong)?;
    let strings = data.rom.file(&format!("MESSAGE/text_{LANGUAGE}.str")).and_then(|file| rom_formats::text::Strings::parse(file).ok());
    let region = data.rom.region().map_err(wrong)?;
    let mut lines = vec!["# monster <number> <name> <group>; special <index> <group>; say <group> <kind> <number> <string> <text>".to_string()];
    lines.extend(talk.monsters.iter().enumerate().map(|(monster, group)| format!("monster\t{monster}\t{}\t{group}", text(&data.monster_name(monster)))));
    lines.extend(talk.special.iter().enumerate().map(|(index, group)| format!("special\t{index}\t{group}")));
    let mut said = 0;
    for (group, kinds) in talk.groups.iter().enumerate() {
        for (kind, numbers) in kinds.iter().enumerate() {
            for &number in numbers {
                let index = rom_formats::talk::string_index(region, kind, number);
                let words = index.and_then(|index| strings.as_ref()?.get(index)).unwrap_or_default();
                let index = index.map_or("-".to_string(), |index| index.to_string());
                lines.push(format!("say\t{group}\t{}\t{number}\t{index}\t{}", rom_formats::talk::KINDS[kind], text(words)));
                said += 1;
            }
        }
    }
    std::fs::write(format!("{folder}/talk.txt"), lines.join("\n") + "\n")?;
    Ok((talk.monsters.len(), said))
}

/// `guests.txt`: `guest <index> <monster> <name> <level> <hp> <attack>
/// <sp. attack> <defense> <sp. defense> <iq> <exp> <moves ,> <joined at:
/// dungeon> <floor>` for each guest, then `dungeon
/// <number> <first guest> <second guest> <flags ,>` for each dungeon that
/// does something (`-`: none; flags: `hidden_land`, `hidden_land_cleared`,
/// `no_recruiting`, `before_side01_boss2nd`; see `rom_formats::guests`). How many
/// guests and dungeons were written.
pub fn guests(data: &RomData, folder: &str) -> std::io::Result<(usize, usize)> {
    let found = rom_formats::guests::load(&data.rom).map_err(wrong)?;
    let mut lines = vec![
        "# guest <index> <monster> <name> <level> <hp> <attack> <sp_attack> <defense> <sp_defense> <iq> <exp> <moves> <joined_at> <floor>".to_string(),
        "# dungeon <number> <first guest> <second guest> <flags>".to_string(),
    ];
    if found.patched {
        lines.push("# (the ROM has SkyTemple's EditGuestPokemon patch: its tables)".to_string());
    }
    for (index, guest) in found.guests.iter().enumerate() {
        let moves: Vec<String> = guest.moves.iter().map(u16::to_string).collect();
        lines.push(format!(
            "guest\t{index}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            guest.monster,
            text(&data.monster_name(guest.monster as usize)),
            guest.level,
            guest.hp,
            guest.attack,
            guest.sp_attack,
            guest.defense,
            guest.sp_defense,
            guest.iq,
            guest.exp,
            moves.join(","),
            guest.joined_at,
            guest.joined_floor,
        ));
    }
    let mut dungeons = 0;
    for (number, dungeon) in found.dungeons.iter().enumerate() {
        let flags: Vec<&str> = [
            (dungeon.hidden_land_rules, "hidden_land"),
            (dungeon.hidden_land_rules_cleared, "hidden_land_cleared"),
            (dungeon.no_recruiting, "no_recruiting"),
            (dungeon.only_before_side01_boss2nd, "before_side01_boss2nd"),
        ]
        .into_iter()
        .filter_map(|(set, flag)| set.then_some(flag))
        .collect();
        if dungeon.guests == [None, None] && flags.is_empty() {
            continue;
        }
        let guest = |slot: Option<usize>| slot.map_or("-".to_string(), |index| index.to_string());
        let flags = if flags.is_empty() { "-".to_string() } else { flags.join(",") };
        lines.push(format!("dungeon\t{number}\t{}\t{}\t{flags}", guest(dungeon.guests[0]), guest(dungeon.guests[1])));
        dungeons += 1;
    }
    std::fs::write(format!("{folder}/guests.txt"), lines.join("\n") + "\n")?;
    Ok((found.guests.len(), dungeons))
}
