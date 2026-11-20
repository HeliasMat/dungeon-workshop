use super::read_text as read_scene;
use super::*;
use engine::speaker;
use engine::*;
use scenes::vocabulary::POKEMON as SAMPLE;

#[test]
fn names_said_are_written_as_the_game_reads_them() {
    let script =
        read_scene("cast\tpika\tm25\ncast\tevoli\tm133\nput\tevoli\t1\t2\tdown\nline\tevoli\t-\tHi {name:pika}!\nask\tYes|With {name:pika}\n", None, &SAMPLE);
    assert!(script.problems.is_empty(), "{:?}", script.problems);
    assert_eq!(script.lines[1].1, ["say", "m133", "-", "Hi {name:m25}!"]);
    assert_eq!(script.lines[3].1, ["ask", "Yes|With {name:m25}"]);
    assert_eq!(Beat::parse(&["recruit".to_string(), "m25".to_string()], &SAMPLE).ok(), Some(Beat::Host(vec!["recruit".to_string(), "m25".to_string()])));
}

#[test]
fn once_and_switch_are_written_out_as_ifs() {
    let text = "once\tseen\nsay\t0\t0\tHello!\nendonce\nswitch\tr\ncase\t0\nset\ta\t1\ncase\t1,2\nset\ta\t2\ndefault\nset\ta\t3\nendswitch\n";
    let lines: Vec<Vec<String>> = read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    let kinds: Vec<&str> = lines.iter().map(|line| line[0].as_str()).collect();
    assert_eq!(kinds, ["if", "set", "say", "endif", "if", "set", "elif", "set", "else", "set", "endif"]);
    assert_eq!(lines[0], ["if", "seen", "==", "0"]);
    assert_eq!(lines[6], ["elif", "r", "==", "1", "or", "r", "==", "2"]);
}

#[test]
fn sounds_by_name() {
    let parse = |line: &[&str]| Beat::parse(&line.iter().map(|field| field.to_string()).collect::<Vec<_>>(), &SAMPLE).ok();
    assert_eq!(parse(&["sound", "exclamation"]), Some(Beat::Sound(SoundBeat::Sound(engine::SoundId(8964)))));
    assert_eq!(parse(&["sound", "8962"]), Some(Beat::Sound(SoundBeat::Sound(engine::SoundId(8962)))));
    assert_eq!(parse(&["sound", "thunderclap"]), None);
}

#[test]
fn crowds_cheer_and_songs_keep_their_names() {
    let text = "cast\tjolly_master\tm40\ncast\tpika\tm25\ngroup\tcrowd\tjolly_master,pika\nmusic\tjolly_master\ncheer\tcrowd\nboo\tpika\tsweat\n";
    let lines: Vec<Vec<String>> = read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    assert_eq!(lines[0], ["music", "jolly_master"]);
    assert_eq!(lines[1], ["after", "0~20", "emote", "m40", "joy", "40"]);
    assert_eq!(lines[2], ["after", "0~20", "jump", "m40", "8", "12"]);
    assert_eq!(lines.len(), 6);
    assert_eq!(lines[5], ["after", "0~20", "emote", "m25", "sweat", "40"]);
}

#[test]
fn a_sequence_has_numbers_of_its_own_at_each_call() {
    let text = "define\tdouble\ncalc\t%x\t$1\t*\t2\ncalc\t$2\t%x\t%\t7\nlog\t{%x}\nenddef\ndo\tdouble\t3\ta\ndo\tdouble\t4\tb\n";
    let lines: Vec<Vec<String>> = read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    let (first, second) = (lines[0][1].clone(), lines[3][1].clone());
    assert!(first.starts_with("_x_") && second.starts_with("_x_") && first != second, "{lines:?}");
    assert_eq!(lines[1], ["calc", "a", first.as_str(), "%", "7"]);
    assert_eq!(lines[2][1], format!("{{{first}}}"));
}

#[test]
fn going_to_the_next_place_walking() {
    let text = "cast\tpika\tm25\nspot\tbridge\t40\t204\ncrossto\tright\t224\t@bridge\t0\tpika\n";
    let lines: Vec<Vec<String>> = read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    let kinds: Vec<&str> = lines.iter().map(|line| line[0].as_str()).collect();
    assert_eq!(kinds, ["step", "step", "wipe", "waitfade", "backdrop", "keeponly", "put", "walk", "put", "walk", "wipe", "waitfor", "waitfor"]);
    assert_eq!(lines[0], ["step", "0", "180", "0"]);
    assert_eq!(lines[5], ["keeponly", "0", "m25"]);
    assert_eq!(lines[6], ["put", "0", "-20", "204", "right"]);
    assert_eq!(lines[9], ["walk", "m25", "18", "214"]);
    assert_eq!(lines[10], ["wipe", "in", "left", "40"]);
}

#[test]
fn two_names_for_one_character_are_told() {
    let problems = read_scene(
        "cast\trock1\tm74\ncast\trock2\tm74\ncast\trock3\tm74.1\nput\trock1\t0\t0\tdown\nput\trock2\t0\t0\tdown\nput\trock3\t0\t0\tdown\n",
        None,
        &SAMPLE,
    )
    .problems;
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].says("m74.1"));
}

#[test]
fn words_compared_are_not_someone() {
    let text = "cast\traichu\tm26\nput\traichu\t0\t0\tdown\nasktext\tword\nif\tword == raichu or target == raichu\nendif\n";
    let lines: Vec<Vec<String>> = read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    assert_eq!(lines[2], ["if", "word", "==", "raichu", "or", "target", "==", "m26"]);
}

#[test]
fn a_table_of_ways_on() {
    let lines: Vec<Vec<String>> = read_scene("when\ta == 1 and b\tperfect\nwhen\ta == 1\tgood\notherwise\tmissed\nset\tx\t1\n", None, &SAMPLE)
        .lines
        .into_iter()
        .map(|(_, line)| line)
        .collect();
    let kinds: Vec<&str> = lines.iter().map(|line| line[0].as_str()).collect();
    assert_eq!(kinds, ["if", "gotoscene", "elif", "gotoscene", "else", "gotoscene", "endif", "set"]);
    assert_eq!(lines[0], ["if", "a", "==", "1", "and", "b"]);
    assert_eq!(lines[5], ["gotoscene", "missed"]);
}

#[test]
fn each_plays_a_sequence() {
    let text = "group\tduo\t0,10\ndefine\thop\njump\t$1\t5\nenddef\neach\tduo\tdo\thop\t$\n";
    let lines: Vec<Vec<String>> = read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    assert_eq!(lines, [["jump", "0", "5"], ["jump", "10", "5"]]);
}

#[test]
fn a_page_may_change_the_face() {
    let lines: Vec<Vec<String>> =
        read_scene("line\t0\tjoyous\t Hi!||[angry] Enough!|| Really.\n", None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    let faces: Vec<&str> = lines.iter().filter(|line| line[0] == "say").map(|line| line[2].as_str()).collect();
    assert_eq!(faces, ["joyous", "angry", "angry"]);
    assert_eq!(lines[2][3], " Enough!");
}

#[test]
fn short_ways_are_written_out() {
    let text = "repeat\t2\njump\t0\nrepeat\t2\nturn\t0\tdown\nendrepeat\nendrepeat\neach\t0,10\temote\t$\tjoy\nline\t0\t1\tHi\n";
    let lines: Vec<Vec<String>> = read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    let kinds: Vec<&str> = lines.iter().map(|line| line[0].as_str()).collect();
    assert_eq!(kinds, ["jump", "turn", "turn", "jump", "turn", "turn", "emote", "emote", "say", "said"]);
    assert_eq!(lines[7], ["emote", "10", "joy"]);
    assert_eq!(lines[8], ["say", "0", "1", "Hi"]);
    assert!(read_scene("endrepeat\n", None, &SAMPLE).problems.len() == 1);
}

#[test]
fn one_of_several_lines_is_written_out() {
    let lines: Vec<Vec<String>> = read_scene("oneof\t0\tjoyous\tHi!|Hello!\n", None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    let kinds: Vec<&str> = lines.iter().map(|line| line[0].as_str()).collect();
    assert_eq!(kinds, ["random", "if", "say", "said", "endif", "if", "say", "said", "endif"]);
    assert_eq!(lines[6], ["say", "0", "joyous", "Hello!"]);
}

#[test]
fn unknown_spots_are_told_and_speakers_keep_their_shown_name() {
    let script = read_scene("cast\tpika\tm25\nspot\there\t1\t2\nput\tpika\t@here\t1\ngo\tpika\t@there\t10\t0\nsay\tpika:???\t-\tWho?\n", None, &SAMPLE);
    assert_eq!(script.problems.len(), 1, "{:?}", script.problems);
    assert!(script.problems[0].says("@there"));
    assert_eq!(script.lines[2].1, ["say", "m25:???", "-", "Who?"]);
}

#[test]
fn chats_are_written_out_and_names_never_used_told() {
    let script = read_scene("cast\tpika\tm25\ncast\tlost\tm1\nspot\tnowhere\t1\t2\nchat\tpika\tHi!\t0\tHello!\n", None, &SAMPLE);
    let kinds: Vec<&str> = script.lines.iter().map(|(_, line)| line[0].as_str()).collect();
    assert_eq!(kinds, ["bubble", "waitbubbles", "bubble", "waitbubbles"]);
    assert_eq!(script.lines[0].1, ["bubble", "m25", "-", "Hi!"]);
    assert_eq!(script.problems.len(), 2, "{:?}", script.problems);
    assert!(script.problems[0].says("lost") && script.problems[1].says("nowhere"));
}

#[test]
fn sequences_take_defaults_and_tell_values_missing() {
    let text = "define\tsees\nturn\t$1\t$2?down\nemote\t$1\t$3\nenddef\ndo\tsees\t0\t\t!\ndo\tsees\t0\n";
    let script = read_scene(text, None, &SAMPLE);
    assert_eq!(script.lines[0].1, ["turn", "0", "down"]);
    assert_eq!(script.problems.len(), 1, "{:?}", script.problems);
    assert!(script.problems[0].says("3 value"));
}

#[test]
fn places_beside_a_named_one() {
    let lines = read_scene("cast\tpika\tm25\nspot\there\t100\t50\nput\tpika\t@here+20-5\t1\n", None, &SAMPLE).lines;
    assert_eq!(lines[0].1, ["put", "m25", "120", "45", "1"]);
}

#[test]
fn narrations_are_centred() {
    let lines = read_scene("narrate\tNight falls.\\nAll sleep.\n", None, &SAMPLE).lines;
    assert_eq!(lines[0].1, ["say", "-", "-", "[CN]Night falls.\\n[CN]All sleep."]);
    assert_eq!(lines[1].1, ["said"]);
}

#[test]
fn long_speeches_go_in_several_boxes() {
    let lines: Vec<Vec<String>> = read_scene("line\t0\tjoyous\tOne.||Two.\n", None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    assert_eq!(lines, [vec!["say", "0", "joyous", "One."], vec!["said"], vec!["say", "0", "joyous", "Two."], vec!["said"]]);
}

#[test]
fn groups_are_put_back() {
    let text = "group\tteam\t0,10\ngroup\teveryone\tteam,m25\neach\teveryone\temote\t$\tjoy\nwaitfor\tteam\n";
    let lines: Vec<Vec<String>> = read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[2], ["emote", "m25", "joy"]);
    assert_eq!(lines[3], ["waitfor", "0"]);
    assert_eq!(lines[4], ["waitfor", "10"]);
}

#[test]
fn names_are_put_back() {
    let text = "cast\tpika\tm25\nspot\tfountain\t120\t80\n# a comment\nput\tpika\t@fountain\t1\nsay\tpika\t0\tpika!\n";
    let lines: Vec<Vec<String>> = read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line).collect();
    assert_eq!(lines[0], ["put", "m25", "120", "80", "1"]);
    assert_eq!(lines[1], ["say", "m25", "0", "pika!"]);
    assert_eq!(speaker(Some(&"m25".to_string())), Some(Who::Monster(25)));
    assert_eq!(speaker(Some(&"-".to_string())), None);
}

#[test]
fn sequences_are_played_where_called() {
    let text = "define\tjoy\neffect\t$1\t93\njump\t$1\nenddef\ncast\tpika\t10\ndo\tjoy\tpika\ndo\tjoy\t0\n";
    let lines: Lines = read_scene(text, None, &SAMPLE).lines;
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], (6, vec!["effect".to_string(), "10".to_string(), "93".to_string()]));
    assert_eq!(lines[3], (7, vec!["jump".to_string(), "0".to_string()]));
    // A sequence never ended, one never defined: both told.
    let problems = read_scene("do\tnothing\ndefine\tdangling\njump\t0\n", None, &SAMPLE).problems;
    assert_eq!(problems.len(), 2, "{problems:?}");
}
