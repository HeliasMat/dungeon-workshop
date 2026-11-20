//! What the rules count with, read from the game once.

/// Balance figures, extracted once from the game data.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Constants {
    pub oran_berry_hp: i32,
    pub sitrus_berry_hp: i32,
    /// Max HP gained by eating one at full HP.
    pub oran_berry_full_hp_boost: i32,
    pub sitrus_berry_full_hp_boost: i32,
    pub life_seed_hp_boost: i32,
    /// Thrown items: their chance to hit, and by item id (1 to 10) the
    /// power of the sticks, thorns and spikes or the fixed damage of the
    /// rocks.
    pub thrown_hit_chance: i32,
    /// Damage of a Blast Seed, eaten (on the monster in front) and thrown.
    pub blast_seed_damage: (i32, i32),
    pub thrown_powers: [i32; 11],
    pub oren_berry_damage: i32,
    /// What Protein, Calcium, Iron and Zinc add to attack, special attack,
    /// defense and special defense.
    pub vitamin_boosts: [i32; 4],
    pub poison_damage: i32,
    /// Turns between two poison damage ticks.
    pub poison_cooldown: i32,
    /// Bad poison (Toxic): turns between two ticks, and the damage of each
    /// tick in turn (the last one again after).
    pub bad_poison_cooldown: i32,
    pub bad_poison_damage: [i32; 30],
    /// Leech Seed: HP drained a tick, turns between ticks, how long.
    pub leech_seed_drain: i32,
    pub leech_seed_cooldown: i32,
    pub leech_seed_turns: (i32, i32),
    /// How long before a yawning monster falls asleep, and a taunt.
    pub yawn_turns: (i32, i32),
    pub taunt_turns: (i32, i32),
    /// How long Sure Shot (Mind Reader, Lock-On), infatuation,
    /// invisibility, Magic Coat and Snatch last.
    pub sure_shot_turns: (i32, i32),
    pub infatuated_turns: (i32, i32),
    pub invisible_turns: (i32, i32),
    pub magic_coat_turns: (i32, i32),
    pub snatch_turns: (i32, i32),
    /// How long Mud Sport and Water Sport last on a floor, and Worry Seed's
    /// sleeplessness.
    pub sport_turns: (i32, i32),
    pub sleepless_turns: (i32, i32),
    /// How long Magnet Rise and Embargo last.
    pub magnet_rise_turns: (i32, i32),
    pub embargo_turns: (i32, i32),
    /// How long Encore lasts, and seeing things (an X-Eye Seed).
    pub encore_turns: (i32, i32),
    pub cross_eyed_turns: (i32, i32),
    /// How long Nightmare's sleep and Heal Block last.
    pub nightmare_turns: (i32, i32),
    pub heal_block_turns: (i32, i32),
    /// What Metronome may turn into, by rows.
    pub metronome: [[u16; 28]; 6],
    /// How long Counter and Metal Burst last, Mirror Coat, and the count
    /// of Perish Song.
    pub counter_turns: (i32, i32),
    pub mirror_coat_turns: (i32, i32),
    pub perish_turns: (i32, i32),
    /// Doom Desire, Future Sight: how long the user's damage is set, and to
    /// how much; Mirror Move; Imprison's pause.
    pub set_damage_turns: (i32, i32),
    pub set_damage: i32,
    pub mirror_move_turns: (i32, i32),
    pub imprison_turns: (i32, i32),
    /// How long Wrap holds the two.
    pub wrap_turns: (i32, i32),
    /// Return's and Frustration's damage by the user's IQ: `(below this
    /// IQ, this damage)`, the first that fits; Magnitude's
    /// by level; Vacuum Cut's.
    pub return_damage: [(i32, i32); 12],
    pub frustration_damage: [(i32, i32); 12],
    pub magnitude_damage: [i32; 7],
    pub vacuum_cut_damage: i32,
    /// What waking from a nightmare costs, and what waking from Rest's nap
    /// heals.
    pub nightmare_damage: i32,
    pub napping_heal: i32,
    /// How long a Destiny Bond lasts, and Transform.
    pub destiny_bond_turns: (i32, i32),
    pub transform_turns: (i32, i32),
    /// What a Gummi gives, by the eater's type (row) and the Gummi's
    /// (column): IQ and belly; a Wonder Gummi's IQ.
    pub gummi_iq: [[i32; 18]; 18],
    pub gummi_belly: [[i32; 18]; 18],
    pub wonder_gummi_iq: i32,
    /// A Gummi it likes (3 IQ or more) may raise a stat too: the chance in
    /// 100, the odds (one in this many) that it raises all four, by how
    /// much.
    pub gummi_boost_chance: i32,
    pub gummi_all_stats_odds: i32,
    pub gummi_stat_boost: i32,
    /// The berries, seeds and vitamins, by bit (item number): eaten, they
    /// give a little belly back.
    pub small_food: [u64; 10],
    /// What the Exp. Elite IQ skill adds to the experience earned, in
    /// percent.
    pub exp_elite_boost: i32,
    /// The damage of a move Me First takes, times this over 256.
    pub me_first: i32,
    /// What some moves' damage is multiplied by (8 fraction bits): Facade
    /// on a user with a status problem; by the quarter of its HP the user
    /// (Flail, Reversal, Water Spout, Eruption) or the target (Wring Out,
    /// Crush Grip) is at.
    pub facade_mult: i32,
    pub reversal_mult: [i32; 4],
    pub water_spout_mult: [i32; 4],
    pub eruption_mult: [i32; 4],
    pub wring_out_mult: [i32; 4],
    /// Constriction: its chance, how long it lasts, and what it costs how
    /// often.
    pub constriction_chance: i32,
    pub constriction_turns: (i32, i32),
    pub constriction_damage: i32,
    pub constriction_cooldown: i32,
    /// Last Resort: by how many of the user's other moves are out of PP
    /// (from one).
    pub last_resort_mult: [i32; 4],
    /// Natural Gift, in two halves: by the item held, `[item, type, power
    /// added]`.
    pub natural_gift: [[[i32; 3]; 17]; 2],
    /// What an exclusive item's experience boost adds, in percent.
    pub exclusive_exp_boost: i32,
    /// Wise Healer's extra healing, in percent; Intimidator's chance to make
    /// a striker cringe; Power Pitcher's multiplier of thrown items (8
    /// fraction bits).
    pub wise_healer_percent: i32,
    pub intimidator_chance: i32,
    pub power_pitcher: i32,
    /// The chances Arena Trap and Shadow Tag hold who strikes their bearer.
    pub arena_trap_chance: i32,
    pub shadow_tag_chance: i32,
    /// The chance an exclusive item answers a blow.
    pub exclusive_reaction_chance: i32,
    /// The chance of Secret Power's effect.
    pub secret_power_chance: i32,
    /// How long the user of Hyper Beam and its like rests after it.
    pub recharge_turns: (i32, i32),
    /// Rollout, Ice Ball: each successive hit's multiplier.
    pub rollout_mult: [i32; 10],
    /// How long Vital Throw lasts.
    pub vital_throw_turns: (i32, i32),
    /// How long a decoy (and Follow Me) and Conversion 2 last.
    pub decoy_turns: (i32, i32),
    pub conversion2_turns: (i32, i32),
    /// The type Conversion 2 turns to, by the type of the move that hits
    /// (0: none).
    pub conversion2_types: [u8; 18],
    /// The chance in 100 Grimy Food makes the eater ill.
    pub grimy_food_chance: i32,
    /// The chances of the moves of [`ROM_SECONDARY_EFFECTS`](super::moves::ROM_SECONDARY_EFFECTS), in its order,
    /// and Tri Attack's.
    pub secondary_chances: [i32; 32],
    pub tri_attack_chance: i32,
    /// Shortest and longest sleep, in turns.
    pub sleep_turns: (i32, i32),
    pub pitfall_trap_damage: i32,
    /// Belly lost every turn, in 16.16 fixed point (about 0.1).
    pub belly_lost_per_turn: i32,
    /// What a chain of linked moves costs in belly, by how many moves it
    /// strings together (nothing for one).
    pub link_belly: [i32; 5],
    /// Attack and defense multipliers of each stat stage (0 to 20, 10 being
    /// neutral), in 8-bit fixed point.
    pub offensive_stages: [i32; STAGE_COUNT],
    pub defensive_stages: [i32; STAGE_COUNT],
    /// Accuracy and evasion multipliers of each stage (the male tables).
    pub accuracy_stages: [i32; STAGE_COUNT],
    /// The same for females, on the attacker's side and the target's.
    pub female_accuracy_stages: [i32; STAGE_COUNT],
    pub female_evasion_stages: [i32; STAGE_COUNT],
    /// Turns between enemies once the team is wanted for theft.
    pub spawn_cooldown_alert: i32,
    pub evasion_stages: [i32; STAGE_COUNT],
    /// Damage multiplier of each move of [`CHARGED_MOVES`] when it is
    /// unleashed, in 8-bit fixed point.
    pub charged_multipliers: [i32; CHARGED_MOVES.len()],
    /// Shortest and longest duration of a speed change, in turns.
    pub speed_boost_turns: (i32, i32),
    /// Every how many turns Speed Boost makes its bearer faster.
    pub speed_boost_every: i32,
    pub speed_lower_turns: (i32, i32),
    /// Chances, in percent, of the stat boosts of Meteor Mash, Metal Claw,
    /// Steel Wing and the Silver Wind family.
    pub meteor_mash_chance: i32,
    pub metal_claw_chance: i32,
    pub steel_wing_chance: i32,
    pub silver_wind_chance: i32,
    /// Chances of the special stat changes of Mist Ball, Charge Beam and
    /// the Luster Purge family.
    pub mist_ball_chance: i32,
    pub charge_beam_chance: i32,
    pub luster_purge_chance: i32,
    pub reflect_turns: (i32, i32),
    pub light_screen_turns: (i32, i32),
    /// HP restored by Morning Sun, Synthesis, Moonlight and Roost in each
    /// weather, and by Heal Order.
    pub heal_tables: [[i32; 8]; 4],
    pub heal_order_hp: i32,
    /// Natural HP regeneration of the leader's species: the amount its
    /// regeneration counter must reach for 1 HP (0 without a ROM).
    pub regen_speed: i32,
    /// What Wish adds to that amount, and how long Wish lasts.
    pub wish_bonus_regen: i32,
    pub wish_turns: (i32, i32),
    /// HP restored by Ingrain and Aqua Ring, how often, and for how long.
    pub ingrain_regen: i32,
    pub ingrain_cooldown: i32,
    pub ingrain_turns: (i32, i32),
    pub aqua_ring_regen: i32,
    pub aqua_ring_cooldown: i32,
    pub aqua_ring_turns: (i32, i32),
    /// Damage of the Chestnut Trap, the Spiked Tile and Stealth Rock.
    pub chestnut_damage: i32,
    pub spiked_tile_damage: i32,
    pub stealth_rock_damage: i32,
    /// Damage of an explosion to monsters outside the team, by radius.
    pub explosion_damages: [i32; 3],
    /// Damage of a monster blown into an obstacle.
    pub blowback_damage: i32,
    pub endure_turns: (i32, i32),
    pub rest_turns: (i32, i32),
    pub curse_turns: (i32, i32),
    /// Turns between two hits of curse damage.
    pub curse_cooldown: i32,
    /// How long a weather move's weather lasts, and what sandstorms and
    /// hail do every ten turns.
    pub weather_move_turns: i32,
    pub bad_weather_damage: i32,
    /// What held items add to a stat: Power Band, Special Band, Defense
    /// Scarf, Zinc Band, Munch Belt.
    pub held_boosts: [i32; 5],
    /// Critical hit chance the Scope Lens, the Patsy Band and Sharpshooter
    /// add; accuracy the Detect Band and Quick Dodger take off an
    /// attacker; the chance of a critical hit with Type-Advantage Master.
    pub scope_lens_crit: i32,
    pub detect_band_drop: i32,
    pub quick_dodger_drop: i32,
    pub type_advantage_crit: i32,
    /// Weight of the regular attack in the AI's random pick, by how many
    /// moves the monster knows.
    pub ai_regular_attack_weights: [i32; 5],
    /// Turns between two attempts to bring a new enemy onto the floor, and
    /// how many enemies stop them, without and with a Monster House.
    pub spawn_cooldown: i32,
    pub spawn_cap: i32,
    pub spawn_cap_monster_house: i32,
    /// Type and damage multiplier (8-bit fixed point) of Weather Ball in
    /// each weather.
    pub weather_ball_types: [u8; 8],
    pub weather_ball_multipliers: [i32; 8],
    /// Shortest and longest duration of Protect, Safeguard and Mist.
    pub protect_turns: (i32, i32),
    pub safeguard_turns: (i32, i32),
    pub mist_turns: (i32, i32),
    pub lucky_chant_turns: (i32, i32),
    /// Damage of SonicBoom and Dragon Rage.
    pub sonicboom_damage: i32,
    pub dragon_rage_damage: i32,
    pub burn_damage: i32,
    /// Turns between two burn damage ticks.
    pub burn_cooldown: i32,
    /// Damage multiplier of a burned attacker, in 8-bit fixed point.
    pub burn_multiplier: i32,
    /// Shortest and longest duration of each status, in turns.
    pub burn_turns: (i32, i32),
    pub paralysis_turns: (i32, i32),
    pub frozen_turns: (i32, i32),
    pub cringe_turns: (i32, i32),
    pub confused_turns: (i32, i32),
}

impl Constants {
    /// HP restored by the healing move of this index in this weather.
    pub(crate) fn heal_amount(&self, index: usize, weather: u8) -> i32 {
        match self.heal_tables.get(index) {
            Some(table) => table[weather as usize % 8],
            None => self.heal_order_hp,
        }
    }

    /// Type and damage multiplier of Weather Ball.
    pub(crate) fn weather_ball(&self, weather: u8) -> (u8, i32) {
        let index = weather as usize % 8;
        (self.weather_ball_types[index], self.weather_ball_multipliers[index])
    }
}

pub(crate) const WEATHER_BALL: u16 = 31;

pub(crate) const SOLARBEAM: u16 = 151;

/// The moves charged on one turn and unleashed on the next: SolarBeam, Sky
/// Attack, Razor Wind, Focus Punch, Skull Bash, Fly, Dig, Dive, Bounce,
/// Shadow Force.
pub const CHARGED_MOVES: [u16; 10] = [151, 100, 60, 75, 348, 153, 8, 156, 205, 477];

/// The damage multiplier of a charged move, or `None` for the other moves
/// (and without extracted data).
pub fn charged_multiplier(constants: &Constants, move_id: u16) -> Option<i32> {
    let index = CHARGED_MOVES.iter().position(|&id| id == move_id)?;
    Some(constants.charged_multipliers[index]).filter(|&multiplier| multiplier != 0)
}

/// Damage of a one-hit KO.
pub(super) const OHKO_DAMAGE: i32 = 9999;

pub(crate) const SPEED_COUNTERS: usize = 5;

pub(crate) const MAX_SPEED_STAGE: i32 = 4;

pub const STAGE_COUNT: usize = 21;

pub const NEUTRAL_STAGE: i32 = 10;

impl Constants {
    /// Natural Gift with this item held: its type and power, or `None`
    /// for an item it does not know.
    pub fn natural_gift(&self, item: u16) -> Option<(u8, i32)> {
        self.natural_gift.iter().flatten().find(|entry| entry[0] == item as i32 && item != 0).map(|entry| (entry[1] as u8, 1 + entry[2]))
    }
}
