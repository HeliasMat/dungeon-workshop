use crate::ast::{ArgKind, Element, Event, ExprKind, Kind, Opening, PartKind, Piece};
use crate::diag::{applied, codes};
use crate::format::expression;
use crate::{Source, formatted, parse};

const FILM: &str = "title: Fishing for two
uses welcome
pinchy = krabby

# He casts several times and counts his catches.
define long_cast who:pokemon to where:place [turns:number=3] -> number
  let catches = 0
  repeat turns
    throw who line to where
    if chance(50) and catches < 9
      catches = catches + 1
    end
  end
  return catches
end

scene: beach_evening pikachu_angler piplup
moment: the bet
frame pikachu_angler piplup # everyone
pikachu_angler (happy, fast): Shall we bet? &
let n = long_cast(pikachu_angler, (480, 108))
if n >= 2
  piplup (surprised): {n} fish?! [1s] Well done.
else if n = 1
  piplup: Only one...
else
  piplup (joyous): Nothing at all!
end
walk piplup to right & as escape
when piplup arrives : emote pikachu_angler question
every random(3s, 4s) as noises : sound click
at escape + 2s
  put pinchy {piplup.x + 16} {piplup.y}
end
for each q, rank in [pikachu_angler, piplup]
  jump q
end
choice
option Let's go!
  wait 1s
option We stay
end
fight directed pikachu_angler against pinchy level 20
  pikachu_angler Thunderbolt pinchy -20
  when health(pinchy) <= 50 : pinchy (angry): Ouch!
end
wait escape
";

#[test]
fn a_film_reads_into_its_tree() {
    let parsed = parse(FILM);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let file = &parsed.file;
    assert_eq!(file.scenes.len(), 1);
    assert!(matches!(&file.head[0].kind, Kind::Title(title) if title == "Fishing for two"));
    assert!(matches!(&file.head[1].kind, Kind::Uses(name) if name.text == "welcome"));
    assert!(matches!(&file.head[2].kind, Kind::Naming { name, species } if name.text == "pinchy" && species.text == "krabby"));
    let recipe = file.head.iter().find_map(|stmt| match &stmt.kind {
        Kind::Block(block) => match &block.opening {
            Opening::Recipe(recipe) => Some(recipe),
            _ => None,
        },
        _ => None,
    });
    let recipe = recipe.expect("the recipe");
    assert_eq!(recipe.form.word.text, "long_cast");
    assert!(matches!(&recipe.form.elements[1], Element::Word(word) if word.text == "to"));
    assert!(matches!(&recipe.form.elements[3], Element::Optional(inner) if matches!(&inner[0], Element::Param { default: Some(three), .. } if three == "3")));
    let scene = &file.scenes[0];
    assert_eq!(scene.args.iter().map(|arg| arg.text.as_str()).collect::<Vec<_>>(), ["beach_evening", "pikachu_angler", "piplup"]);
    let kinds: Vec<&Kind> = scene.body.iter().map(|stmt| &stmt.kind).collect();
    assert!(matches!(kinds[0], Kind::Moment(moment) if moment == "the bet"));
    assert_eq!(scene.body[1].comment.as_deref(), Some("everyone"));
    // (A colour is no comment.)
    assert!(crate::line::colour_word("ff8800") && crate::line::colour_word("f80 then") && crate::line::colour_word("ff880080"));
    assert!(!crate::line::colour_word(" a word") && !crate::line::colour_word("ff88000") && !crate::line::colour_word("zap"));
    let Kind::Said(said) = kinds[2] else { panic!("a said line") };
    assert_eq!(said.who.text, "pikachu_angler");
    assert_eq!(said.manners.iter().map(|manner| manner.text.as_str()).collect::<Vec<_>>(), ["happy", "fast"]);
    assert_eq!(said.text, "Shall we bet?");
    assert!(scene.body[2].beside.is_some());
    let Kind::Block(test) = kinds[4] else { panic!("an if") };
    assert_eq!(test.parts.len(), 2);
    assert!(matches!(test.parts[0].kind, PartKind::ElseIf(_)));
    let Kind::Said(said) = &test.body[0].kind else { panic!() };
    assert!(matches!(said.pieces.as_slice(), [Piece::Value(_), Piece::Text(_), Piece::Pause(_), Piece::Text(_)]));
    assert_eq!(scene.body[5].beside.as_ref().and_then(|beside| beside.named.as_ref()).map(|name| name.text.as_str()), Some("escape"));
    let Kind::Block(when) = kinds[6] else { panic!("a when") };
    assert!(when.short && matches!(&when.opening, Opening::When(Event::Arrives(who)) if who.text == "piplup"));
    let Kind::Block(every) = kinds[7] else { panic!("an every") };
    assert_eq!(every.named.as_ref().map(|name| name.text.as_str()), Some("noises"));
    let Kind::Block(at) = kinds[8] else { panic!("an at") };
    let Kind::Call(call) = &at.body[0].kind else { panic!() };
    assert!(matches!(&call.args[1].kind, ArgKind::Value(value) if expression(value) == "piplup.x + 16"));
    let Kind::Block(each) = kinds[9] else { panic!() };
    assert!(matches!(&each.opening, Opening::ForEach { rank: Some(rank), .. } if rank.text == "rank"));
    let Kind::Block(choice) = kinds[10] else { panic!() };
    assert_eq!(choice.parts.len(), 2);
    let Kind::Block(fight) = kinds[11] else { panic!() };
    assert!(matches!(&fight.opening, Opening::Fight(args) if args.len() == 5));
    let Kind::Call(turn) = &fight.body[0].kind else { panic!() };
    assert!(matches!(turn.args[2].kind, ArgKind::Number(number) if number == -20.0));
}

#[test]
fn written_back_twice_a_text_does_not_change() {
    let once = formatted(FILM);
    assert_eq!(once, FILM, "the film is already in order");
    let untidy = "scene: beach a b\nif n>2\njump a\n   else\njump   b  &\nend\n\n\n\ntogether\njump a\n  end\n";
    let tidy = formatted(untidy);
    assert_eq!(tidy, "scene: beach a b\nif n > 2\n  jump a\nelse\n  jump b &\nend\n\ntogether\n  jump a\nend\n");
    assert_eq!(formatted(&tidy), tidy);
}

#[test]
fn calculations_bind_as_said() {
    let read = |text: &str| expression(&crate::expr::expression(text, 0).expect(text));
    assert_eq!(read("1+2*3"), "1 + 2 * 3");
    assert_eq!(read("(1+2)*3"), "(1 + 2) * 3");
    assert_eq!(read("a - (b - c)"), "a - (b - c)");
    assert_eq!(read("not a and b or c"), "not a and b or c");
    assert_eq!(read("health(a) <= 50 and a.hp > 0"), "health(a) <= 50 and a.hp > 0");
    assert_eq!(read("l[1].x + random(1, 6)"), "l[1].x + random(1, 6)");
    assert_eq!(read("clock + 1.5s"), "clock + 1.5s");
    assert_eq!(read("[a, b] + [\"c\"]"), "[a, b] + [\"c\"]");
    let point = crate::expr::expression("(480, 108)", 0).expect("a point");
    assert!(matches!(point.kind, ExprKind::Point(..)));
    assert!(crate::expr::expression("1 < n < 5", 0).is_err(), "two comparisons in a row");
}

/// Line and column of a diagnostic, its code.
fn told(text: &str) -> Vec<(usize, usize, &'static str)> {
    let source = Source::new(text);
    parse(text)
        .diagnostics
        .iter()
        .map(|wrong| {
            let at = source.position(wrong.bytes().start);
            (at.line, at.column, wrong.code_or(""))
        })
        .collect()
}

#[test]
fn what_is_wrong_is_said_where_it_is() {
    assert_eq!(told("scene: beach a\nif n >= \nend\n"), [(2, 8, codes::EXPRESSION)]);
    assert_eq!(told("scene: beach a\nrepeat 3\njump a\n"), [(2, 1, codes::UNCLOSED)]);
    assert_eq!(told("scene: beach a\njump a\nend\n"), [(3, 1, codes::STRAY_END)]);
    assert_eq!(told("scene: beach a\nelse\n"), [(2, 1, codes::STRAY_MIDDLE)]);
    assert_eq!(told("scene: beach a\ntogether\n  put a {a.x +}\nend\n"), [(3, 15, codes::EXPRESSION)]);
    assert_eq!(told("scene: beach a\na: Hehe, ' {n '!\n"), [(2, 12, codes::UNCLOSED_MARK)]);
    assert_eq!(told("define f who:pokemon [x:number\nend\n"), [(1, 23, codes::FORM)]);
    // A block left open at the next scene is closed there, and said.
    assert_eq!(told("scene: beach a\ntogether\njump a\nscene: town a\njump a\n"), [(2, 1, codes::UNCLOSED)]);
    // Every wrong line is said, not only the first.
    assert_eq!(told("scene: beach a\nif\njump a\nend\nrepeat\nend\n").len(), 2);
}

#[test]
fn old_forms_are_known_and_fixed() {
    let fix_of = |text: &str| {
        let parsed = parse(text);
        let wrong = parsed.diagnostics.iter().find(|wrong| wrong.code == Some(codes::OLD_FORM)).unwrap_or_else(|| panic!("{text}: {:?}", parsed.diagnostics));
        applied(text, &wrong.fixes[0].edits)
    };
    assert_eq!(fix_of("scene: beach a\nif n > 2\njump a\nendif\n"), "scene: beach a\nif n > 2\njump a\nend\n");
    assert_eq!(fix_of("define f who:pokemon\nif other:\njump who\nend\nend\n"), "define f who:pokemon\nif other\njump who\nend\nend\n");
    assert_eq!(fix_of("scene: beach a\nevery 3s~4s sound click\n"), "scene: beach a\nevery random(3s, 4s) : sound click\n");
    let parsed = parse("scene: beach a\nchoice Yes | No\nend\n");
    assert_eq!(parsed.diagnostics[0].code, Some(codes::OLD_FORM));
    let parsed = parse("scene: arena a c\nfight directed a against c\nwhen c 50 | c (angry): Enough!\nend\n");
    assert_eq!(parsed.diagnostics.iter().map(|wrong| wrong.code).collect::<Vec<_>>(), [Some(codes::OLD_FORM)]);
}

#[test]
fn a_said_line_is_not_a_header_nor_a_block() {
    let parsed = parse("scene: beach a moment\na: Yes.\na (joyous): If: all.\nnarrator: The next day...\n");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let said: Vec<String> = parsed.file.scenes[0]
        .body
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            Kind::Said(said) => Some(format!("{}={}", said.who.text, said.text)),
            _ => None,
        })
        .collect();
    assert_eq!(said, ["a=Yes.", "a=If: all.", "narrator=The next day..."]);
}

/// Every film, recipe and model the project has reads without a panic, and
/// written back in order keeps every word (only the spaces may change), and
/// a second time does not change.
#[test]
fn the_corpus_reads_and_is_written_back() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    let mut files = Vec::new();
    let games = std::fs::read_dir(format!("{root}/tests/games"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|game| format!("tests/games/{}/films", game.file_name().to_string_lossy()));
    let folders = ["library/films", "library/films/recipes", "film/recipes", "film/recipes/dungeon", "film/recipes/fishing"].map(String::from);
    for folder in folders.into_iter().chain(games) {
        let Ok(found) = std::fs::read_dir(format!("{root}/{folder}")) else { continue };
        files.extend(found.flatten().map(|entry| entry.path()).filter(|path| path.extension().is_some_and(|extension| extension == "film")));
    }
    assert!(files.len() >= 10, "{} files", files.len());
    let squeezed = |text: &str| text.chars().filter(|letter| !letter.is_whitespace()).collect::<String>();
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let once = formatted(&text);
        // `endif` is written `end`, `if x:` `if x`: the only changes in the words.
        let words: String = text
            .lines()
            .map(|line| match line.trim() {
                "endif" => "end",
                test if test.starts_with("if ") => test.strip_suffix(':').unwrap_or(test),
                _ => line,
            })
            .collect();
        assert_eq!(squeezed(&once), squeezed(&words), "{}: a word changed", path.display());
        assert_eq!(formatted(&once), once, "{}: not stable", path.display());
    }
}

#[test]
fn a_line_from_here_reads_and_is_written_back() {
    let text = "scene: beach a\nat +2s : jump a\nat +1.5s\n  jump a\nend\n";
    assert!(told(text).is_empty(), "{:?}", told(text));
    assert_eq!(formatted(text), "scene: beach a\nat +2s : jump a\nat +1.5s\n  jump a\nend\n");
    assert!(!told("scene: beach a\nat +2s 3 : jump a\n").is_empty(), "nothing after the time");
}

/// The story's own writing: an answer in quotes with its test and `once`,
/// words chosen by a test inside a line said, a line's id written last.
#[test]
fn answers_words_chosen_and_ids() {
    let text = "scene: beach a\nchoice\noption \"Give an apple\" if apples > 0 once\noption \"Leave\"\noption Old style\nend\na: I {if met: know | do not know} you. #k3f9\na: We're #1 | fr: On est #1\n";
    let parsed = parse(text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let scene = &parsed.file.scenes[0];
    let Kind::Block(choice) = &scene.body[0].kind else { panic!("a choice") };
    let options: Vec<(&str, bool, bool)> = choice
        .parts
        .iter()
        .map(|part| match &part.kind {
            PartKind::Option { words, test, once, .. } => (words.as_str(), test.is_some(), *once),
            _ => panic!("an option"),
        })
        .collect();
    assert_eq!(options, [("Give an apple", true, true), ("Leave", false, false), ("Old style", false, false)]);
    let Kind::Said(said) = &scene.body[1].kind else { panic!("a line said") };
    assert!(
        matches!(said.pieces.as_slice(), [Piece::Text(_), Piece::Choose { then, otherwise, .. }, Piece::Text(_)] if then.len() == 1 && otherwise.len() == 1),
        "{:?}",
        said.pieces
    );
    assert_eq!(said.id.as_ref().map(|id| id.text.as_str()), Some("k3f9"));
    assert_eq!(&text[said.id.as_ref().map_or(0, |id| id.span.start)..][..4], "k3f9");
    let Kind::Said(said) = &scene.body[2].kind else { panic!("a line said") };
    assert!(said.id.is_none() && said.languages.len() == 1, "{said:?}");
    // (Written back as written.)
    assert_eq!(formatted(text), text);
    // (What does not read is told.)
    let wrong = parse("scene: beach a\nchoice\noption \"Go\" when it rains\nend\na: {if met know}\n");
    assert_eq!(wrong.diagnostics.iter().map(|wrong| wrong.code).collect::<Vec<_>>(), [Some(codes::EXTRA), Some(codes::EXPRESSION)]);
}
