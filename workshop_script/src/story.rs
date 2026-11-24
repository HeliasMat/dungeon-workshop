//! A film seen as a story, not as code: as a screenplay is read (each line said by its portrait, what is done in
//! italics, each scene a heading; written out as a page to print), as a corkboard (a card each scene, put in
//! another order by dragging), by its characters (who speaks where and how much; each one's sheet), as a graph
//! (its scenes, their choices, the ways from place to place).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

use workshop_base::theme;

/// How the Script shows the film.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Text,
    Screenplay,
    Board,
    Characters,
    Graph,
}

impl View {
    pub const ALL: [View; 5] = [View::Text, View::Screenplay, View::Board, View::Characters, View::Graph];

    pub fn name(self) -> &'static str {
        match self {
            View::Text => "Text",
            View::Screenplay => "Screenplay",
            View::Board => "Board",
            View::Characters => "Characters",
            View::Graph => "Graph",
        }
    }
}

/// A line of a scene, as a story reads it.
#[derive(Clone, Debug, PartialEq)]
pub enum Beat {
    Said { who: String, face: Option<String>, text: String },
    Done(String),
    Moment(String),
    Note(String),
    Choice(Vec<String>),
    Way { by: String, to: String },
}

/// A scene: its line (from 1) and its last, its place's word, those named on its line, a summary (its first
/// comment), its lines (each with its own line number).
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub line: usize,
    pub end: usize,
    pub place: String,
    pub cast: Vec<String>,
    pub summary: String,
    pub beats: Vec<(usize, Beat)>,
}

/// The film's title, and its scenes.
pub struct Story {
    pub title: String,
    pub scenes: Vec<Scene>,
}

impl Story {
    /// The film read as a story.
    pub fn of(text: &str) -> Story {
        let lines: Vec<&str> = text.split('\n').collect();
        let mut title = String::new();
        let mut scenes: Vec<Scene> = Vec::new();
        let mut in_choice: Option<usize> = None;
        for (at, raw) in lines.iter().enumerate() {
            let number = at + 1;
            let line = raw.trim();
            if let Some(said) = line.strip_prefix("title:") {
                title = said.trim().to_string();
                continue;
            }
            if let Some(rest) = line.strip_prefix("scene:") {
                let mut words = rest.split_whitespace();
                let place = words.next().unwrap_or_default().to_string();
                scenes.push(Scene { line: number, end: number, place, cast: words.map(str::to_string).collect(), summary: String::new(), beats: Vec::new() });
                in_choice = None;
                continue;
            }
            let Some(scene) = scenes.last_mut() else { continue };
            scene.end = number;
            if line.is_empty() {
                continue;
            }
            if let Some(note) = line.strip_prefix('#') {
                if scene.summary.is_empty() && scene.beats.is_empty() {
                    scene.summary = note.trim().to_string();
                } else {
                    scene.beats.push((number, Beat::Note(note.trim().to_string())));
                }
                continue;
            }
            if let Some(name) = line.strip_prefix("moment:") {
                scene.beats.push((number, Beat::Moment(name.trim().to_string())));
                continue;
            }
            if line == "choice" {
                scene.beats.push((number, Beat::Choice(Vec::new())));
                in_choice = Some(scene.beats.len() - 1);
                continue;
            }
            if let (Some(option), Some(at)) = (line.strip_prefix("option "), in_choice) {
                if let Some((_, Beat::Choice(options))) = scene.beats.get_mut(at) {
                    options.push(option.trim().to_string());
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix("passage ") {
                let words: Vec<&str> = rest.split_whitespace().collect();
                if let (Some(by), Some(to)) = (words.first(), words.iter().position(|word| *word == "to").and_then(|at| words.get(at + 1))) {
                    scene.beats.push((number, Beat::Way { by: by.to_string(), to: to.to_string() }));
                    continue;
                }
            }
            if let Some((who, said_at)) = crate::speaker(raw) {
                let head = &raw[..said_at - 1];
                let face = head
                    .find('(')
                    .and_then(|open| head[open + 1..].split(')').next())
                    .map(|inside| inside.split(',').next().unwrap_or_default().trim().to_string())
                    .filter(|face| !face.is_empty());
                scene.beats.push((number, Beat::Said { who: who.to_string(), face, text: raw[said_at..].trim().to_string() }));
                continue;
            }
            scene.beats.push((number, Beat::Done(line.to_string())));
        }
        Story { title, scenes }
    }

    /// Who speaks (in the order they first do), and how many lines each says in each scene.
    pub fn speakers(&self) -> (Vec<String>, BTreeMap<(String, usize), usize>) {
        let mut order: Vec<String> = Vec::new();
        let mut counts = BTreeMap::new();
        for (at, scene) in self.scenes.iter().enumerate() {
            for who in &scene.cast {
                if !order.contains(who) {
                    order.push(who.clone());
                }
            }
            for (_, beat) in &scene.beats {
                if let Beat::Said { who, .. } = beat {
                    if !order.contains(who) {
                        order.push(who.clone());
                    }
                    *counts.entry((who.clone(), at)).or_insert(0) += 1;
                }
            }
        }
        (order, counts)
    }
}

/// The text with its scenes in another order (their blocks moved whole; what comes before the first kept first).
pub fn reordered(text: &str, from: usize, to: usize) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let starts: Vec<usize> = lines.iter().enumerate().filter(|(_, line)| line.trim_start().starts_with("scene:")).map(|(at, _)| at).collect();
    if from >= starts.len() || to >= starts.len() || from == to {
        return text.to_string();
    }
    let head = lines[..starts[0]].join("\n");
    let mut blocks: Vec<String> =
        starts.iter().enumerate().map(|(at, start)| lines[*start..starts.get(at + 1).copied().unwrap_or(lines.len())].join("\n")).collect();
    // (Each block ends with a blank line, the last too, so they stay apart wherever they go.)
    for block in &mut blocks {
        while block.ends_with('\n') {
            block.pop();
        }
    }
    let moved = blocks.remove(from);
    blocks.insert(to, moved);
    let body = blocks.join("\n\n");
    if head.trim().is_empty() { format!("{body}\n") } else { format!("{}\n\n{body}\n", head.trim_end()) }
}

/// The folder of the characters' sheets.
fn sheets_folder(library: &assets::project::Project) -> PathBuf {
    library.kind("characters")
}

/// A character's sheet (its species, its nature, how it speaks…), as written.
pub fn sheet(library: &assets::project::Project, name: &str) -> String {
    std::fs::read_to_string(sheets_folder(library).join(format!("{}.txt", film::words::plain(name)))).unwrap_or_default()
}

fn write_sheet(library: &assets::project::Project, name: &str, said: &str) {
    _ = std::fs::create_dir_all(sheets_folder(library));
    _ = std::fs::write(sheets_folder(library).join(format!("{}.txt", film::words::plain(name))), said);
}

/// The screenplay as a page (HTML: printed to PDF by any browser).
pub fn screenplay_page(story: &Story, name_of: &dyn Fn(&str) -> String, place_of: &dyn Fn(&str) -> String) -> String {
    let escape = |text: &str| text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let mut page = String::from(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Screenplay</title><style>\
         body{font-family:'Courier New',monospace;max-width:42em;margin:3em auto;line-height:1.5;color:#111}\
         h1{text-align:center;text-transform:uppercase;letter-spacing:.1em}\
         h2{text-transform:uppercase;margin-top:2.5em;border-bottom:1px solid #999}\
         .cast{color:#555;font-size:.9em}.who{margin:1.2em 0 0 14em;text-transform:uppercase;font-weight:bold}\
         .face{font-weight:normal;text-transform:none;color:#555}.said{margin:0 8em 0 10em}\
         .done{font-style:italic;color:#444;margin:.8em 0}.moment{font-weight:bold;margin-top:1.5em}\
         .note{color:#777;font-size:.9em}.choice{margin:1em 0 1em 10em}</style></head><body>",
    );
    page.push_str(&format!("<h1>{}</h1>", escape(if story.title.is_empty() { "Untitled" } else { &story.title })));
    for (at, scene) in story.scenes.iter().enumerate() {
        page.push_str(&format!("<h2>Scene {} — {}</h2>", at + 1, escape(&place_of(&scene.place))));
        if !scene.cast.is_empty() {
            page.push_str(&format!("<p class=\"cast\">With {}</p>", escape(&scene.cast.iter().map(|who| name_of(who)).collect::<Vec<_>>().join(", "))));
        }
        if !scene.summary.is_empty() {
            page.push_str(&format!("<p class=\"note\">{}</p>", escape(&scene.summary)));
        }
        for (_, beat) in &scene.beats {
            match beat {
                Beat::Said { who, face, text } => {
                    let face = face.as_ref().map_or(String::new(), |face| format!(" <span class=\"face\">({})</span>", escape(face)));
                    page.push_str(&format!("<p class=\"who\">{}{face}</p><p class=\"said\">{}</p>", escape(&name_of(who)), escape(text)));
                }
                Beat::Done(text) => page.push_str(&format!("<p class=\"done\">{}</p>", escape(text))),
                Beat::Moment(name) => page.push_str(&format!("<p class=\"moment\">— {} —</p>", escape(name))),
                Beat::Note(note) => page.push_str(&format!("<p class=\"note\"># {}</p>", escape(note))),
                Beat::Choice(options) => page.push_str(&format!("<p class=\"choice\">Choice: {}</p>", escape(&options.join(" / ")))),
                Beat::Way { by, to } => page.push_str(&format!("<p class=\"done\">By {}, to {}.</p>", escape(by), escape(&place_of(to)))),
            }
        }
    }
    page.push_str("</body></html>");
    page
}

/// What a story view asks of the Script: go to a line (in the text), the text changed, a page written.
pub enum Asked {
    Line(usize),
    Text(String),
    Said(String),
}

/// What a story view is drawn with, from the Script.
pub struct Means<'a> {
    /// What the Workshop works with (`session`).
    pub session: &'a workshop_base::session::Session,
    pub text: &'a str,
    pub path: &'a Path,
    pub timing: &'a BTreeMap<usize, (f64, f64)>,
    pub examples: &'a mut workshop_base::example::Examples,
    pub portrait: &'a mut dyn FnMut(&egui::Context, &str, &str) -> Option<egui::TextureHandle>,
    pub name_of: &'a dyn Fn(&str) -> String,
    pub place_of: &'a dyn Fn(&str) -> String,
    pub dragged: &'a mut Option<usize>,
    pub sheets: &'a mut HashMap<String, String>,
}

/// A story view drawn: what it asks.
pub fn shown(ui: &mut egui::Ui, view: View, means: &mut Means) -> Option<Asked> {
    let story = Story::of(means.text);
    if story.scenes.is_empty() {
        ui.add_space(12.0);
        ui.label(theme::weak("No scene yet: \"scene: place who…\" in the text."));
        return None;
    }
    match view {
        View::Screenplay => screenplay(ui, &story, means),
        View::Board => board(ui, &story, means),
        View::Characters => characters(ui, &story, means),
        View::Graph => graph(ui, &story, means),
        View::Text => None,
    }
}

/// How long a range of lines lasts as it plays (s).
fn lasts(timing: &BTreeMap<usize, (f64, f64)>, lines: std::ops::RangeInclusive<usize>) -> Option<f64> {
    let played: Vec<(f64, f64)> = timing.range(lines).map(|(_, played)| *played).collect();
    let begins = played.iter().map(|(start, _)| *start).reduce(f64::min)?;
    let ends = played.iter().map(|(start, length)| start + length).reduce(f64::max)?;
    Some(ends - begins)
}

fn clock(seconds: f64) -> String {
    format!("{}:{:04.1}", (seconds / 60.0) as u32, seconds % 60.0)
}

/// The screenplay: each scene a heading, each line said by its portrait and name, what is done in italics; a click
/// goes to the line in the text; written out as a page.
fn screenplay(ui: &mut egui::Ui, story: &Story, means: &mut Means) -> Option<Asked> {
    let mut asked = None;
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(theme::text("The film read as a screenplay: a click on a line shows it in the text", theme::SMALL, theme::FAINT));
        if ui.button("Export the Page").on_hover_text("as HTML, to open in a browser and print (or as PDF)").clicked() {
            let page = screenplay_page(story, means.name_of, means.place_of);
            let stem = means.path.file_stem().map_or_else(|| "film".to_string(), |stem| stem.to_string_lossy().into_owned());
            let folder = means.session.root().join("target").join("workshop").join("screenplays");
            let file = folder.join(format!("{stem}.html"));
            asked = Some(Asked::Said(match std::fs::create_dir_all(&folder).and_then(|()| std::fs::write(&file, page)) {
                Ok(()) => format!("screenplay written: {}", file.display()),
                Err(why) => why.to_string(),
            }));
        }
    });
    egui::ScrollArea::vertical().id_salt("screenplay").auto_shrink(false).show(ui, |ui| {
        ui.set_max_width(760.0);
        if !story.title.is_empty() {
            ui.vertical_centered(|ui| ui.label(RichText::new(story.title.to_uppercase()).size(theme::TITLE + 6.0).strong().color(theme::STRONG)));
        }
        for (at, scene) in story.scenes.iter().enumerate() {
            ui.add_space(16.0);
            let heading = format!("SCENE {} — {}", at + 1, (means.place_of)(&scene.place).to_uppercase());
            if ui.add(egui::Label::new(RichText::new(heading).size(theme::TITLE).strong().color(theme::FILM)).sense(Sense::click())).clicked() {
                asked = Some(Asked::Line(scene.line));
            }
            let mut said_by = format!("with {}", scene.cast.iter().map(|who| (means.name_of)(who)).collect::<Vec<_>>().join(", "));
            if let Some(length) = lasts(means.timing, scene.line..=scene.end) {
                said_by.push_str(&format!(" · {}", clock(length)));
            }
            ui.label(theme::text(said_by, theme::SMALL, theme::WEAK));
            if !scene.summary.is_empty() {
                ui.label(RichText::new(&scene.summary).italics().color(theme::FAINT));
            }
            ui.separator();
            for (line, beat) in &scene.beats {
                let response = match beat {
                    Beat::Said { who, face, text } => {
                        ui.horizontal_top(|ui| {
                            let species = crate::species_of(means.session, means.text, who);
                            let (rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                            if let Some(picture) =
                                (who != "narrateur").then(|| (means.portrait)(ui.ctx(), &species, face.as_deref().unwrap_or("normal"))).flatten()
                            {
                                ui.painter().image(picture.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                            }
                            ui.vertical(|ui| {
                                let mut head = RichText::new((means.name_of)(who).to_uppercase()).strong().color(theme::STRONG).size(theme::BODY);
                                if face.is_some() {
                                    head = RichText::new(format!("{} ({})", (means.name_of)(who).to_uppercase(), face.as_deref().unwrap_or_default()))
                                        .strong()
                                        .color(theme::STRONG)
                                        .size(theme::BODY);
                                }
                                ui.label(head);
                                ui.add(egui::Label::new(RichText::new(text).size(theme::BODY + 1.0).color(theme::TEXT)).wrap());
                            });
                        })
                        .response
                    }
                    Beat::Done(text) => ui.label(RichText::new(text).italics().color(theme::WEAK).size(theme::SMALL + 0.5)),
                    Beat::Moment(name) => ui.label(RichText::new(format!("— {name} —")).strong().color(theme::FILM)),
                    Beat::Note(note) => ui.label(RichText::new(format!("# {note}")).color(Color32::from_rgb(0x6A, 0x99, 0x55)).size(theme::SMALL)),
                    Beat::Choice(options) => ui.label(RichText::new(format!("Choice: {}", options.join("  /  "))).strong().color(theme::HOME)),
                    Beat::Way { by, to } => ui.label(RichText::new(format!("By {by}, to {}.", (means.place_of)(to))).italics().color(theme::DECOR)),
                };
                if ui.interact(response.rect, egui::Id::new(("screenplay line", line)), Sense::click()).clicked() {
                    asked = Some(Asked::Line(*line));
                }
            }
        }
    });
    asked
}

/// The corkboard: a card each scene (its number, its place seen, who is there, its summary, how much is said, how
/// long); dragged onto another, the scene goes there in the text; a click goes to it.
fn board(ui: &mut egui::Ui, story: &Story, means: &mut Means) -> Option<Asked> {
    let mut asked = None;
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(theme::text("A card per scene: drag one onto another to change their order (the text follows); a click goes there; \"# summary\" at the top of a scene: its summary", theme::SMALL, theme::FAINT));
    });
    let mut dropped: Option<(usize, usize)> = None;
    egui::ScrollArea::vertical().id_salt("corkboard").auto_shrink(false).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
            for (at, scene) in story.scenes.iter().enumerate() {
                let (rect, response) = ui.allocate_exact_size(vec2(230.0, 250.0), Sense::click_and_drag());
                let lifted = *means.dragged == Some(at);
                let pointed = response.hovered() || (means.dragged.is_some() && ui.rect_contains_pointer(rect));
                ui.painter().rect_filled(
                    rect,
                    6.0,
                    if lifted {
                        theme::GAP
                    } else if pointed {
                        theme::HOVERED
                    } else {
                        theme::PANEL
                    },
                );
                ui.painter().rect_stroke(
                    rect,
                    6.0,
                    Stroke::new(if pointed { 2.0_f32 } else { 1.0 }, if pointed { theme::FILM } else { theme::GAP }),
                    StrokeKind::Inside,
                );
                let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(8.0)).layout(egui::Layout::top_down(egui::Align::Min)));
                inside.label(RichText::new(format!("{}. {}", at + 1, (means.place_of)(&scene.place))).strong().color(theme::STRONG));
                means.examples.still(&mut inside, &format!("scene: {}\nattends 1.5s\n", scene.place), 214.0);
                inside.label(theme::text(scene.cast.iter().map(|who| (means.name_of)(who)).collect::<Vec<_>>().join(", "), theme::SMALL, theme::WEAK));
                let said = scene.beats.iter().filter(|(_, beat)| matches!(beat, Beat::Said { .. })).count();
                let mut counts = format!("{said} line(s) said · {} line(s)", scene.end - scene.line + 1);
                if let Some(length) = lasts(means.timing, scene.line..=scene.end) {
                    counts.push_str(&format!(" · {}", clock(length)));
                }
                inside.label(theme::text(counts, theme::SMALL - 1.0, theme::FAINT));
                if !scene.summary.is_empty() {
                    inside.label(RichText::new(&scene.summary).italics().size(theme::SMALL).color(theme::TEXT));
                }
                if response.drag_started() {
                    *means.dragged = Some(at);
                }
                if response.clicked() {
                    asked = Some(Asked::Line(scene.line));
                }
                if let Some(from) = *means.dragged
                    && ui.input(|input| input.pointer.any_released())
                    && ui.rect_contains_pointer(rect)
                    && from != at
                {
                    dropped = Some((from, at));
                }
            }
        });
    });
    if ui.input(|input| input.pointer.any_released()) {
        *means.dragged = None;
    }
    if let Some((from, to)) = dropped {
        return Some(Asked::Text(reordered(means.text, from, to)));
    }
    asked
}

/// The characters: who speaks in which scene and how much (a cell each), how much each says in all; each one's
/// sheet (its species, its nature, how it speaks), written beside the films and shown when pointed at.
fn characters(ui: &mut egui::Ui, story: &Story, means: &mut Means) -> Option<Asked> {
    let mut asked = None;
    let (order, counts) = story.speakers();
    egui::ScrollArea::both().id_salt("characters").auto_shrink(false).show(ui, |ui| {
        ui.label(theme::text(
            "Who speaks where: how many lines each says in each scene (·: there, not speaking); a click goes to the scene",
            theme::SMALL,
            theme::FAINT,
        ));
        egui::Grid::new("characters × scenes").striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
            ui.label("");
            for (at, scene) in story.scenes.iter().enumerate() {
                if ui
                    .add(egui::Label::new(theme::text(format!("{}", at + 1), theme::SMALL, theme::WEAK)).sense(Sense::click()))
                    .on_hover_text((means.place_of)(&scene.place))
                    .clicked()
                {
                    asked = Some(Asked::Line(scene.line));
                }
            }
            ui.label(theme::text("in all", theme::SMALL, theme::WEAK));
            ui.label(theme::text("words", theme::SMALL, theme::WEAK));
            ui.end_row();
            for who in &order {
                ui.horizontal(|ui| {
                    let species = crate::species_of(means.session, means.text, who);
                    if let Some(picture) = (who != "narrateur").then(|| (means.portrait)(ui.ctx(), &species, "normal")).flatten() {
                        ui.add(egui::Image::new(&picture).fit_to_exact_size(vec2(22.0, 22.0)));
                    }
                    ui.label(RichText::new((means.name_of)(who)).strong());
                });
                let mut total = 0;
                for (at, scene) in story.scenes.iter().enumerate() {
                    let count = counts.get(&(who.clone(), at)).copied().unwrap_or(0);
                    total += count;
                    let there = scene.cast.contains(who);
                    let said = if count > 0 {
                        count.to_string()
                    } else if there {
                        "·".to_string()
                    } else {
                        String::new()
                    };
                    let shade = if count > 0 { Color32::from_rgb(0xE0, 0xA9, 0x3A).gamma_multiply((0.35 + count as f32 / 8.0).min(1.0)) } else { theme::FAINT };
                    ui.label(RichText::new(said).strong().color(shade));
                }
                let words: usize = story
                    .scenes
                    .iter()
                    .flat_map(|scene| &scene.beats)
                    .filter_map(|(_, beat)| match beat {
                        Beat::Said { who: speaker, text, .. } if speaker == who => Some(text.split_whitespace().count()),
                        _ => None,
                    })
                    .sum();
                ui.label(RichText::new(total.to_string()).strong());
                ui.label(theme::text(words.to_string(), theme::SMALL, theme::WEAK));
                ui.end_row();
            }
        });
        ui.add_space(14.0);
        ui.label(RichText::new("Character Sheets").strong());
        ui.label(theme::text(
            "their species, their nature, how they speak, their habits… kept in library/characters/ and shown when one points at them in the text",
            theme::SMALL,
            theme::FAINT,
        ));
        for who in order.iter().filter(|who| *who != "narrateur") {
            egui::CollapsingHeader::new(RichText::new((means.name_of)(who)).strong()).id_salt(("sheet", who)).show(ui, |ui| {
                let library = means.session.library();
                let said = means.sheets.entry(who.clone()).or_insert_with(|| sheet(library, who));
                let response = ui.add(egui::TextEdit::multiline(said).desired_rows(4).desired_width(520.0).hint_text("Nature: …\nSpeaks: …\nWants: …"));
                if response.lost_focus() || (response.changed() && !response.has_focus()) {
                    write_sheet(library, who, said);
                }
                if response.changed() {
                    write_sheet(library, who, said);
                }
            });
        }
    });
    asked
}

/// The graph: the scenes as boxes, one after another (their choices inside), arrows to the next, and the ways from
/// place to place (`passage`) as arrows to the scene of that place; a click on a box goes to it.
fn graph(ui: &mut egui::Ui, story: &Story, means: &mut Means) -> Option<Asked> {
    let mut asked = None;
    ui.label(theme::text(
        "The scenes in order, their choices, and the passages from one place to another (in blue); a click goes to the scene",
        theme::SMALL,
        theme::FAINT,
    ));
    let (box_wide, box_high, gap) = (190.0, 120.0, 46.0);
    let per_row = ((ui.available_width() + gap) / (box_wide + gap)).floor().max(1.0) as usize;
    let rows = story.scenes.len().div_ceil(per_row);
    egui::ScrollArea::both().id_salt("graph").auto_shrink(false).show(ui, |ui| {
        let (area, _) = ui.allocate_exact_size(vec2(per_row as f32 * (box_wide + gap), rows as f32 * (box_high + gap) + 20.0), Sense::hover());
        let painter = ui.painter_at(area);
        let place_of_box = |at: usize| {
            Rect::from_min_size(
                area.min + vec2((at % per_row) as f32 * (box_wide + gap) + 10.0, (at / per_row) as f32 * (box_high + gap) + 10.0),
                vec2(box_wide, box_high),
            )
        };
        let arrow = |from: egui::Pos2, to: egui::Pos2, shade: Color32| {
            painter.line_segment([from, to], Stroke::new(2.0_f32, shade));
            let way = (to - from).normalized();
            let side = vec2(-way.y, way.x);
            painter.add(egui::Shape::convex_polygon(vec![to, to - way * 9.0 + side * 5.0, to - way * 9.0 - side * 5.0], shade, Stroke::NONE));
        };
        // (One after another.)
        for at in 1..story.scenes.len() {
            let (from, to) = (place_of_box(at - 1), place_of_box(at));
            let (start, end) = if to.min.y > from.min.y + 1.0 { (from.center_bottom(), to.center_top()) } else { (from.right_center(), to.left_center()) };
            arrow(start, end, theme::FAINT);
        }
        // (The ways: to the first scene of their place.)
        for (at, scene) in story.scenes.iter().enumerate() {
            for (_, beat) in &scene.beats {
                if let Beat::Way { by, to } = beat
                    && let Some(target) = story.scenes.iter().position(|other| other.place == *to)
                {
                    let (from, into) = (place_of_box(at), place_of_box(target));
                    let lift: f32 = if target < at { -30.0 } else { 30.0 };
                    let points = [
                        from.center_top(),
                        from.center_top() + vec2(0.0, -14.0 + lift.min(0.0)),
                        into.center_top() + vec2(0.0, -14.0 + lift.min(0.0)),
                        into.center_top(),
                    ];
                    painter.add(egui::Shape::line(points.to_vec(), Stroke::new(2.0_f32, theme::DECOR)));
                    painter.text(
                        points[1] + vec2(4.0, -2.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("via {by}"),
                        egui::FontId::proportional(theme::SMALL - 1.5),
                        theme::DECOR,
                    );
                }
            }
        }
        for (at, scene) in story.scenes.iter().enumerate() {
            let shape = place_of_box(at);
            let response = ui.interact(shape, egui::Id::new(("graph scene", at)), Sense::click());
            painter.rect_filled(shape, 6.0, if response.hovered() { theme::HOVERED } else { theme::PANEL });
            painter.rect_stroke(shape, 6.0, Stroke::new(1.0_f32, theme::FILM), StrokeKind::Inside);
            painter.text(
                shape.min + vec2(8.0, 6.0),
                egui::Align2::LEFT_TOP,
                format!("{}. {}", at + 1, (means.place_of)(&scene.place)),
                egui::FontId::proportional(theme::SMALL + 1.0),
                theme::STRONG,
            );
            let mut y = shape.min.y + 26.0;
            let said = scene.beats.iter().filter(|(_, beat)| matches!(beat, Beat::Said { .. })).count();
            painter.text(
                pos2(shape.min.x + 8.0, y),
                egui::Align2::LEFT_TOP,
                format!("{said} line(s) said"),
                egui::FontId::proportional(theme::SMALL - 1.0),
                theme::WEAK,
            );
            y += 16.0;
            for (_, beat) in &scene.beats {
                let said = match beat {
                    Beat::Choice(options) => format!("choice: {}", options.join(" / ")),
                    Beat::Moment(name) => format!("· {name}"),
                    _ => continue,
                };
                if y > shape.max.y - 14.0 {
                    break;
                }
                let clip = painter.with_clip_rect(shape.shrink(4.0));
                clip.text(
                    pos2(shape.min.x + 8.0, y),
                    egui::Align2::LEFT_TOP,
                    said,
                    egui::FontId::proportional(theme::SMALL - 1.0),
                    if matches!(beat, Beat::Choice(_)) { theme::HOME } else { theme::TEXT },
                );
                y += 15.0;
            }
            if response.clicked() {
                asked = Some(Asked::Line(scene.line));
            }
        }
    });
    asked
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILM: &str = "title: Trial\n\nscene: beach pikachu piplup\n# They meet again.\npikachu (joyous): Hi!\nwalk piplup to right\nchoice\n  option Yes\n  option No\nend\npassage east to port left\n\nscene: port pikachu\npikachu: The port.\n";

    #[test]
    fn a_film_is_read_as_a_story_and_its_scenes_moved() {
        let story = Story::of(FILM);
        assert_eq!(story.title, "Trial");
        assert_eq!(story.scenes.len(), 2);
        let first = &story.scenes[0];
        assert_eq!(
            (first.place.as_str(), first.cast.clone(), first.summary.as_str()),
            ("beach", vec!["pikachu".to_string(), "piplup".to_string()], "They meet again.")
        );
        assert!(first.beats.iter().any(|(_, beat)| *beat == Beat::Said { who: "pikachu".into(), face: Some("joyous".into()), text: "Hi!".into() }));
        assert!(first.beats.iter().any(|(_, beat)| *beat == Beat::Choice(vec!["Yes".into(), "No".into()])));
        assert!(first.beats.iter().any(|(_, beat)| *beat == Beat::Way { by: "east".into(), to: "port".into() }));
        let (order, counts) = story.speakers();
        assert_eq!(order, vec!["pikachu", "piplup"]);
        assert_eq!(counts.get(&("pikachu".to_string(), 1)), Some(&1));
        let moved = reordered(FILM, 1, 0);
        let again = Story::of(&moved);
        assert_eq!(again.scenes.iter().map(|scene| scene.place.as_str()).collect::<Vec<_>>(), vec!["port", "beach"]);
        assert!(moved.starts_with("title: Trial"), "what comes before the first scene stays at the top");
        assert_eq!(again.scenes[1].beats.len(), first.beats.len(), "each scene whole");
    }
}
