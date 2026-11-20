//! Every song of the game by a word, as the scenes name it: its
//! number (in the user's own copy of the game), a label of ours, and the
//! moods it serves, for `scene_player music <word>`. Those named
//! `ambient` are sounds that last, not songs. And the sounds
//! the scenes name (`sound <name>`): the game names none; each is named from
//! what it sounds like, for `scene_player sounds <word>`.

/// A song of the game, as the scenes know it.
pub struct Song {
    pub word: &'static str,
    pub number: u16,
    pub title: &'static str,
    pub moods: &'static str,
}

macro_rules! songs {
    ($(($word:literal, $number:literal, $title:literal, $moods:literal)),* $(,)?) => {
        /// The songs by their words, for the scenes' vocabulary.
        pub const SONG_WORDS: &[(&str, u16)] = &[$(($word, $number)),*];
        pub const SONGS: &[Song] = &[$(Song { word: $word, number: $number, title: $title, moods: $moods }),*];
    };
}

/// A sound the scenes name, and when the game's scenes play it.
pub struct Noise {
    pub word: &'static str,
    pub number: u16,
    pub when: &'static str,
}

macro_rules! noises {
    ($(($name:ident, $word:literal, $number:expr, $when:literal)),* $(,)?) => {
        /// The sounds by what code names them (`noise::CLICK`): a name
        /// mistyped is a mistake the compiler finds.
        pub mod noise {
            $(
                #[doc = $when]
                pub const $name: u16 = $number;
            )*
        }

        /// The sounds by their words, for the scenes' vocabulary.
        pub const NOISE_WORDS: &[(&str, u16)] = &[$(($word, $number)),*];
        pub const NOISES: &[Noise] = &[$(Noise { word: $word, number: $number, when: $when }),*];
    };
}

noises! {
    (EXCLAMATION, "exclamation", 8964, "\"!\": they notice something"),
    (QUESTION, "question", 8962, "\"?\": they wonder"),
    (SWEAT, "sweat", 8972, "drops of sweat: embarrassed, overwhelmed"),
    (DROP, "drop", 8961, "a drop of embarrassment"),
    (SHOCK, "shock", 8968, "shocked"),
    (STUNNED, "stunned", 8973, "stunned"),
    (STARTLE, "startle", 8967, "\"!\" with a start"),
    (ALERT, "alert", 8978, "an alarmed \"!\""),
    (ANGER, "anger", 8971, "a jump of anger"),
    (ANNOYED, "annoyed", 8975, "annoyed"),
    (HOP, "hop", 8974, "a jump of surprise"),
    (FLASH, "flash", 5143, "a white flash over the whole screen"),
    (CHIRP, "chirp", 5122, "a bird's chirp"),
    (CLICK, "click", 0x3F02, "a line read, a choice, a menu"),
    (QUAKE, "quake", 5131, "the ground shakes, and the view with it"),
    (JOLT, "jolt", 6914, "a blow that shakes the view"),
    (ROAR, "roar", 6930, "a great roar, the view shaking"),
    (LIGHT, "light", 7183, "a white light that fills everything"),
    (VISION, "vision", 5133, "the screen lights up: a vision begins"),
    (DARKNESS, "darkness", 6925, "everything goes black"),
    (FALL, "fall", 7170, "they fall on their behind"),
    (ROCKS, "rocks", 7192, "rocks falling and hitting the ground"),
    (CONFETTI, "confetti", 7944, "confetti: the party"),
    (LAUGH, "laugh", 6418, "someone laughs"),
    (JOY, "joy", 5138, "amazed, overjoyed"),
    (STOMACH, "stomach", 8720, "a rumbling stomach"),
    (MEALTIME, "mealtime", 5123, "a call to the table"),
    (ANNOUNCE, "announce", 11021, "an announcement of the narrator, in its box"),
    (LOOKOUT, "lookout", 6422, "a lookout's shout"),
    (ITEM, "item", 8710, "an item shown, given or taken"),
    (PUZZLED, "puzzled", 8963, "\"?\" while looking around"),
    (SPLASH, "splash", 6675, "a dive into the water"),
    (TORRENT, "torrent", 5127, "a torrent of water rushing in, the screen white"),
    (AMAZED, "amazed", 8969, "amazed, with a start"),
    (EUREKA, "eureka", 8965, "\"!\": they found it"),
    (STRIKE, "strike", 6401, "a blow landed, the screen white"),
    (IMPACT, "impact", 5121, "a great crash, the screen white"),
    (WINGS, "wings", 7430, "flapping wings"),
    (MAP, "map", 6667, "a map being unfolded"),
    (BOSS, "boss", 5136, "a boss appears: the screen tearing apart"),
    (TREMOR, "tremor", 8195, "the ground trembles"),
    (SNICKER, "snicker", 6420, "a snicker"),
    (HOORAY, "hooray", 6669, "everyone cheers"),
    (WAKE_UP, "wake_up", 8970, "a rude awakening, anger"),
    (TUMBLE, "tumble", 7939, "they fall from high up"),
    (AURA, "aura", 7950, "an aura spreading"),
    (WATER_JET, "water_jet", 9735, "a jet of water"),
}

songs! {
    ("exploration_team", 1, "Exploration team", "epic start adventure"),
    ("menu", 2, "Menu", "calm menu"),
    ("dojo", 3, "Dojo", "upbeat training"),
    ("guards", 4, "Guards", "funny upbeat guild"),
    ("mission_accomplished", 5, "Mission accomplished", "joy victory"),
    ("welcome", 6, "Welcome", "joy start calm"),
    ("guild", 7, "Guild", "upbeat guild village"),
    ("guild_party", 8, "Guild party", "party joy upbeat guild"),
    ("treasure_town", 9, "Treasure town", "calm village joy"),
    ("upbeat", 10, "Upbeat", "upbeat joy adventure"),
    ("monster_house", 11, "Monster house", "danger battle dungeon"),
    ("shop", 12, "Shop", "calm funny village"),
    ("outlaw", 13, "Outlaw", "tension chase battle"),
    ("abyss_bottom", 14, "Abyss bottom", "dungeon dark"),
    ("battle", 15, "Battle", "battle danger"),
    ("dialga_battle", 16, "Dialga battle", "battle epic legendary"),
    ("dusknoir_battle", 17, "Dusknoir battle", "battle epic villain"),
    ("legendary_challenge", 18, "Legendary challenge", "battle legendary epic"),
    ("failure", 19, "Failure", "sad failure"),
    ("success", 20, "Success", "joy victory"),
    ("beach_cave", 21, "Beach cave", "dungeon sea calm"),
    ("drenched_bluff", 22, "Drenched bluff", "dungeon sea"),
    ("mt_bristle", 23, "Mt bristle", "dungeon mountain"),
    ("waterfall_cave", 24, "Waterfall cave", "dungeon water"),
    ("apple_woods", 25, "Apple woods", "dungeon forest joy"),
    ("craggy_coast", 26, "Craggy coast", "dungeon sea"),
    ("cave_and_side_path", 27, "Cave and side path", "dungeon"),
    ("mt_horn", 28, "Mt horn", "dungeon mountain"),
    ("foggy_forest", 29, "Foggy forest", "dungeon forest mystery"),
    ("steam_cave", 30, "Steam cave", "dungeon heat"),
    ("upper_steam_cave", 31, "Upper steam cave", "dungeon heat tension"),
    ("amp_plains", 32, "Amp plains", "dungeon upbeat"),
    ("far_amp_plains", 33, "Far amp plains", "dungeon tension"),
    ("northern_desert", 34, "Northern desert", "dungeon desert"),
    ("quicksand_cave", 35, "Quicksand cave", "dungeon desert"),
    ("quicksand_pit", 36, "Quicksand pit", "dungeon desert dark"),
    ("crystal_cave", 37, "Crystal cave", "dungeon mystery beautiful"),
    ("crystal_crossing", 38, "Crystal crossing", "dungeon mystery beautiful"),
    ("chasm_cave", 39, "Chasm cave", "dungeon dark"),
    ("dark_hill", 40, "Dark hill", "dungeon night dark"),
    ("sealed_ruin", 41, "Sealed ruin", "dungeon mystery"),
    ("sealed_ruin_pit", 42, "Sealed ruin pit", "dungeon mystery dark"),
    ("dusk_forest", 43, "Dusk forest", "dungeon forest dark"),
    ("deep_dusk_forest", 44, "Deep dusk forest", "dungeon forest dark tension"),
    ("dungeon_2", 45, "Dungeon 2", "dungeon"),
    ("brine_cave", 46, "Brine cave", "dungeon sea dark"),
    ("lower_brine_cave", 47, "Lower brine cave", "dungeon sea tension"),
    ("hidden_land", 48, "Hidden land", "dungeon mystery epic"),
    ("hidden_highland", 49, "Hidden highland", "dungeon epic tension"),
    ("temporal_tower", 50, "Temporal tower", "dungeon epic"),
    ("temporal_spire", 51, "Temporal spire", "dungeon epic tension"),
    ("mystifying_forest", 52, "Mystifying forest", "dungeon forest mystery"),
    ("blizzard_island", 53, "Blizzard island", "dungeon snow memory"),
    ("surrounded_sea", 54, "Surrounded sea", "dungeon sea"),
    ("dungeon_1", 55, "Dungeon 1", "dungeon"),
    ("aegis_cave", 56, "Aegis cave", "dungeon mystery legendary"),
    ("concealed_ruins", 57, "Concealed ruins", "dungeon mystery"),
    ("mt_travail", 58, "Mt travail", "dungeon mountain"),
    ("nightmare", 59, "Nightmare", "fear dark night"),
    ("miracle_sea", 60, "Miracle sea", "dungeon sea beautiful"),
    ("treeshroud_forest", 61, "Treeshroud forest", "dungeon forest sad"),
    ("dark_crater", 62, "Dark crater", "dungeon dark heat"),
    ("deep_dark_crater", 63, "Deep dark crater", "dungeon dark tension"),
    ("intro_variant", 64, "Intro variant", "start"),
    ("sentry_perfect", 65, "Sentry perfect", "joy victory guild"),
    ("sentry_good", 66, "Sentry good", "joy guild"),
    ("sentry_okay", 67, "Sentry okay", "calm guild"),
    ("sentry_failed", 68, "Sentry failed", "sad failure funny"),
    ("evening_beach", 69, "Evening beach", "calm evening sea emotion"),
    ("good_night", 70, "Good night", "calm night"),
    ("good_night_variant", 71, "Good night variant", "calm night"),
    ("end_of_day", 72, "End of day", "calm evening"),
    ("jolly_master", 73, "Jolly master", "funny guild"),
    ("worry", 74, "Worry", "tension worry"),
    ("darkness", 75, "Darkness", "villain fear dark"),
    ("oh_no", 76, "Oh no", "funny panic"),
    ("gears", 77, "Gears", "mystery emotion"),
    ("gears_remix", 78, "Gears remix", "mystery tension"),
    ("vision", 79, "Vision", "mystery vision"),
    ("future", 80, "Future", "dark sad future"),
    ("frozen_planet", 81, "Frozen planet", "dark sad future"),
    ("sea_of_time", 82, "Sea of time", "epic emotion"),
    ("destiny", 83, "Destiny", "tension epic"),
    ("time_restored", 84, "Time restored", "emotion victory beautiful"),
    ("memory", 85, "Memory", "sad emotion farewell"),
    ("peace", 86, "Peace", "calm emotion hope"),
    ("evening_beach_intro", 87, "Evening beach intro", "calm evening"),
    ("remembrance", 88, "Remembrance", "emotion memory"),
    ("ending_intro", 89, "Ending intro", "ending emotion"),
    ("ending", 90, "Ending", "ending emotion epic"),
    ("epilogue", 91, "Epilogue", "ending calm joy"),
    ("title_variant_1", 92, "Title variant 1", "menu"),
    ("title_variant_2", 93, "Title variant 2", "menu"),
    ("murky_forest", 94, "Murky forest", "dungeon forest dark"),
    ("sky_peak_cave", 95, "Sky peak cave", "dungeon mountain"),
    ("southern_jungle", 96, "Southern jungle", "dungeon forest upbeat"),
    ("sky_peak_coast", 97, "Sky peak coast", "dungeon sea"),
    ("spring_cave", 98, "Spring cave", "dungeon calm water"),
    ("lower_spring_cave", 99, "Lower spring cave", "dungeon water"),
    ("sea", 100, "Sea", "ambient sea"),
    ("storm", 101, "Storm", "ambient storm rain"),
    ("muffled_storm", 102, "Muffled storm", "ambient storm"),
    ("rumble", 103, "Rumble", "ambient quake"),
    ("rumble_loud", 104, "Rumble loud", "ambient quake"),
    ("rumble_very_loud", 105, "Rumble very loud", "ambient quake"),
    ("hot_water", 106, "Hot water", "ambient water spring"),
    ("peak_of_time_noise", 107, "Peak of time noise", "ambient mystery"),
    ("peak_of_time", 108, "Peak of time", "epic legendary tension"),
    ("heavy_steps", 110, "Heavy steps", "ambient footsteps fear"),
    ("chime", 111, "Chime", "ambient mystery"),
    ("electricity_1", 112, "Electricity 1", "ambient electricity"),
    ("electricity_2", 113, "Electricity 2", "ambient electricity"),
    ("electricity_3", 114, "Electricity 3", "ambient electricity"),
    ("fire_1", 115, "Fire 1", "ambient fire"),
    ("fire_2", 116, "Fire 2", "ambient fire"),
    ("fragment", 117, "Fragment", "ambient mystery"),
    ("rainbow", 118, "Rainbow", "ambient mystery"),
    ("rainbow_rising", 119, "Rainbow rising", "ambient mystery"),
    ("buzzing", 120, "Buzzing", "ambient"),
    ("going_home", 121, "Going home", "sad emotion farewell"),
    ("further_away", 122, "Further away", "sad emotion"),
    ("palkia_onslaught", 123, "Palkia onslaught", "battle legendary epic"),
    ("waterfall", 125, "Waterfall", "ambient water"),
    ("eating", 126, "Eating", "ambient meal funny"),
    ("sinister_string", 127, "Sinister string", "ambient fear"),
    ("flood", 128, "Flood", "ambient water"),
    ("pelipper_island", 129, "Pelipper island", "calm sea village"),
    ("title", 130, "Title", "menu epic"),
    ("comfort", 131, "Comfort", "emotion calm joy friendship"),
    ("dark_path", 132, "Dark path", "tension dark"),
    ("rising_fear", 133, "Rising fear", "fear tension"),
    ("team_skull", 134, "Team skull", "villain funny"),
    ("compassion", 135, "Compassion", "sad emotion"),
    ("beyond_the_dream", 136, "Beyond the dream", "emotion hope epic"),
    ("unease", 137, "Unease", "tension worry mystery"),
    ("one_for_all", 138, "One for all", "battle epic upbeat"),
    ("all_for_one", 139, "All for one", "victory epic"),
    ("boulder_quarry", 140, "Boulder quarry", "dungeon"),
    ("spring_cave_depths", 141, "Spring cave depths", "dungeon water"),
    ("star_cave", 142, "Star cave", "dungeon mystery beautiful"),
    ("deep_star_cave", 143, "Deep star cave", "dungeon mystery"),
    ("limestone_cavern", 144, "Limestone cavern", "dungeon"),
    ("deep_limestone_cavern", 145, "Deep limestone cavern", "dungeon dark"),
    ("dungeon_3", 146, "Dungeon 3", "dungeon"),
    ("fortune_ravine", 147, "Fortune ravine", "dungeon"),
    ("fortune_ravine_depths", 148, "Fortune ravine depths", "dungeon tension"),
    ("barren_valley", 149, "Barren valley", "dungeon desert"),
    ("dark_wasteland", 150, "Dark wasteland", "dungeon dark"),
    ("spacial_cliffs", 151, "Spacial cliffs", "dungeon mystery"),
    ("dark_ice_mountain", 152, "Dark ice mountain", "dungeon snow dark"),
    ("icicle_forest", 153, "Icicle forest", "dungeon snow forest"),
    ("vast_ice_mountain", 154, "Vast ice mountain", "dungeon snow"),
    ("vast_ice_mountain_peak", 155, "Vast ice mountain peak", "dungeon snow epic"),
    ("sky_peak_forest", 156, "Sky peak forest", "dungeon forest"),
    ("sky_peak_prairie", 157, "Sky peak prairie", "dungeon calm joy"),
    ("sky_peak_snowfield", 158, "Sky peak snowfield", "dungeon snow"),
    ("sky_peak_final_pass", 159, "Sky peak final pass", "dungeon epic mountain"),
    ("spinda_cafe", 160, "Spinda cafe", "party joy village"),
    ("ludicolo_dance", 161, "Ludicolo dance", "party funny dance"),
    ("illusion_chamber", 162, "Illusion chamber", "mystery"),
    ("impossible", 163, "Impossible", "shock tension revelation"),
    ("defend_globe", 164, "Defend globe", "battle upbeat"),
    ("defend_globe_end", 165, "Defend globe end", "victory"),
    ("team_charm", 166, "Team charm", "joy heroes"),
    ("here_comes_team_charm", 167, "Here comes team charm", "joy upbeat heroes"),
    ("new_life", 168, "New life", "emotion hope"),
    ("living_spirit", 169, "Living spirit", "emotion epic"),
    ("pride", 170, "Pride", "victory emotion"),
    ("morning_sun", 171, "Morning sun", "morning calm joy"),
    ("new_world", 172, "New world", "emotion hope beautiful"),
    ("thoughts_of_friends", 173, "Thoughts of friends", "emotion friendship sad"),
    ("life_goes_on", 174, "Life goes on", "ending calm hope"),
    ("not_a_miracle", 175, "Not a miracle", "emotion epic"),
    ("message_on_the_wind", 176, "Message on the wind", "emotion calm mystery"),
    ("joyful_exploration", 177, "Joyful exploration", "joy upbeat adventure"),
    ("shaymin_village", 178, "Shaymin village", "calm village joy"),
    ("team_charm_intro", 179, "Team charm intro", "joy heroes"),
    ("rumble_4", 180, "Rumble 4", "ambient quake"),
    ("rumble_5", 181, "Rumble 5", "ambient quake"),
    ("muffled_sea", 182, "Muffled sea", "ambient sea"),
    ("wind_1", 183, "Wind 1", "ambient wind"),
    ("limestone_cavern_noise", 184, "Limestone cavern noise", "ambient water"),
    ("fire_3", 185, "Fire 3", "ambient fire"),
    ("sky_peak_clearing_noise", 186, "Sky peak clearing noise", "ambient wind"),
    ("rain", 187, "Rain", "ambient rain wind storm"),
    ("ice_peak_noise", 188, "Ice peak noise", "ambient wind snow"),
    ("spring_noise", 189, "Spring noise", "ambient water"),
    ("wind_2", 190, "Wind 2", "ambient wind"),
    ("jungle_exit_noise", 191, "Jungle exit noise", "ambient forest"),
    ("quarry_noise", 192, "Quarry noise", "ambient"),
    ("waterfall_vision_noise", 193, "Waterfall vision noise", "ambient vision"),
    ("team_charm_variant_1", 200, "Team charm variant 1", "joy heroes"),
    ("team_charm_jingle", 201, "Team charm jingle", "joy heroes"),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each word names one song, each song has one word, each a mood.
    #[test]
    fn each_song_has_its_own_word() {
        let mut words: Vec<&str> = SONGS.iter().map(|song| song.word).collect();
        let mut numbers: Vec<u16> = SONGS.iter().map(|song| song.number).collect();
        words.sort();
        numbers.sort();
        assert!(words.windows(2).all(|two| two[0] != two[1]), "a word twice");
        assert!(numbers.windows(2).all(|two| two[0] != two[1]), "a song twice");
        assert!(
            SONGS
                .iter()
                .all(|song| !song.moods.is_empty() && song.word.bytes().all(|letter| letter.is_ascii_lowercase() || letter == b'_' || letter.is_ascii_digit()))
        );
    }

    /// Each sound's word is its own, and says when it plays.
    #[test]
    fn each_sound_has_its_own_word() {
        let mut words: Vec<&str> = NOISES.iter().map(|noise| noise.word).collect();
        words.sort();
        assert!(words.windows(2).all(|two| two[0] != two[1]), "a word twice");
        assert!(NOISES.iter().all(|noise| !noise.when.is_empty()));
    }
}
