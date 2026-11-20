//! A film told in words and numbers: when lines play, who is where.

use std::time::{Duration, Instant};

use engine::{Cam, Who};
use shell::Screen;
use stage_view::ui::Canvas;

use super::Piece;
use super::json::{self, Value, json};
use crate::moment;
use preview::film::Film;
use preview::played::{Pass, Timeline, seconds};

fn played_through(piece: &Piece) -> Result<Timeline, String> {
    let mut pass = Pass::new(piece.film()?);
    pass.to_the_end();
    Ok(pass.timeline)
}

/// Who, by the film's name.
pub(super) fn name_of(piece: &Piece, film: &Film, who: Who) -> String {
    piece.written.as_ref().and_then(|written| written.name_of(who)).unwrap_or_else(|| film.play().sim.cast.who(who).1)
}

/// The size of a picture of a film's own, read once.
pub(super) fn picture_size(node: &engine::NodeRef, stage: &engine::Stage) -> Option<glam::Vec2> {
    type Read = std::sync::Mutex<std::collections::HashMap<String, Option<(u32, u32)>>>;
    static READ: std::sync::OnceLock<Read> = std::sync::OnceLock::new();
    let engine::NodeRef::Named(name) = node else { return None };
    let file = &stage.words.pictures.iter().find(|picture| picture.name == *name)?.file;
    let mut read = READ.get_or_init(Default::default).lock().ok()?;
    let size = *read.entry(file.clone()).or_insert_with(|| image::image_dimensions(file).ok());
    size.map(|(wide, high)| glam::Vec2::new(wide as f32, high as f32))
}

/// A node as it is on screen, for a program.
fn on_screen_json(on_screen: &engine::framed::Framed) -> Value {
    json!({
        "rect": on_screen.rect.map(|rect| json!([json::number(rect.x), json::number(rect.y), json::number(rect.w), json::number(rect.h)])),
        "in_frame": on_screen.in_frame,
        "visible": on_screen.visible,
        "covered": json::number(on_screen.covered),
        "covered_by_nodes": json::number(on_screen.covered_by_nodes),
        "covered_by_box": json::number(on_screen.covered_by_box),
        "size": json::point(on_screen.size),
        "edges": on_screen.edges.map(|edges| json!({
            "left": json::number(edges.left),
            "upper": json::number(edges.top),
            "right": json::number(edges.right),
            "lower": json::number(edges.bottom),
        })),
    })
}

/// A moment, for a program: the view, the box, what is heard, and every
/// node with its properties as the registry names them and how it is on
/// screen. See "For agents" in `film/FILMS.md`.
pub fn moment_json(piece: &Piece, film: &Film, frame: usize, timeline: &Timeline, heard: Option<&json::Heard>) -> Value {
    let play = film.play();
    let stage = &play.sim.stage;
    let line = play.sim.director.numbers().get(engine::LINE).map_or(0, |line| *line as i32);
    let view = film.view();
    let page = film.page().map(stage_view::ui::strip_tags);
    let framed = stage.on_screen(&view, page.is_some(), &|node| picture_size(node, stage));
    let nodes: Vec<Value> = framed
        .iter()
        .filter_map(|on_screen| {
            let node = stage.node(&on_screen.node)?;
            let mut properties = json::Map::new();
            for prop in engine::props::of_kind(node.kind) {
                if let Some(value) = stage.get(&on_screen.node, prop.id) {
                    properties.insert(prop.name.to_string(), json::prop_value(&value));
                }
            }
            let name = match &on_screen.node {
                engine::NodeRef::Who(who) => name_of(piece, film, *who),
                other => other.name(),
            };
            let mut told = json!({
                "ref": on_screen.node.name(),
                "name": name,
                "kind": node.kind.name(),
                "properties": properties,
                "screen": on_screen_json(on_screen),
                // (Where it hangs, and as it is drawn: its parents' added.)
                "parent": node.parent.as_ref().map(engine::NodeRef::name),
                "inherits": node.parent.as_ref().map(|_| node.inherit.words()),
                "world": {
                    "position": json::point(node.transform.pos),
                    "angle": json::number(node.transform.angle_deg),
                    "scale": [json::number(node.transform.scale.x), json::number(node.transform.scale.y)],
                    "opacity": json::number(node.looks.opacity),
                },
                "effects": node.looks.effects.list().iter().map(|effect| effect.name()).collect::<Vec<&str>>(),
            });
            if let engine::NodeRef::Who(who) = &on_screen.node
                && let Some(actor) = stage.cast.get(who)
            {
                told["body"] = json!({
                    "pose": play.sim.pose(*who),
                    "walk": actor.walks(),
                    "runs": actor.runs(),
                    "to": actor.walks().then(|| json::point(actor.goal())),
                    "statuses": stage.emotes.statuses_of(*who),
                    "hp": actor.bar.map(json::number),
                });
            }
            Some(told)
        })
        .collect();
    let follows = match stage.view.camera {
        Some(Cam::Who(who)) => Some(name_of(piece, film, who)),
        _ => None,
    };
    let camera = json!({
        "name": "main",
        "center": json::point(view.centre),
        "zoom": json::number(view.close),
        "screen": json::point(view.screen),
        "follows": follows,
        "kept": stage.view.framing.iter().map(|who| name_of(piece, film, *who)).collect::<Vec<String>>(),
        "glides": stage.view.pan.is_some(),
    });
    json!({
        "image": frame,
        "second": json::seconds(frame),
        "scene": timeline.scene_at(frame),
        "line": line,
        "place": play.sim.place,
        "cameras": [camera],
        "box": {"shown": page.is_some(), "upper": stage.words.box_top, "line_said": page},
        "bars": stage.words.hud.iter().flatten().map(|hud| play.sim.director.fill(hud)).collect::<Vec<String>>(),
        "sound": heard.map_or(Value::Null, |heard| heard.at(frame)),
        "tracks": stage.tracks.iter().map(|track| {
            let node = match &track.node {
                engine::NodeRef::Who(who) => name_of(piece, film, *who),
                other => other.name(),
            };
            let (value, _) = track.at(track.gone);
            json!({
                "node": node,
                "property": track.prop.prop().name,
                "t": json::number(track.gone / 60.0),
                "duration": json::number(track.length() / 60.0),
                "value": json::prop_value(&value),
                "keys": track.keys(),
                "mode": match track.mode {
                    engine::TrackMode::Once => "once",
                    engine::TrackMode::Loop => "loop",
                    engine::TrackMode::PingPong => "pingpong",
                },
                "route": matches!(track.course, engine::tracks::Through::Way(_)),
                "ends": track.ends(),
            })
        }).collect::<Vec<Value>>(),
        "nodes": nodes,
    })
}

/// A listener for what a film plays to be heard, kept from the start.
pub(super) fn listening() -> (std::sync::Arc<std::sync::Mutex<json::Heard>>, Box<dyn shell::sound::Hears>) {
    let heard = std::sync::Arc::new(std::sync::Mutex::new(json::Heard { keeping: true, ..json::Heard::default() }));
    (heard.clone(), Box::new(json::Listener(heard)))
}

/// A moment told: the view, then who is there.
pub fn state(piece: &Piece, asked: &[String], as_json: bool) -> Result<(), String> {
    let timeline = played_through(piece)?;
    let moments = moment::read(asked)?;
    let frames = moments.iter().map(|moment| moment.frame(&timeline)).collect::<Result<Vec<usize>, String>>()?;
    let (heard, listener) = listening();
    let mut pass = Pass::new(piece.film_with(piece.art()?, as_json.then_some(listener))?);
    let mut wanted: Vec<(usize, String)> = frames.iter().copied().zip(moments.iter().map(|moment| moment.name())).collect();
    wanted.sort();
    if as_json {
        let mut told = Vec::new();
        for (frame, name) in wanted {
            while pass.frame < frame && pass.step() {}
            let heard = heard.lock().map_err(|_| "the sound")?.clone();
            let mut moment = moment_json(piece, &pass.film, frame, &timeline, Some(&heard));
            moment["asked"] = json!(name);
            told.push(moment);
        }
        json::print(&json::answer("state", json!({"film": piece.name, "duration_s": json::seconds(timeline.frames), "moments": told})));
        return Ok(());
    }
    for (frame, _) in wanted {
        while pass.frame < frame && pass.step() {}
        let film = &pass.film;
        let line = film.play().sim.director.numbers().get(engine::LINE).map_or(0, |line| *line as i32);
        println!("{} at {} s (frame {frame}), scene {}, line {line}", piece.name, seconds(frame), pass.timeline.scene_at(frame));
        let moment = preview::state::moment(film, &|who| name_of(piece, film, who));
        println!("{}", moment.camera.told());
        moment.bars.iter().for_each(|bar| println!("bar: {bar}"));
        if let Some(said) = &moment.said {
            println!("line: {said}");
        }
        moment.bodies.iter().for_each(|body| println!("{}", body.told()));
        moment.objects.iter().for_each(|object| println!("{}", object.told()));
    }
    Ok(())
}

/// Each frame timed: on average, and the slowest once it has started (what
/// it reads to start being told apart).
pub fn measure(piece: &Piece, scale: usize) -> Result<(), String> {
    const STARTING: usize = 10;
    let mut pass = Pass::new(piece.film()?);
    let (mut canvas, mut screen) = (Canvas::default(), Screen::new(scale.clamp(1, 8)));
    let (mut thought, mut drawn, mut started) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
    let (mut worst_thought, mut worst_drawn) = ((Duration::ZERO, 0), (Duration::ZERO, 0));
    let whole = Instant::now();
    loop {
        let start = Instant::now();
        let going = pass.step();
        let middle = Instant::now();
        pass.film.draw(&mut canvas, &mut screen);
        let (think, draw) = (middle - start, middle.elapsed());
        (thought, drawn) = (thought + think, drawn + draw);
        if pass.frame < STARTING {
            started += think + draw;
        } else {
            worst_thought = worst_thought.max((think, pass.frame));
            worst_drawn = worst_drawn.max((draw, pass.frame));
        }
        if !going {
            break;
        }
    }
    let ms = |time: Duration| time.as_secs_f32() * 1000.0;
    let each = |time: Duration| ms(time) / pass.frame.max(1) as f32;
    println!(
        "{} frames in {:.1} s; start {:.0} ms; computing {:.2} ms, at worst {:.1} (frame {}); drawing {:.2} ms, at worst {:.1} (frame {})",
        pass.frame,
        whole.elapsed().as_secs_f32(),
        ms(started),
        each(thought),
        ms(worst_thought.0),
        worst_thought.1,
        each(drawn),
        ms(worst_drawn.0),
        worst_drawn.1
    );
    Ok(())
}
