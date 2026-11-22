"""A dungeon of the Workshop (and the boss fight it ends with) put into the
user's own Explorers of Sky ROM, in place of Beach Cave: used by
`patch_rom.py --dungeon`.

    scene_player export-ds games/<game>/dungeons/<name>.dungeon --out target/ds/<name>.json
    python tools/ds/patch_rom.py <rom> <film.exps> --boot <out.nds> --dungeon target/ds/<name>.json
    python tools/ds/patch_rom.py <rom> - --boot <out.nds> --dungeon target/ds/<name>.json [--boss]

`scene_player export-ds <file.dungeon>` dumps the dungeon as the rules read
it (JSON); this module writes it as Sky's data (see tools/ds/DUNGEONS.md):

- Beach Cave (dungeon 1) gets the dungeon's floors: floor list 1 of
  `BALANCE/mappa_s.bin` (and `mappa_gs.bin`, made from it), each floor with
  its look's tileset and layout figures, its song, its Pokémon (species,
  level, weights) and its items; the dungeon list (arm9) gets the count.
- Beach Cave Pit (dungeon 2, the floor after Beach Cave's in the same list)
  becomes the boss fight: fixed room 1 of `BALANCE/fixed.bin` laid out as the
  arena, the boss and its guards placed (fixed-room entities of overlay 29,
  their level and stats, the boss's life multiplied, in overlay 10), the
  fixed room's song.
- Both dungeons are named after it, in every language.
- `boss_says` / `boss_falls` become two acting scenes, played as Sky plays its
  boss scenes: between the two dungeons, then after the fight, from
  `GETOUT_SCENARIO_DUNGEON` (unionall), which the game runs as a dungeon ends.

What Sky cannot hold this way is told as it goes (and in tools/ds/LIMITS.md).
Nothing written here may be committed: it is made of the user's ROM.
"""
import copy
import json

from skytemple_files.common.types.file_types import FileType
from skytemple_files.common.util import get_binary_from_rom, set_binary_in_rom
from skytemple_files.dungeon_data.fixed_bin.model import EntityRule, FixedFloor, TileRule, TileRuleType
from skytemple_files.dungeon_data.mappa_g_bin.mappa_converter import convert_mappa_to_mappag
from skytemple_files.hardcoded.dungeon_music import HardcodedDungeonMusic
from skytemple_files.hardcoded.default_starters import HardcodedDefaultStarters
from skytemple_files.hardcoded.dungeons import HardcodedDungeons
from skytemple_files.hardcoded.fixed_floor import HardcodedFixedFloorTables, MonsterSpawnType

# The slots taken: Beach Cave, its pit (the floor after it, a fixed room),
# their floor list, the pit's fixed room. Safe early in a new game: the
# story's own way there is never played (the film starts the game).
FLOORS_OF, BOSS_OF, LIST, ROOM = 1, 2, 1, 1
KECLEON = 383  # md index of the shop's Kecleon: kept on every floor (weight 0)
MAPPA, MAPPA_G, FIXED = 'BALANCE/mappa_s.bin', 'BALANCE/mappa_gs.bin', 'BALANCE/fixed.bin'

# The arena's tiles (0 ground, 1 wall, 2 water) as fixed-room tiles.
ARENA_TILES = {0: TileRuleType.FLOOR_ROOM, 1: TileRuleType.WALL_HALLWAY_IMPASSABLE, 2: TileRuleType.SECONDARY_ROOM}

# The rules' figures of a floor (`props`) and the mappa layout's fields.
LAYOUT_FIELDS = {
    'layout': 'structure', 'room_density': 'room_density', 'floor_connectivity': 'floor_connectivity',
    'enemy_density': 'initial_enemy_density', 'kecleon_shop_chance': 'kecleon_shop_chance',
    'monster_house_chance': 'monster_house_chance', 'item_density': 'item_density', 'trap_density': 'trap_density',
    'extra_hallways': 'extra_hallway_density', 'buried_item_density': 'buried_item_density',
    'secondary_density': 'water_density', 'dead_ends': 'dead_ends',
    'empty_monster_house_chance': 'empty_monster_house_chance',
}


def load(path):
    with open(path, encoding='utf-8') as file:
        return json.load(file)


def weights_of(thresholds):
    """Thresholds out of 10,000, each over the one before: the weights."""
    weights, before = [], 0
    for threshold in thresholds:
        weights.append(max(threshold - before, 0))
        before = max(before, threshold)
    return weights


def cumulated(weights):
    """Weights as Sky keeps them: thresholds out of 10,000 (the last 10,000)."""
    total = sum(weights) or 1
    out, running = [], 0
    for weight in weights:
        running += weight
        out.append(round(running * 10000 / total) if weight else 0)
    for at in range(len(out) - 1, -1, -1):
        if out[at]:
            out[at] = 10000
            break
    return out


def item_list(kind, table):
    """The rules' item table (in their order) as Sky's: categories by number,
    each one's items by number, thresholds within each (SkyTemple's way)."""
    cats = table['categories']
    by_cat = dict(zip([c for c, _ in cats], weights_of([t for _, t in cats])))
    categories = dict(zip(sorted(by_cat), cumulated([by_cat[c] for c in sorted(by_cat)])))
    items = {}
    for cat in by_cat:
        of = [(item, threshold) for item, category, threshold in table['items'] if category == cat]
        weights = dict(zip([i for i, _ in of], weights_of([t for _, t in of])))
        items.update(zip(sorted(weights), cumulated([weights[i] for i in sorted(weights)])))
    return kind(categories, dict(sorted(items.items())))


def music_index(songs, track, told):
    """Sky's floors name a song by its place in the dungeon music list."""
    for at, entry in enumerate(songs):
        if not entry.is_random_ref and entry.track_or_ref == track:
            return at
    told.append(f'song {track}: not in the dungeon music list, Beach Cave\'s kept')
    return None


def floor_made(base, floor, number, songs, told):
    """A random floor of Beach Cave's list made as the rules say."""
    made = copy.deepcopy(base)
    layout = made.layout
    layout.tileset_id = floor['tileset']
    layout.floor_number = number
    layout.fixed_floor_id = 0
    for said, field in LAYOUT_FIELDS.items():
        if said in floor['props']:
            setattr(layout, field, type(getattr(layout, field))(floor['props'][said]))
    layout.weather = floor['weather']
    layout.enemy_iq = floor['enemy_iq']
    layout.max_coin_amount = floor['max_money']
    layout.sticky_item_chance = floor['sticky_chance']
    if floor.get('music') is not None:
        index = music_index(songs, floor['music'], told)
        if index is not None:
            layout.music_id = index
    monster = type(base.monsters[0])
    enemies = floor['enemies']
    weights = dict(zip([(e['species'], e['stats']['level']) for e in enemies], weights_of([e['weight'] for e in enemies])))
    spawns = {}
    for (species, level), weight in weights.items():
        if species in spawns:
            told.append(f'floor {number}: species {species} at two levels, one kept (Sky has one entry a species)')
            continue
        spawns[species] = (level, weight)
    order = sorted(spawns)
    thresholds = dict(zip(order, cumulated([spawns[s][1] for s in order])))
    made.monsters = [monster(spawns[s][0], thresholds[s], thresholds[s], s) for s in order]
    # (Kecleon, for the shops, and the list's end as the game has them.)
    made.monsters += [copy.deepcopy(m) for m in base.monsters if m.main_spawn_weight == 0 and m.md_index not in spawns]
    made.monsters.sort(key=lambda m: m.md_index)
    if len(floor['traps']) == 25:
        made.traps = type(base.traps)(list(floor['traps']))
    made.floor_items = item_list(type(base.floor_items), floor['items'])
    if floor['shop_items']['items']:
        made.shop_items = item_list(type(base.shop_items), floor['shop_items'])
    return made


def free_entries(count, live, what, wanted):
    """Table entries no other fixed room reaches (0 left alone: the game's "none")."""
    free = [i for i in range(1, count) if i not in live]
    if len(free) < wanted:
        raise SystemExit(f'the boss fight needs {wanted} {what}; only {len(free)} are free')
    return free[:wanted]



def put_arena(rom, config, fixed_floor, song, told):
    """Fixed room 1 as the arena: its tiles, the team's place, the boss and
    guards as fixed-room entities of their own, their stats; its song."""
    ov29 = get_binary_from_rom(rom, config.bin_sections.overlay29)
    ov10 = get_binary_from_rom(rom, config.bin_sections.overlay10)
    entities = HardcodedFixedFloorTables.get_entity_spawn_table(ov29, config)
    monsters = HardcodedFixedFloorTables.get_monster_spawn_list(ov29, config)
    stats = HardcodedFixedFloorTables.get_monster_spawn_stats_table(ov10, config)
    props = HardcodedFixedFloorTables.get_fixed_floor_properties(ov10, config)
    fixed = FileType.FIXED_BIN.deserialize(rom.getFileByName(FIXED))
    # What the other fixed rooms reach: their entities, those entities'
    # monster entries, those entries' stats. The rest is free to take.
    live_entities = {action.entity_rule_id for room, floor in enumerate(fixed.fixed_floors) if room != ROOM
                     for action in floor.actions if isinstance(action, EntityRule)}
    live_monsters = {entities[i].monster_id for i in live_entities if i < len(entities)}
    live_stats = {monsters[i].stats_entry for i in live_monsters if i < len(monsters)}
    # (Those placed with the same species, level and life share one entity.)
    kinds = []
    for placed in fixed_floor['monsters']:
        enemy = placed['enemy']
        key = (enemy['species'], enemy['stats']['level'], enemy['stats']['max_hp'])
        if key not in [k for k, _ in kinds]:
            kinds.append((key, enemy))
    chosen = free_entries(len(entities), live_entities, 'fixed-room entities', len(kinds))
    spawn_ids = free_entries(len(monsters), live_monsters, 'fixed-room monster entries', len(kinds))
    stats_ids = free_entries(len(stats), live_stats, 'fixed-room stats entries', len(kinds))
    entity_of = {}
    for (key, enemy), entity_id, spawn_id, stats_id in zip(kinds, chosen, spawn_ids, stats_ids):
        s = enemy['stats']
        entities[entity_id].monster_id = spawn_id
        entities[entity_id].item_id = 0
        entities[entity_id].tile_id = 0
        monsters[spawn_id].md_idx = enemy['species']
        monsters[spawn_id].stats_entry = stats_id
        monsters[spawn_id].enemy_settings = MonsterSpawnType.ENEMY_STRONG
        row = stats[stats_id]
        row.level, row.hp, row.exp_yield = s['level'], s['max_hp'], min(enemy.get('exp', 0), 0xFFFF)
        row.attack, row.special_attack = min(s['attack'], 255), min(s['sp_attack'], 255)
        row.defense, row.special_defense = min(s['defense'], 255), min(s['sp_defense'], 255)
        entity_of[key] = entity_id
        if s.get('held'):
            told.append(f'species {enemy["species"]}: the item it holds (boss_item) is lost: fixed-room Pokémon hold nothing')
    # The room, a wall around it: (x, y) of the arena are (x + 1, y + 1).
    wide, high = fixed_floor['width'] + 2, fixed_floor['height'] + 2
    wall = TileRule(TileRuleType.WALL_HALLWAY_IMPASSABLE)
    actions = [wall] * (wide * high)
    for at, tile in enumerate(fixed_floor['tiles']):
        x, y = at % fixed_floor['width'], at // fixed_floor['width']
        actions[(y + 1) * wide + x + 1] = TileRule(ARENA_TILES.get(tile, TileRuleType.FLOOR_ROOM))
    leader = fixed_floor.get('leader') or [fixed_floor['width'] // 2, fixed_floor['height'] - 2]
    actions[(leader[1] + 1) * wide + leader[0] + 1] = TileRule(TileRuleType.LEADER_SPAWN)
    for placed in fixed_floor['monsters']:
        enemy = placed['enemy']
        key = (enemy['species'], enemy['stats']['level'], enemy['stats']['max_hp'])
        x, y = placed['at']
        actions[(y + 1) * wide + x + 1] = EntityRule(entity_of[key])
    room = FixedFloor.new(wide, high, actions)
    room.unk4 = fixed.fixed_floors[ROOM].unk4
    fixed.fixed_floors[ROOM] = room
    for what in ('allies', 'watchers', 'items', 'traps'):
        if fixed_floor.get(what):
            told.append(f'the arena\'s {what}: not placed (only the boss and its guards are)')
    prop = props[ROOM]
    prop.exit_floor_when_defeating_enemies = fixed_floor.get('ends_with_enemies', True)
    prop.moves_enabled = fixed_floor.get('moves_allowed', True)
    prop.orbs_enabled = fixed_floor.get('orbs_allowed', False)
    if song is not None:
        prop.music_track = song
    rom.setFileByName(FIXED, FileType.FIXED_BIN.serialize(fixed))
    HardcodedFixedFloorTables.set_entity_spawn_table(ov29, entities, config)
    HardcodedFixedFloorTables.set_monster_spawn_list(ov29, monsters, config)
    set_binary_in_rom(rom, config.bin_sections.overlay29, ov29)
    HardcodedFixedFloorTables.set_monster_spawn_stats_table(ov10, stats, config)
    HardcodedFixedFloorTables.set_fixed_floor_properties(ov10, props, config)
    set_binary_in_rom(rom, config.bin_sections.overlay10, ov10)


def put_names(rom, config, names):
    """Dungeons named, in each language's text: {dungeon id: name}."""
    blocks = [block for name, block in config.string_index_data.string_blocks.items() if name.startswith('Dungeon Names')]
    for language in config.string_index_data.languages:
        path = f'MESSAGE/{language.filename}'
        if rom.filenames.idOf(path) is None:
            continue
        text = FileType.STR.deserialize(rom.getFileByName(path))
        for block in blocks:
            for dungeon, name in names.items():
                text.strings[block.begin + dungeon] = name
        rom.setFileByName(path, FileType.STR.serialize(text))


def put(rom, config, dump):
    """The dungeon (and its boss fight) in place of Beach Cave and its pit.
    Returns what is lost, said in words."""
    told = []
    table = dump['dungeon']
    songs = HardcodedDungeonMusic.get_music_list(get_binary_from_rom(rom, config.bin_sections.overlay10), config)
    mappa = FileType.MAPPA_BIN.deserialize(rom.getFileByName(MAPPA))
    old = mappa.floor_lists[LIST]
    base, pit = old[0], old[-1]
    floors = [floor for floor in table['floors'] if not floor['fixed']]
    arena = next((floor for floor in table['floors'] if floor['fixed']), None)
    if not floors:
        # (A fight alone: one floor of the look before it, empty, Sky needs a dungeon 1.)
        raise SystemExit('a boss fight alone (0 floors): give the dungeon one floor at least')
    made = [floor_made(base, floor, number, songs, told) for number, floor in enumerate(floors, 1)]
    boss_floor = copy.deepcopy(pit)
    boss_floor.layout.floor_number = len(made) + 1
    if arena:
        boss_floor.layout.tileset_id = arena['tileset']
        boss_floor.layout.fixed_floor_id = ROOM
        song = music_index(songs, arena['music'], told) if arena.get('music') is not None else None
        put_arena(rom, config, arena['fixed'], song, told)
    while len(mappa.floor_lists[LIST]):
        mappa.remove_floor_from_floor_list(LIST, 0)
    for floor in made + [boss_floor]:
        mappa.add_floor_to_floor_list(LIST, floor)
    rom.setFileByName(MAPPA, FileType.MAPPA_BIN.serialize(mappa))
    rom.setFileByName(MAPPA_G, FileType.MAPPA_G_BIN.serialize(convert_mappa_to_mappag(mappa)))
    arm9 = bytearray(get_binary_from_rom(rom, config.bin_sections.arm9))
    dungeons = HardcodedDungeons.get_dungeon_list(arm9, config)
    first, second = dungeons[FLOORS_OF], dungeons[BOSS_OF]
    first.number_floors, first.mappa_index, first.start_after = len(made), LIST, 0
    second.number_floors, second.mappa_index, second.start_after = 1, LIST, len(made)
    first.number_floors_in_group = second.number_floors_in_group = len(made) + 1
    HardcodedDungeons.set_dungeon_list(dungeons, arm9, config)
    restrictions = HardcodedDungeons.get_dungeon_restrictions(arm9, config)
    rules = table['rules']
    for dungeon in (FLOORS_OF, BOSS_OF):
        held = restrictions[dungeon]
        held.recruiting_allowed = rules['recruitment']
        held.enemies_grant_exp = rules['enemies_give_exp']
        held.level_reset = rules['level_one']
        held.money_allowed = rules['money_allowed']
        held.iq_skills_disabled = rules['iq_disabled']
        held.max_items_allowed = min(rules['max_items'], 48)
        held.max_party_members = min(max(rules['max_party'], 1), 4)
        held.turn_limit = rules['turn_limit']
    HardcodedDungeons.set_dungeon_restrictions(restrictions, arm9, config)
    # The team: the game's default hero and partner (what a new game has
    # without the personality quiz), at the dungeon's level if it says one.
    team = dump.get('team') or []
    if len(team) >= 1:
        HardcodedDefaultStarters.set_player_md_id(team[0], arm9, config)
    if len(team) >= 2:
        HardcodedDefaultStarters.set_partner_md_id(team[1], arm9, config)
    if dump.get('team_level'):
        level = min(max(dump['team_level'], 1), 100)
        HardcodedDefaultStarters.set_player_level(level, arm9, config)
        HardcodedDefaultStarters.set_partner_level(level, arm9, config)
    set_binary_in_rom(rom, config.bin_sections.arm9, arm9)
    boss = dump.get('boss')
    put_names(rom, config, {FLOORS_OF: dump['name'], BOSS_OF: (boss or {}).get('title') or dump['name']})
    if rules.get('goes_up'):
        told.append('floors counted up: Beach Cave counts down (a restriction flag not set)')
    if boss:
        if boss.get('moves'):
            told.append('boss_moves: fixed-room Pokémon know their species\' level-up moves')
        if boss.get('phases'):
            told.append(f'{len(boss["phases"])} boss_phase lines: Sky has no phases without code (see LIMITS.md)')
        if boss.get('joins'):
            told.append('the boss joining the team after the fight: not done')
    print(f'  dungeon: {dump["name"]}, {len(made)} floors in Beach Cave' + (', the boss fight in Beach Cave Pit' if arena else ''))
    return told


def boss_scene(boss, guards, said, level, centre, falls):
    """A scene of the boss fight, as a film's export: the team before the
    boss and its guards, what the boss says. (`ACTOR_DWBOSS..`: free actors
    patch_rom.py casts.)"""
    x, y = centre
    lines = [f'// The boss {"falls" if falls else "speaks"}: a scene made by tools/ds/dungeon.py.',
             f'// ds-export level {level}',
             f'// ds-export actor ACTOR_PLAYER x {x - 1} y {y + 3} facing DIR_UP',
             f'// ds-export actor ACTOR_ATTENDANT1 x {x + 1} y {y + 3} facing DIR_UP',
             f'// ds-export actor ACTOR_DWBOSS0 species {boss["species"]} x {x} y {y - 2} facing DIR_DOWN']
    for n, (species, dx) in enumerate(zip(guards, (-3, 3, -5, 5, -2, 2))):
        lines.append(f'// ds-export actor ACTOR_DWBOSS{n + 1} species {species} x {x + dx} y {y - 1} facing DIR_DOWN')
    lines.append(f'// ds-export performer 0 type 0 x {x} y {y}')
    face = 'FACE_PAIN' if falls else 'FACE_DETERMINED'
    text = ' ' + said.replace('"', "'")
    if boss.get('title') and not falls:
        text = f' I am [CS:N]{boss["name"]}[CR], {boss["title"][0].lower()}{boss["title"][1:]}.[K]' + text
    lines += [
        'def 0 {',
        f'    back_SetGround({level});',
        '    supervision_Acting(0);',
        '    camera_SetMyself<performer 0>();',
        '    screen_FadeIn(1, 30);',
        '    WaitScreenFade();',
        '    Wait(30);',
        f'    SetEffect<actor ACTOR_DWBOSS0>({"EFFECT_SWEAT_DROPS_FROM_BOTH_SIDES_MEDIUM" if falls else "EFFECT_ANGRY"}, 3);',
        '    Wait(30);',
        f'    message_SetFace(ACTOR_DWBOSS0, {face}, FACE_POS_STANDARD);',
        f'    message_Talk("{text}");',
        '    message_Close();',
        '    SetEffect<actor ACTOR_DWBOSS0>(EFFECT_NONE, 3);',
        '    Wait(30);',
        '    screen_FadeOut(1, 30);',
        '    WaitScreenFade();',
        '    end;',
        '}',
    ]
    return '\n'.join(lines) + '\n'


def boss_scenes(dump, level, centre):
    """The scenes before and after the fight (texts as `export-ds` writes
    them), or none if the dungeon has no boss."""
    boss = dump.get('boss')
    arena = next((floor['fixed'] for floor in dump['dungeon']['floors'] if floor['fixed']), None)
    if not boss or not arena:
        return []
    guards = [placed['enemy']['species'] for placed in arena['monsters'][1:]][:6]
    return [boss_scene(boss, guards, boss.get('begins') or '...', level, centre, False),
            boss_scene(boss, guards, boss.get('falls') or '...', level, centre, True)]


def team_lines():
    """The team made as after the personality quiz; with no quiz, of the
    game's default hero and partner, which `put` sets to the game's team."""
    return ['    switch ( ProcessSpecial(PROCESS_SPECIAL_INIT_MAIN_TEAM_AFTER_QUIZ, 0, 0) ) { }\n']


def enter_lines(dungeon):
    """Into that dungeon, as the story enters Beach Cave (m01a0301)."""
    return [
        '    screen_FadeOut(1, 30);\n',
        f'    main_EnterDungeon({dungeon}, 0);\n',
        '    switch ( message_Menu(MENU_DUNGEON_INITIALIZE_TEAM) ) { }\n',
        '    main_EnterDungeon(-1, 0);\n',
        '    hold;\n',
    ]


def getout_edit(source, before, after):
    """`GETOUT_SCENARIO_DUNGEON`, as a dungeon is cleared: Beach Cave's end
    plays the scene before the fight and enters the pit; the pit's plays the
    scene after it and goes back to the title. Only while the story has not
    begun (scenario under 2, 0: a new game before the quiz, where the film
    plays), Sky's own story untouched."""
    import re
    ours = "if ( scn($SCENARIO_MAIN) < [2, 0] ) {\n"

    def block(indent, plays, then):
        body = ''.join(f'{indent}    {line}\n' for line in plays + then)
        return f'{indent}{ours}{body}{indent}}}\n'

    first = re.compile(r'( *)case 1:\n( *)if \( scn\(\$SCENARIO_MAIN\) == \[2, 2\] \) \{\n( *)JumpCommon\(CORO_EVENT_M01_04\);')
    second = re.compile(r'( *)case 2:\n( *)dungeon_mode\(1\) = DMODE_OPEN_AND_REQUEST;\n( *)dungeon_mode\(2\) = DMODE_REQUEST;\n( *)JumpCommon\(CORO_EVENT_M01_07_08\);')
    for pattern, plays, then in (
        (first, before, ['main_EnterDungeon(2, 30);', 'main_EnterDungeon(-1, 30);', 'hold;']),
        (second, after, ['switch ( ProcessSpecial(PROCESS_SPECIAL_RETURN_TO_TITLE_SCREEN, 60, 0) ) { }', 'hold;']),
    ):
        found = pattern.search(source)
        if not found:
            raise SystemExit('unionall: GETOUT_SCENARIO_DUNGEON is not as expected (Beach Cave\'s end)')
        indent = found.group(2)
        head = found.group(0).split('\n')[0] + '\n'
        source = source[:found.start()] + head + block(indent, plays, then) + source[found.start() + len(head):]
    return source
