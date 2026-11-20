//! The places of the ROM: maps, who stands there, the scenes of a level.

use std::collections::HashSet;
use std::sync::Arc;

use rom_formats::mapbg::MapBackground;
use rom_formats::scene::Scene;
use rom_formats::script::{Level, Script};
use rom_formats::tileset::Tileset;
use rom_formats::vm::{self, Program};

use super::{LANGUAGE, PARTNER_MD_INDEX, PLAYER_MD_INDEX, RomData, SpriteArt};

/// A palette of a map whose colours change with time: every `ticks` 60ths
/// of a second it takes the next of `colours`.
#[derive(Clone)]
pub(crate) struct PaletteCycle {
    pub palette: usize,
    pub ticks: u16,
    pub colours: Vec<[[u8; 3]; 16]>,
}

/// A level's map, decoded.
pub(crate) struct PlaceArt {
    /// In pixels.
    pub width: usize,
    pub height: usize,
    /// One RGBA picture per frame of its animated tiles: the layer
    /// characters walk on.
    pub frames: Vec<Vec<u8>>,
    /// The same for the layer they walk under, and which palette and
    /// colour its pixels show (see `indices`).
    pub upper: Vec<Vec<u8>>,
    pub upper_indices: Vec<Vec<u8>>,
    /// How long each frame is shown.
    pub frame_seconds: f32,
    /// The palettes that change with time and, for each of `frames`, which
    /// palette and colour every pixel shows (`palette << 4 | colour`, 0 for
    /// none; empty without such palettes).
    pub cycles: Vec<PaletteCycle>,
    pub indices: Vec<Vec<u8>>,
    /// For such maps too: the same for each layer alone (the lower first),
    /// and every palette as it starts.
    pub layers: Vec<Vec<u8>>,
    pub palettes: Vec<[[u8; 3]; 16]>,
    /// One flag per 8x8 tile: true where it is solid.
    pub collision: Vec<bool>,
    pub tiles_wide: usize,
    pub tiles_high: usize,
    pub actors: Vec<Actor>,
    pub exits: Vec<Area>,
    /// The level's number in the script engine's table.
    pub level: u16,
}

/// A character standing in a place.
pub(crate) struct Actor {
    pub sprite: Option<Arc<SpriteArt>>,
    /// In pixels from the corner.
    pub x: i32,
    pub y: i32,
    /// 1 to 8, starting facing down and turning counter-clockwise.
    pub direction: u16,
    /// The script run by talking to it.
    pub script: Option<Program>,
    /// Its number among the scripts' characters.
    pub entity: u16,
    /// It stands for the leader.
    pub leader: bool,
}

/// An area running a script when walked into, or when looked at.
#[derive(Clone)]
pub(crate) struct Area {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub script: Program,
    pub examine: bool,
}

/// The characters of one of a level's scenes, layer by layer; a cutscene
/// comes with the script that plays it.
pub(crate) struct SceneArt {
    pub layers: Vec<Vec<Actor>>,
    /// Unseen spots (number, position in pixels), layer by layer.
    pub performers: Vec<Vec<(u16, i32, i32)>>,
    /// Things standing in the scene (object, position), layer by layer.
    pub things: Vec<Vec<(u16, i32, i32)>>,
    /// Areas, layer by layer.
    pub exits: Vec<Vec<Area>>,
    pub program: Option<Program>,
}

impl RomData {
    pub(crate) fn zones(&self) -> Vec<String> {
        (0..self.maps.len()).map(|index| self.zone_name(index)).collect()
    }

    pub(crate) fn load_zone(&self, index: usize) -> Option<PlaceArt> {
        let map = MapBackground::load(&self.rom, self.maps.get(index)?).ok()?;
        Some(PlaceArt {
            width: map.width,
            height: map.height,
            frames: map.frames,
            upper: map.upper,
            upper_indices: map.upper_indices,
            frame_seconds: map.frame_ticks as f32 / 60.0,
            cycles: map.cycles.into_iter().map(|cycle| PaletteCycle { palette: cycle.palette, ticks: cycle.ticks, colours: cycle.colours }).collect(),
            indices: map.indices,
            layers: map.layers,
            palettes: map.palettes,
            collision: map.collision,
            tiles_wide: map.tiles_wide,
            tiles_high: map.tiles_high,
            actors: self.zone_actors(index).unwrap_or_default(),
            exits: self.zone_exits(index).unwrap_or_default(),
            level: self.zone_level(index).map_or(0, |(level, _)| level as u16),
        })
    }

    pub(crate) fn scene(&self, level: u16, name: &str, station: bool) -> Option<SceneArt> {
        let folder = format!("SCRIPT/{}/", self.levels.get(level as usize)?.name);
        let extension = match (station, name) {
            (true, vm::ENTER) => ".sse",
            (true, _) => ".sss",
            (false, _) => ".ssa",
        };
        let path = match name.is_empty() {
            false => format!("{folder}{name}{extension}"),
            true => {
                let stations = self.rom.paths();
                let stations = stations.filter(|p| p.starts_with(&folder) && p.ends_with(".sss"));
                stations.min()?.to_string()
            }
        };
        let scene = Scene::parse(self.rom.file(&path)?).ok()?;
        let stem = path.trim_end_matches(extension);
        let actors = |layer: &Vec<rom_formats::scene::SceneActor>| layer.iter().filter_map(|actor| self.zone_actor(stem, actor, station)).collect();
        // A cutscene is played by the script named like it, a station by
        // its first script.
        let program = match station {
            true => self.program(&format!("{stem}00.ssb"), 0),
            false => self.program(&format!("{stem}.ssb"), 0),
        };
        // The areas of a station run its scripts, like its characters.
        let mut events = scene.events.iter();
        let mut areas = |count: &usize| -> Vec<Area> {
            let of_layer = events.by_ref().take(*count);
            let areas = of_layer.filter_map(|event| {
                let script = self.program(&format!("{stem}{:02}.ssb", event.script), 0)?;
                Some(Area { x: event.x, y: event.y, width: event.width, height: event.height, script, examine: false })
            });
            areas.collect()
        };
        let mut exits: Vec<Vec<Area>> = match station {
            true => scene.layer_events.iter().map(&mut areas).collect(),
            false => Vec::new(),
        };
        // Props with a script (signs, boards...) are examined.
        for (layer, objects) in scene.objects.iter().enumerate().filter(|_| station) {
            let props = objects.iter().filter_map(|object| {
                let script = self.program(&format!("{stem}{:02}.ssb", object.script?), 0)?;
                Some(Area {
                    x: object.x - object.width / 2,
                    y: object.y - object.height / 2,
                    width: object.width,
                    height: object.height,
                    script,
                    examine: true,
                })
            });
            if let Some(exits) = exits.get_mut(layer) {
                exits.extend(props);
            }
        }
        Some(SceneArt {
            exits,
            layers: scene.layers.iter().map(actors).collect(),
            performers: scene.performers,
            things: scene.objects.iter().map(|objects| objects.iter().map(|object| (object.object, object.x, object.y)).collect()).collect(),
            program,
        })
    }

    // Zones are listed in map order.
    pub(crate) fn level_zone(&self, level: u16) -> Option<usize> {
        let map = self.levels.get(level as usize)?.map_id as usize;
        (map < self.maps.len()).then_some(map)
    }

    pub(crate) fn level_art(&self, level: u16) -> Option<PlaceArt> {
        let art = self.load_zone(self.level_zone(level)?)?;
        // (Its own picture is blank: what is shown is a dungeon floor's.)
        Some(match self.floor_picture(level) {
            Some((picture, indices, cycles)) => PlaceArt {
                frames: vec![picture],
                upper: Vec::new(),
                upper_indices: Vec::new(),
                cycles,
                indices: vec![indices],
                layers: Vec::new(),
                palettes: Vec::new(),
                ..art
            },
            None => art,
        })
    }

    /// Display name of the zone drawn on map `map_index`.
    ///
    /// The place name the game shows for the level drawn on this map, with
    /// the level's internal name; levels without a place keep only that.
    pub(crate) fn zone_name(&self, map_index: usize) -> String {
        let Some(level) = self.levels.iter().find(|l| l.map_id as usize == map_index) else {
            return self.maps[map_index].bma.to_uppercase();
        };
        match self.names.places.get(level.place_id as usize) {
            Some(place) if !place.is_empty() => format!("{place} ({})", level.name),
            // (A place without a name of its own: the dungeon's whose
            // banner its scripts put up, as Luminous Spring's does.)
            _ => match self.banner_of(&level.name) {
                Some(dungeon) => format!("{} ({})", self.dungeon_name(dungeon), level.name),
                None => level.name.clone(),
            },
        }
    }

    /// The dungeon whose banner a level's scripts put up first
    /// (`back_SetDungeonBanner`), if any.
    pub(crate) fn banner_of(&self, level: &str) -> Option<u16> {
        let folder = format!("SCRIPT/{level}/");
        let banner = self.opcodes.iter().position(|opcode| opcode.name == "back_SetDungeonBanner")? as u16;
        self.rom.paths().filter(|path| path.starts_with(&folder) && path.ends_with(".ssb")).find_map(|path| {
            let script = Script::parse(self.rom.file(path)?, &self.opcodes, LANGUAGE).ok()?;
            script.operations.iter().find(|operation| operation.opcode == banner).and_then(|operation| operation.parameters.first().copied())
        })
    }

    /// The event areas of the level drawn on map `map_index`: those of its
    /// "enter" scene, with the script each one runs.
    pub(crate) fn zone_exits(&self, map_index: usize) -> Option<Vec<Area>> {
        let level = self.levels.iter().find(|l| l.map_id as usize == map_index)?;
        let folder = format!("SCRIPT/{}/", level.name);
        let scene = Scene::parse(self.rom.file(&format!("{folder}enter.sse"))?).ok()?;
        let exits = scene.events.iter().filter_map(|event| {
            let data = self.rom.file(&format!("{folder}enter{:02}.ssb", event.script))?;
            let script = Script::parse(data, &self.opcodes, LANGUAGE).ok()?;
            Some(Area {
                x: event.x,
                y: event.y,
                width: event.width,
                height: event.height,
                examine: false,
                script: Program { script: Arc::new(script), common: self.common.clone(), opcodes: self.opcodes.clone(), routine: 0 },
            })
        });
        Some(exits.collect())
    }

    /// The first level drawn on map `map_index`, with its number.
    pub(crate) fn zone_level(&self, map_index: usize) -> Option<(usize, &Level)> {
        let on_map = |(_, level): &(usize, &Level)| level.map_id as usize == map_index;
        self.levels.iter().enumerate().find(on_map)
    }

    /// Routine `routine` of the script at `path`.
    pub(crate) fn program(&self, path: &str, routine: usize) -> Option<Program> {
        let script = Script::parse(self.rom.file(path)?, &self.opcodes, LANGUAGE).ok()?;
        Some(Program { script: Arc::new(script), common: self.common.clone(), opcodes: self.opcodes.clone(), routine })
    }

    /// A character of a scene whose files are named `stem`. Entities
    /// without a species are the player's own team (the leader, its
    /// partner) or stand for them in cutscenes; the others are not shown.
    pub(crate) fn zone_actor(&self, stem: &str, actor: &rom_formats::scene::SceneActor, station: bool) -> Option<Actor> {
        let entity = self.entities.get(actor.entity as usize)?;
        let name = entity.name.as_str();
        let leader = matches!(name, "PLAYER" | "PLAYER_HERO");
        let md_index = match entity.monster {
            // (The scenes' generic team: its leader and its second. Its third
            // and fourth are not there with a team of two.)
            0 if name.contains("PARTNER") || name.starts_with("ATTENDANT") || name == "ADVENTURE_NPC1" => PARTNER_MD_INDEX,
            0 if leader || name.contains("HERO") || name == "PLAYER_APPOINT" => PLAYER_MD_INDEX,
            0 => return None,
            monster => monster as usize,
        };
        // In a station each actor has its own script file, numbered by the
        // scene: its own routine if the script has one, else the script's.
        let script = actor
            .script
            .filter(|_| station)
            .and_then(|number| self.program(&format!("{stem}{number:02}.ssb"), 0))
            .map(|program| Program { routine: program.script.actor_routine(actor.entity), ..program });
        Some(Actor { sprite: self.sprite_with(md_index, true), x: actor.x, y: actor.y, direction: actor.direction, script, entity: actor.entity, leader })
    }

    /// The characters standing in the level drawn on map `map_index`, with
    /// the script talking to them runs.
    // (The level's first "station" scene: the places walked in have theirs
    // moment by moment, by the scripts' own choice, see `town`.)
    pub(crate) fn zone_actors(&self, map_index: usize) -> Option<Vec<Actor>> {
        let level = self.levels.iter().find(|l| l.map_id as usize == map_index)?;
        let folder = format!("SCRIPT/{}/", level.name);
        let station = self.rom.paths().filter(|p| p.starts_with(&folder) && p.ends_with(".sss")).min()?;
        let scene = Scene::parse(self.rom.file(station)?).ok()?;
        let stem = station.trim_end_matches(".sss");

        let mut seen = HashSet::new();
        let mut actors = Vec::new();
        for actor in scene.layers.iter().flatten() {
            if seen.insert(actor.entity)
                && let Some(actor) = self.zone_actor(stem, actor, true)
            {
                actors.push(actor);
            }
        }
        Some(actors)
    }

    /// The picture of a level drawn with a dungeon floor's tiles: of the
    /// terrain its map says or, around a margin, that of the floor's fixed
    /// room (SkyTemple's `resolve_mapping_for_level`, `DmaDrawer`).
    pub(crate) fn floor_picture(&self, level: u16) -> Option<(Vec<u8>, Vec<u8>, Vec<PaletteCycle>)> {
        use rom_formats::tileset::{TILESET_COUNT, Terrain};
        const ROOM_MARGIN: usize = 5;
        let named = self.levels.get(level as usize)?;
        if !matches!(named.map_type, rom_formats::script::DUNGEON_TILES | rom_formats::script::FIXED_ROOM_MAP) {
            return None;
        }
        let (dungeon, floor) = rom_formats::script::level_floor(&self.rom, level)?;
        let entry = self.dungeons.dungeons.get(dungeon as usize)?;
        let floors = self.mappa.floors(entry.group as usize);
        let at = entry.first_floor as usize + (floor.max(1) as usize - 1).min((entry.floors as usize).max(1) - 1);
        let floor = floors.get(at).or(floors.last())?;
        let tileset = Some(floor.layout.tileset as usize).filter(|&tileset| tileset < TILESET_COUNT).unwrap_or(0);
        let set = Tileset::load(self.rom.file("DUNGEON/dungeon.bin")?, tileset).ok()?;
        let (chunks, wide) = MapBackground::terrain_chunks(&self.rom, self.maps.get(named.map_id as usize)?).ok()?;
        let room =
            (named.map_type == rom_formats::script::FIXED_ROOM_MAP).then(|| rom_formats::fixed::load_room(&self.rom, floor.layout.fixed_room_id)).flatten();
        let cells: Vec<Terrain> = match room {
            Some(room) => {
                let around = if room.secondary_around { Terrain::Secondary } else { Terrain::Wall };
                let mut cells = vec![around; chunks.len()];
                for (place, tile) in room.tiles.iter().enumerate() {
                    let (x, y) = (place % room.width.max(1) + ROOM_MARGIN, place / room.width.max(1) + ROOM_MARGIN);
                    if x < wide
                        && let Some(cell) = cells.get_mut(y * wide + x)
                    {
                        *cell = match tile {
                            rom_formats::fixed::FixedTile::Floor => Terrain::Floor,
                            rom_formats::fixed::FixedTile::Wall => Terrain::Wall,
                            rom_formats::fixed::FixedTile::Secondary => Terrain::Secondary,
                        };
                    }
                }
                cells
            }
            // (Each chunk's number says it: 2, 5, 8... the secondary
            // terrain; odd ones the floor.)
            None => chunks
                .iter()
                .map(|&chunk| match chunk as usize + 1 {
                    one if one % 3 == 0 => Terrain::Secondary,
                    one if one % 2 == 0 => Terrain::Floor,
                    _ => Terrain::Wall,
                })
                .collect(),
        };
        let (picture, indices) = set.draw_terrain(&cells, wide);
        let cycles = set.cycles().iter().map(|(palette, ticks, colours)| PaletteCycle { palette: *palette, ticks: *ticks, colours: colours.clone() });
        Some((picture, indices, cycles.collect()))
    }
}
