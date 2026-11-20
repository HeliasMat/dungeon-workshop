//! What the game counts with: `dungeons.bin`, and the tilesets of its floors.

use rules::tables::{DungeonTable, Species, Tables};
use shell::art::{CHUNK, CHUNKS_IN_ROW as ROW};

use super::pictures::episode_characters;
use super::rom_data::{Interface, RomData};
use super::wrong;

/// Chunks side by side in a picture, `ROW` a row: so many bytes a pixel.
fn atlas(chunks: &[Vec<u8>], bytes: usize) -> (Vec<u8>, usize) {
    let rows = chunks.len().div_ceil(ROW).max(1);
    let mut pixels = vec![0u8; ROW * CHUNK * rows * CHUNK * bytes];
    for (at, chunk) in chunks.iter().enumerate() {
        for (y, line) in chunk.chunks_exact(CHUNK * bytes).enumerate().take(CHUNK) {
            let to = (((at / ROW) * CHUNK + y) * ROW * CHUNK + (at % ROW) * CHUNK) * bytes;
            pixels[to..to + CHUNK * bytes].copy_from_slice(line);
        }
    }
    (pixels, rows)
}

/// Writes a tileset: its chunks side by side in a picture, and which chunk
/// each tile takes; with which palette and colour each pixel shows
/// (`<id>_i.png`) and the palettes that change with time (`<id>.cycles`),
/// for the water and the lava that move (the `decor` builder's dungeon rooms).
fn write_tileset(folder: &str, id: u8, chunks: &[Vec<u8>], colours: &[Vec<u8>], cycles: &[shell::art_files::Cycle], chunk_for: &[u16]) -> std::io::Result<()> {
    let (pixels, rows) = atlas(chunks, 4);
    let name = format!("{folder}/tilesets/{id}");
    image::save_buffer(format!("{name}.png"), &pixels, (ROW * CHUNK) as u32, (rows * CHUNK) as u32, image::ColorType::Rgba8).map_err(wrong)?;
    let (indices, _) = atlas(colours, 1);
    image::save_buffer(format!("{name}_i.png"), &indices, (ROW * CHUNK) as u32, (rows * CHUNK) as u32, image::ColorType::L8).map_err(wrong)?;
    std::fs::write(format!("{name}.cycles"), bincode::serialize(cycles).map_err(wrong)?)?;
    let table = bincode::serialize(&(chunks.len(), chunk_for)).map_err(wrong)?;
    std::fs::write(format!("{name}.bin"), table)
}

/// The dungeons' tables and tilesets, as `<folder>/dungeons.bin` and
/// `<folder>/tilesets/` (`shell::art`). `interface` is what the ROM
/// sets up. How many dungeons, floors and tilesets were written.
pub fn dungeons(data: &RomData, interface: &Interface, folder: &str) -> std::io::Result<(usize, usize, usize)> {
    std::fs::create_dir_all(format!("{folder}/tilesets"))?;
    let mut dungeons = Vec::new();
    for id in 0..data.dungeons.dungeons.len() {
        let floors = RomData::floor_range(&data.mappa, &data.dungeons, id);
        // (The first floor's song, as `RomData::load` has it.)
        let music = floors.first().and_then(|floor| data.dungeons.songs(floor.layout.music).first().copied());
        dungeons.push(DungeonTable {
            music: music.unwrap_or(super::rom_data::NO_SONG),
            goes_up: data.dungeons.goes_up.get(id).copied().unwrap_or(false),
            rules: data.dungeons.rules.get(id).copied().unwrap_or_default(),
            floors: floors.iter().map(|floor| data.floor(floor)).collect(),
        });
    }
    let species = (0..data.monsters.species_count()).map(|monster| Species {
        name: data.monster_name(monster),
        movement: data.movement_type(monster),
        types: data.monster_types(monster as u16),
        shadow: data.shadow_size(monster as u16),
        weight: data.monsters.species(monster).map_or(0x100, |species| species.weight),
        family: data.monsters.species(monster).map_or(monster as u16, |species| species.family),
        evolution: data.monsters.species(monster).map_or_else(Default::default, |species| species.evolution),
        gender: data.monsters.species(monster).map_or(0, |species| species.gender),
        base: data.monsters.species(monster).map_or([0; 5], |species| {
            [species.base_hp as i32, species.base_attack as i32, species.base_defense as i32, species.base_sp_attack as i32, species.base_sp_defense as i32]
        }),
        learnset: data.learnset(monster).to_vec(),
        machines: data.machines(monster).to_vec(),
        growth: data.monsters.growth_table(monster).to_vec(),
        hero: data.monster(monster, super::rom_data::PLAYER_LEVEL),
        partner: data.monster(monster, super::rom_data::PARTNER_LEVEL),
    });
    let lists = rom_formats::script::load_item_tables(&data.rom).unwrap_or_default();
    let shop = |wares: usize, stage: usize| {
        let list = lists.get((data.shop_tables[wares][stage] as usize).saturating_sub(1));
        let with_category = |&(id, weight): &(u16, u16)| Some((id, data.items.category(id as usize)?, weight));
        list.map_or(rules::dungeon::ItemTable::default(), |list| rules::dungeon::ItemTable {
            categories: list.categories.clone(),
            items: list.items.iter().filter_map(with_category).collect(),
        })
    };
    let groups: Vec<(u8, u8, u8)> = data.dungeons.dungeons.iter().map(|dungeon| (dungeon.group, dungeon.first_floor, dungeon.floors)).collect();
    let tables = Tables {
        jobs: rom_formats::types::load_job_tables(&data.rom, &data.names.explorer_ranks, &groups).unwrap_or_default(),
        quiz: data.quiz().map(|quiz| (*quiz).clone()),
        episode_characters: episode_characters(data),
        map_markers: (0..310).map(|marker| data.map_marker(marker)).collect(),
        item_prices: (0..data.names.items.len()).map(|id| data.item_prices(id as u16)).collect(),
        recycle: rom_formats::types::load_recycle_trades(&data.rom).unwrap_or_default(),
        sentry: rom_formats::types::load_sentry_visitors(&data.rom).unwrap_or_default(),
        shops: (0..2).map(|wares| (0..4).map(|stage| shop(wares, stage)).collect()).collect(),
        types: interface.types.clone(),
        iq: rom_formats::types::load_iq_tables(&data.rom).unwrap_or_default(),
        exclusive: rom_formats::types::load_exclusive_items(&data.rom).unwrap_or_default(),
        recruit_boosts: rom_formats::types::load_recruit_boosts(&data.rom).unwrap_or_default(),
        move_slots: (0..data.moves.move_count()).filter_map(|id| Some(super::rom_data::move_slot(id as u16, data.moves.get(id)?))).collect(),
        item_moves: (0..data.names.items.len()).map(|item| data.items.item_move(item)).collect(),
        item_piles: (0..data.names.items.len()).map(|item| data.items.pile(item)).collect(),
        constants: interface.constants,
        moves: data.names.moves.clone(),
        items: data.names.items.clone(),
        item_descriptions: data.names.item_descriptions.clone(),
        iq_skills: data.names.iq_skills.clone(),
        tactics: data.names.tactics.clone(),
        tactic_descriptions: data.names.tactic_descriptions.clone(),
        trap_art: interface.trap_art.clone(),
        stairs_tile: interface.stairs_tile,
        weather_colors: interface.weather_colors.clone(),
        item_art: interface.item_art.clone(),
        regular_attack: interface.regular_attack,
        money_table: interface.money_table.clone(),
        team: (super::rom_data::PLAYER_MD_INDEX as u16, super::rom_data::PARTNER_MD_INDEX as u16),
        species: species.collect(),
        playable: data.playable.iter().map(|&id| id as u16).collect(),
        dungeon_names: (0..dungeons.len()).map(|id| data.dungeon_name(id as u16)).collect(),
        dungeons,
        move_effects: (0..data.names.moves.len()).map(|id| data.move_effects(id as u16)).collect(),
        move_sounds: (0..data.names.moves.len()).map(|id| data.move_sound(id as u16)).collect(),
        move_motions: (0..data.names.moves.len()).map(|id| rom_formats::script::load_move_motion(&data.rom, id as u16).unwrap_or_default()).collect(),
        trap_effects: (0..32).map(|trap| data.trap_effect(trap)).collect(),
        item_effects: (0..data.names.items.len()).map(|id| data.item_effects(id as u16)).collect(),
    };
    let packed = bincode::serialize(&tables).map_err(|error| std::io::Error::other(error.to_string()))?;
    std::fs::write(format!("{folder}/dungeons.bin"), packed)?;
    // The tilesets the floors use.
    let mut used: Vec<u8> = tables.dungeons.iter().flat_map(|dungeon| dungeon.floors.iter().map(|floor| floor.tileset)).collect();
    used.sort();
    used.dedup();
    let mut written = 0;
    for id in used {
        use rom_formats::tileset::{TILESET_COUNT, Tileset};
        let Some(pack) = data.rom.file("DUNGEON/dungeon.bin") else {
            continue;
        };
        // (A boss's floor is a whole picture: its grid of chunks stands
        // for the table.)
        let set = match (id as usize).checked_sub(TILESET_COUNT) {
            None => Tileset::load(pack, id as usize).ok().map(|set| (set.chunk_table(), set)),
            Some(picture) => Tileset::load_picture(pack, picture).ok().map(|(set, layout)| (layout, set)),
        };
        let Some((chunk_for, set)) = set else {
            continue;
        };
        let chunks: Vec<Vec<u8>> = (0..set.chunk_count()).map(|chunk| set.render_chunk(chunk)).collect();
        let colours: Vec<Vec<u8>> = (0..set.chunk_count()).map(|chunk| set.chunk_colours(chunk)).collect();
        write_tileset(folder, id, &chunks, &colours, set.cycles(), &chunk_for)?;
        written += 1;
    }
    Ok((tables.dungeons.len(), tables.dungeons.iter().map(|dungeon| dungeon.floors.len()).sum(), written))
}
