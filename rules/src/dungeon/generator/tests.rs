use super::*;

fn rich_properties() -> FloorProperties {
    FloorProperties {
        extra_hallways: 6,
        flags: 0x5,
        secondary_density: 3,
        kecleon_shop_chance: 50,
        monster_house_chance: 50,
        secondary_structures: 3,
        ..FloorProperties::default()
    }
}

fn generator_for(seed: u32, props: &FloorProperties) -> (u32, Floor) {
    let mut rng = DungeonRng::new(seed);
    let mut generator = new_generator(&mut rng, props);
    generator.generate();
    (generator.fingerprint(), generator.into_floor())
}

/// Returns how many floors fell back to the one-room layout.
fn check_invariants(props: &FloorProperties, seeds: std::ops::Range<u32>) -> usize {
    let mut fallbacks = 0;
    for seed in seeds {
        let (_, floor) = generator_for(seed, props);
        fallbacks += floor.fallback as usize;
        assert!(matches!(floor.tile(floor.stairs), Tile::Stairs { .. }), "seed {seed}");
        assert!(floor.tile(floor.player_spawn).is_walkable(), "seed {seed}");
        // Every walkable tile reaches the stairs. Like the original's
        // own check, this lets secondary terrain be crossed: a floor may
        // be cut in two by water.
        let index = |p: Pos| (p.y * FLOOR_WIDTH + p.x) as usize;
        let mut seen = vec![false; (FLOOR_WIDTH * FLOOR_HEIGHT) as usize];
        let mut stack = vec![floor.stairs];
        seen[index(floor.stairs)] = true;
        while let Some(p) = stack.pop() {
            for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
                let q = Pos::new(p.x + dx, p.y + dy);
                if floor.tile(q) != Tile::Wall && !seen[index(q)] {
                    seen[index(q)] = true;
                    stack.push(q);
                }
            }
        }
        for p in Floor::positions() {
            assert!(!floor.tile(p).is_walkable() || seen[index(p)], "seed {seed}: {p:?}");
            // The outer border and the rows next to it are never open.
            if p.x == 0 || p.y <= 1 || p.x == FLOOR_WIDTH - 1 || p.y >= FLOOR_HEIGHT - 2 {
                assert!(!floor.tile(p).is_walkable(), "seed {seed}");
            }
        }
        // Spawns sit on distinct open tiles, away from the player.
        for spawn in floor.enemy_spawns.iter().chain(&floor.item_spawns) {
            assert!(floor.tile(*spawn).is_walkable(), "seed {seed}");
            assert_ne!(*spawn, floor.player_spawn, "seed {seed}");
        }
        assert!(!floor.enemy_spawns.is_empty(), "seed {seed}");
    }
    fallbacks
}

#[test]
fn plain_floors_are_valid_and_never_fall_back() {
    assert_eq!(check_invariants(&FloorProperties::default(), 0..1500), 0);
}

#[test]
fn featured_floors_are_valid() {
    let fallbacks = check_invariants(&rich_properties(), 0..1500);
    assert!(fallbacks <= 15, "{fallbacks} fallbacks");
    let secondary = (0..50).any(|seed| {
        let (_, floor) = generator_for(seed, &rich_properties());
        Floor::positions().any(|p| floor.tile(p) == Tile::Secondary)
    });
    assert!(secondary);
}

#[test]
fn every_layout_is_valid() {
    for layout in 1..=11 {
        let props = FloorProperties { layout, ..rich_properties() };
        check_invariants(&props, 0..200);
    }
}

#[test]
fn one_room_floors_may_be_disarmed() {
    let house = FloorProperties { layout: 2, ..rich_properties() };
    let disarmed = FloorProperties { disarmed_lone_house: true, ..house };
    for seed in 0..10 {
        let (_, armed) = generator_for(seed, &house);
        let (_, calm) = generator_for(seed, &disarmed);
        assert!(armed.monster_house.is_some() && calm.monster_house.is_none(), "{seed}");
        assert!(calm.enemy_spawns.len() < armed.enemy_spawns.len());
    }
    // (A floor of several rooms keeps its chance of one.)
    let (_, many) = generator_for(3, &FloorProperties { disarmed_lone_house: true, monster_house_chance: 100, ..rich_properties() });
    let (_, same) = generator_for(3, &FloorProperties { monster_house_chance: 100, ..rich_properties() });
    assert_eq!(many.monster_house, same.monster_house);
}

#[test]
fn buried_items_are_in_walls() {
    let props = FloorProperties { buried_item_density: 6, ..rich_properties() };
    let mut buried = 0;
    for seed in 0..50 {
        let (_, floor) = generator_for(seed, &props);
        buried += floor.buried_item_spawns.len();
        assert!(floor.buried_item_spawns.iter().all(|&p| floor.tile(p) == Tile::Wall));
    }
    assert!(buried > 100, "{buried}");
    let (_, plain) = generator_for(1, &rich_properties());
    assert!(plain.buried_item_spawns.is_empty());
}

#[test]
fn special_rooms_show_up() {
    let (mut shops, mut houses) = (0, 0);
    for seed in 0..400 {
        let (_, floor) = generator_for(seed, &rich_properties());
        shops += floor.shop.is_some() as usize;
        houses += floor.monster_house.is_some() as usize;
    }
    assert!(shops > 60 && houses > 30, "{shops} shops, {houses} monster houses");
}

/// Fingerprints of floors generated by the Python reference
/// (`reference/dungeon-eos`, hidden stairs disabled) for the same seeds
/// and properties: the port must give the same tiles.
#[test]
fn floors_match_the_reference_implementation() {
    for &(seed, layout, expected) in REFERENCE_FINGERPRINTS {
        let props = FloorProperties { layout, ..rich_properties() };
        let (fingerprint, _) = generator_for(seed, &props);
        assert_eq!(fingerprint, expected, "seed {seed}, layout {layout}");
    }
}

/// Prints one floor like the reference harness does, to diff them:
/// .
#[test]
fn dump_floor() {
    let (Ok(seed), Ok(layout)) = (std::env::var("GEN_SEED"), std::env::var("GEN_LAYOUT")) else {
        return;
    };
    let props = FloorProperties { layout: layout.parse().unwrap(), ..rich_properties() };
    let mut rng = DungeonRng::new(seed.parse().unwrap());
    let mut generator = new_generator(&mut rng, &props);
    generator.generate();
    println!("{:08X}", generator.fingerprint());
    for y in 0..H {
        let line: String = (0..W)
            .map(|x| {
                let tile = generator.at(x, y);
                match tile.spawn & 0xF {
                    0 => [' ', '.', '~', '?'][tile.kind() as usize],
                    flags => char::from_digit(flags as u32, 16).unwrap().to_ascii_uppercase(),
                }
            })
            .collect();
        println!("{line}");
    }
    drop(generator);
    println!("state {:08X}", rng.state());
}

/// `(seed, layout, fingerprint)`, from the reference.
const REFERENCE_FINGERPRINTS: &[(u32, u8, u32)] = &[
    (7919, 0, 0x8B0BB089),
    (15838, 0, 0x462A9354),
    (23757, 0, 0x69641AB6),
    (31676, 0, 0x7507CA65),
    (39595, 0, 0xC49A32C1),
    (47514, 0, 0xFAA615E9),
    (55433, 0, 0xEB8C785C),
    (63352, 0, 0xD4B95B2A),
    (71271, 0, 0x99A7149C),
    (79190, 0, 0x3C76EA21),
    (87109, 0, 0x60AD3DD8),
    (95028, 0, 0xF45354A4),
    (102947, 0, 0xAC6D229F),
    (110866, 0, 0x88353F4D),
    (118785, 0, 0x1EE49FA8),
    (126704, 0, 0x97FC4740),
    (134623, 0, 0x1D327ED9),
    (142542, 0, 0x49FDB92A),
    (150461, 0, 0x282FE70D),
    (158380, 0, 0x17B9E685),
    (166299, 0, 0x2ACF8226),
    (174218, 0, 0x2686AC15),
    (182137, 0, 0x7DEDD024),
    (190056, 0, 0x4F74BB9F),
    (197975, 0, 0xF5C91250),
    (7919, 1, 0x8C492D24),
    (15838, 1, 0xE9D77F53),
    (23757, 1, 0xA25F212C),
    (31676, 1, 0x8506E600),
    (39595, 1, 0x6B27B477),
    (47514, 1, 0xF73EB704),
    (55433, 1, 0xFB612B3A),
    (63352, 1, 0xF22858AF),
    (71271, 1, 0xFD8D419C),
    (79190, 1, 0x18940D10),
    (87109, 1, 0x47446197),
    (95028, 1, 0x94EBB9D1),
    (102947, 1, 0x7B5809EE),
    (110866, 1, 0x96793AA1),
    (118785, 1, 0xB99D8AA0),
    (126704, 1, 0x20B22DD3),
    (134623, 1, 0x505CC2A2),
    (142542, 1, 0xA1A1E064),
    (150461, 1, 0xCF51C916),
    (158380, 1, 0x0E16CD88),
    (166299, 1, 0x947B5E38),
    (174218, 1, 0xBE5E9CD2),
    (182137, 1, 0x7B5FBA9C),
    (190056, 1, 0x7A50978E),
    (197975, 1, 0x1D5F39D1),
    (7919, 2, 0xC2F3EE17),
    (15838, 2, 0xEBC5DF69),
    (23757, 2, 0x08AC998E),
    (31676, 2, 0xC2359BD3),
    (39595, 2, 0xEF307BCD),
    (47514, 2, 0xDD19398F),
    (55433, 2, 0xB16FFFBF),
    (63352, 2, 0xE8B12F4F),
    (71271, 2, 0xCD2138D0),
    (79190, 2, 0x4D5AE2E3),
    (87109, 2, 0xE5754733),
    (95028, 2, 0xEBAC03A3),
    (102947, 2, 0x57FB149C),
    (110866, 2, 0x367A103D),
    (118785, 2, 0x2BCA6238),
    (126704, 2, 0x6558AEBB),
    (134623, 2, 0x9EA17565),
    (142542, 2, 0x1FF9A9B6),
    (150461, 2, 0x297F757D),
    (158380, 2, 0x1ACA3487),
    (166299, 2, 0x3AF7449E),
    (174218, 2, 0xF5435BA0),
    (182137, 2, 0x3CCFC5DC),
    (190056, 2, 0xAD8CEBA3),
    (197975, 2, 0x9EFCC15C),
    (7919, 3, 0xC946E14B),
    (15838, 3, 0xCDBC02FB),
    (23757, 3, 0x535D4CD1),
    (31676, 3, 0x3F350D6E),
    (39595, 3, 0x6DB335E1),
    (47514, 3, 0xC204D289),
    (55433, 3, 0x79BFA044),
    (63352, 3, 0x091C942B),
    (71271, 3, 0x4A744B17),
    (79190, 3, 0x5CD8B143),
    (87109, 3, 0x630A8F83),
    (95028, 3, 0x15007CB5),
    (102947, 3, 0xDBD692ED),
    (110866, 3, 0xDEE8CF41),
    (118785, 3, 0xAB6E768A),
    (126704, 3, 0x72C74CB6),
    (134623, 3, 0x9B97AA60),
    (142542, 3, 0x668A6EFA),
    (150461, 3, 0x97DED098),
    (158380, 3, 0xEB83B9E1),
    (166299, 3, 0x59FDF4A6),
    (174218, 3, 0x088577E3),
    (182137, 3, 0x95A3630B),
    (190056, 3, 0xD1D8BD1A),
    (197975, 3, 0x959A13A3),
    (7919, 4, 0xD63F191E),
    (15838, 4, 0x72C432E2),
    (23757, 4, 0xD4261990),
    (31676, 4, 0xC5D2212B),
    (39595, 4, 0xAF0DC4AD),
    (47514, 4, 0x8651C364),
    (55433, 4, 0x891F0140),
    (63352, 4, 0xAE25D86B),
    (71271, 4, 0x83F90B2A),
    (79190, 4, 0x99FB2AED),
    (87109, 4, 0x34CD6B79),
    (95028, 4, 0x83C57826),
    (102947, 4, 0x685F43AA),
    (110866, 4, 0xD1F64608),
    (118785, 4, 0x33A4BF90),
    (126704, 4, 0xB7C6FDB4),
    (134623, 4, 0xF282D299),
    (142542, 4, 0x9419EDD4),
    (150461, 4, 0xBAD69ECC),
    (158380, 4, 0x5E0B2D40),
    (166299, 4, 0x67E5F815),
    (174218, 4, 0xFC691653),
    (182137, 4, 0x3659BD21),
    (190056, 4, 0xD9B7A9A3),
    (197975, 4, 0xC16F95C6),
    (7919, 5, 0x6AD89BE9),
    (15838, 5, 0xD2E0D8B5),
    (23757, 5, 0xCF2615EF),
    (31676, 5, 0xD8161BA7),
    (39595, 5, 0x6A985E51),
    (47514, 5, 0xBD60F041),
    (55433, 5, 0x50D52FBD),
    (63352, 5, 0x77D46414),
    (71271, 5, 0xEC2DC6F3),
    (79190, 5, 0x23238BF0),
    (87109, 5, 0x4BC19326),
    (95028, 5, 0xB8158213),
    (102947, 5, 0x48C17F85),
    (110866, 5, 0xC5D3AD0B),
    (118785, 5, 0x582D6DEF),
    (126704, 5, 0xA9BF9A62),
    (134623, 5, 0xBAEE0832),
    (142542, 5, 0x30E8777A),
    (150461, 5, 0xEEF80F18),
    (158380, 5, 0x8B4C200F),
    (166299, 5, 0xE832CC84),
    (174218, 5, 0x9DD87A34),
    (182137, 5, 0x537FCE28),
    (190056, 5, 0x9067B390),
    (197975, 5, 0xAFEB02DF),
    (7919, 6, 0x1B33B3B2),
    (15838, 6, 0xE9CD06B0),
    (23757, 6, 0x2AF29253),
    (31676, 6, 0xB0F6F4DE),
    (39595, 6, 0x850232D5),
    (47514, 6, 0x7C8E84FD),
    (55433, 6, 0x82D11315),
    (63352, 6, 0xB1F04F73),
    (71271, 6, 0x2FF8A2F4),
    (79190, 6, 0xD0AAE6CC),
    (87109, 6, 0xC07451A7),
    (95028, 6, 0x22926E73),
    (102947, 6, 0xF4B7DEA3),
    (110866, 6, 0x611A5F2B),
    (118785, 6, 0x07AE73A7),
    (126704, 6, 0x4D26392E),
    (134623, 6, 0x9D274743),
    (142542, 6, 0x0D4CC7F6),
    (150461, 6, 0x0990B519),
    (158380, 6, 0x839C0D8A),
    (166299, 6, 0x8AA56B2C),
    (174218, 6, 0x9284B99A),
    (182137, 6, 0xD8DA640A),
    (190056, 6, 0xE19D3C48),
    (197975, 6, 0x93AE2B93),
    (7919, 7, 0x96241F01),
    (15838, 7, 0xD9232FD7),
    (23757, 7, 0xF921D85A),
    (31676, 7, 0x54DAA3A7),
    (39595, 7, 0xFEDD7F5D),
    (47514, 7, 0x8895EBE8),
    (55433, 7, 0x4C948ED7),
    (63352, 7, 0xF98CDE8E),
    (71271, 7, 0xC115C97C),
    (79190, 7, 0xA532B6D7),
    (87109, 7, 0x9D8C84F3),
    (95028, 7, 0x2AFFD535),
    (102947, 7, 0xFD3A9DA4),
    (110866, 7, 0xCB470D15),
    (118785, 7, 0x1BBC19AC),
    (126704, 7, 0x76C465D4),
    (134623, 7, 0x2893ECC7),
    (142542, 7, 0x24727240),
    (150461, 7, 0xAE6D53D6),
    (158380, 7, 0x269981B7),
    (166299, 7, 0x9E72ED05),
    (174218, 7, 0x62432BEA),
    (182137, 7, 0x3D17723A),
    (190056, 7, 0x6DE9AC79),
    (197975, 7, 0x7162E4E9),
    (7919, 9, 0x50B23007),
    (15838, 9, 0x841B2F18),
    (23757, 9, 0xA969F37D),
    (31676, 9, 0x1E0B940A),
    (39595, 9, 0xFB952328),
    (47514, 9, 0xD06BA6F4),
    (55433, 9, 0x8E907D9F),
    (63352, 9, 0x75EE470E),
    (71271, 9, 0x165D6C73),
    (79190, 9, 0xA47E5EF8),
    (87109, 9, 0x351B64E8),
    (95028, 9, 0x34D8BAF5),
    (102947, 9, 0xBDF6C57D),
    (110866, 9, 0x401D4220),
    (118785, 9, 0x6B96C931),
    (126704, 9, 0xDC11FEC8),
    (134623, 9, 0xA9D624F7),
    (142542, 9, 0xEE94F5CA),
    (150461, 9, 0x8B5BB927),
    (158380, 9, 0x40621A30),
    (166299, 9, 0x8C4DAB96),
    (174218, 9, 0xCE7C4751),
    (182137, 9, 0xFCEF560E),
    (190056, 9, 0x0956C2B5),
    (197975, 9, 0x76577714),
    (7919, 10, 0x56470F6D),
    (15838, 10, 0xE5095910),
    (23757, 10, 0x5BF9B3B7),
    (31676, 10, 0xE53FDA22),
    (39595, 10, 0xFE910B2C),
    (47514, 10, 0xCD04B893),
    (55433, 10, 0x175BDF61),
    (63352, 10, 0xE57ACA64),
    (71271, 10, 0x951BCAF2),
    (79190, 10, 0xE6AC595F),
    (87109, 10, 0x9AB3A221),
    (95028, 10, 0xAE38FF58),
    (102947, 10, 0x1898AA75),
    (110866, 10, 0x143B0310),
    (118785, 10, 0x816AEBCD),
    (126704, 10, 0xBF2055A3),
    (134623, 10, 0x758C6612),
    (142542, 10, 0x440426D4),
    (150461, 10, 0xE097E103),
    (158380, 10, 0x2AB51157),
    (166299, 10, 0xE1F10E4F),
    (174218, 10, 0xCEB09FC1),
    (182137, 10, 0xEAA58C06),
    (190056, 10, 0x124B5732),
    (197975, 10, 0xB9F5BAF5),
    (7919, 11, 0x373D49FB),
    (15838, 11, 0xAF0751F3),
    (23757, 11, 0x0661C73E),
    (31676, 11, 0xDF05F5F2),
    (39595, 11, 0x286959D3),
    (47514, 11, 0x9216431B),
    (55433, 11, 0x8B464418),
    (63352, 11, 0xEC8B0F8F),
    (71271, 11, 0x3B9164C4),
    (79190, 11, 0x81563CF9),
    (87109, 11, 0xC14C6E61),
    (95028, 11, 0xB7A266D8),
    (102947, 11, 0x357DFB8B),
    (110866, 11, 0x25C9305F),
    (118785, 11, 0x8C5A4B10),
    (126704, 11, 0x25C0B12B),
    (134623, 11, 0xDEEA2D74),
    (142542, 11, 0x9CA69414),
    (150461, 11, 0xE490A4DF),
    (158380, 11, 0x307EF1D6),
    (166299, 11, 0x69CF009C),
    (174218, 11, 0x359D12EA),
    (182137, 11, 0x5ADF8515),
    (190056, 11, 0x32354847),
    (197975, 11, 0x0CC13094),
];
