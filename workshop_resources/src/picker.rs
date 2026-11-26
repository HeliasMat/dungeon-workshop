//! The Film space's Assets panel: the Resources' shelves made small, beside
//! the film being written. Tabs: the Pokémon (the film's cast first, then the
//! catalog of `catalog.rs`, each species' forms under it, SpriteCollab's not
//! imported imported when first used), the faces and the
//! gestures of the Pokémon chosen (the Scene's chosen one, followed), the
//! music (its title, how long), the sounds, the things, the places, the
//! moves' effects. Each card is dragged onto the Scene or the Script
//! (`workshop_base::asset::Asset`), or double-clicked: written at the
//! Script's cursor. Music and sounds play on hover (a click keeps a song
//! playing; Stop hushes it), gestures and effects play on hover.

use std::collections::HashSet;
use std::time::Instant;

use eframe::egui::{self, Color32, FontId, Rect, RichText, Sense, Stroke, vec2};
use workshop_base::ask::Ask;
use workshop_base::asset::Asset;
use workshop_base::theme;
use workshop_decor::Decors;
use workshop_picture::fitted;
use workshop_script::Scripts;

use super::pictures::{animations_of, body_sheet, face_name, faces_of, read_png};
use super::{Item, READ_EACH_FRAME, Resources, Sheet, Shelf, pokemon_cards};

/// How long the pointer rests on a song or a sound before it is heard.
pub(super) const HEARD_AFTER: f64 = 0.35;

/// The panel's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Pokemon,
    Faces,
    Gestures,
    Music,
    Sounds,
    Things,
    Places,
    Effects,
}

impl Tab {
    pub const ALL: [Tab; 8] = [Tab::Pokemon, Tab::Faces, Tab::Gestures, Tab::Music, Tab::Sounds, Tab::Things, Tab::Places, Tab::Effects];

    pub fn name(self) -> &'static str {
        match self {
            Tab::Pokemon => "Pokémon",
            Tab::Faces => "Faces",
            Tab::Gestures => "Gestures",
            Tab::Music => "Music",
            Tab::Sounds => "Sounds",
            Tab::Things => "Things",
            Tab::Places => "Places",
            Tab::Effects => "Effects",
        }
    }

    /// Its word, as `WORKSHOP_PICKER` says it (`gestures`, or `animations`).
    pub fn of(word: &str) -> Option<Tab> {
        let word = word.trim().to_lowercase();
        match word.as_str() {
            "animations" | "poses" => Some(Tab::Gestures),
            "items" | "objects" => Some(Tab::Things),
            "moves" => Some(Tab::Effects),
            _ => Tab::ALL.into_iter().find(|tab| film::words::plain(tab.name()) == word || film::words::plain(tab.name()).trim_end_matches('s') == word),
        }
    }

    fn about(self) -> &'static str {
        match self {
            Tab::Pokemon => "drag onto the Scene: in the scene's cast, put there · onto the Script: its name · a click: its faces and gestures",
            Tab::Faces => "onto a spoken line: said with that face · elsewhere: a new line to say",
            Tab::Gestures => "who does it: `pikachu sit` · on the Scene, onto someone: they do it · hover: it plays",
            Tab::Music => "`music word` · hover: heard · a click: heard on, Stop hushes it",
            Tab::Sounds => "`sound word` · hover: heard",
            Tab::Things => "onto someone: held · onto the ground: put there · onto the Script: held or put",
            Tab::Places => "a new scene there, with the cast of the one before",
            Tab::Effects => "`who use Move on other` · onto someone on the Scene: theirs · hover: it plays",
        }
    }
}

/// The panel's own state, kept by the Resources.
pub(super) struct Picker {
    tab: Tab,
    sought: String,
    /// The Pokémon whose faces and gestures are offered; the Scene's chosen one last followed.
    who: Option<String>,
    followed: Option<String>,
    /// What the pointer rests on, since when; what a hover made heard (hushed when it leaves).
    pub(super) resting: Option<(String, Instant)>,
    pub(super) hover_heard: Option<String>,
}

impl Picker {
    pub(super) fn new(asked: Option<&str>) -> Picker {
        // (`WORKSHOP_PICKER=gestures:pikachu:sit`: that tab, that Pokémon, that sought.)
        let mut parts = asked.unwrap_or_default().splitn(3, ':');
        let tab = parts.next().and_then(Tab::of).unwrap_or(Tab::Pokemon);
        let who = parts.next().filter(|who| !who.is_empty()).map(str::to_string);
        let sought = parts.next().unwrap_or_default().to_string();
        Picker { tab, sought, followed: None, who, resting: None, hover_heard: None }
    }
}

/// What a card of the panel is: its key (pictures, hovers), the asset it hands, what it shows.
struct Card {
    key: String,
    asset: Asset,
    label: String,
    about: String,
    /// The shelf's thing it is, if it is one (its picture the Resources' own).
    item: Option<Item>,
}

/// The Assets panel: the tabs, the search, the cards; `chosen`: who is chosen on the Scene, `cast`: the film's.
pub fn picker(ui: &mut egui::Ui, resources: &mut Resources, decors: &Decors, scripts: &mut Scripts, chosen: Option<&str>, cast: &[String]) {
    resources.read_now = 0;
    resources.work_done();
    resources.thumbs.take(ui.ctx());
    follow(resources, chosen, cast);
    head(ui, resources);
    ui.add_space(2.0);
    ui.label(theme::text("drag onto the Scene or the Script · double-click: at the cursor", theme::SMALL, theme::FAINT))
        .on_hover_text(resources.picker.tab.about());
    ui.add_space(2.0);
    let mut hovered_audio = false;
    let mut hovered_card = false;
    let mut written: Option<Asset> = None;
    if resources.picker.tab == Tab::Pokemon {
        hovered_card = pokemon_tab(ui, resources, cast, &mut written);
    } else {
        egui::ScrollArea::vertical().id_salt("assets picker").auto_shrink([false, false]).show(ui, |ui| {
            let tab = resources.picker.tab;
            match tab {
                Tab::Pokemon => {}
                Tab::Music | Tab::Sounds => audio_rows(ui, resources, decors, tab, &mut hovered_audio, &mut written),
                _ => {
                    for (title, cards) in sections(resources, decors) {
                        if !title.is_empty() {
                            ui.add_space(4.0);
                            ui.label(theme::text(title, theme::SMALL, theme::WEAK));
                        }
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                            for card in &cards {
                                hovered_card |= card_shown(ui, resources, decors, card, &mut written);
                            }
                        });
                        if cards.is_empty() {
                            ui.label(theme::weak(match tab {
                                Tab::Faces | Tab::Gestures => "choose a Pokémon (its tab, or someone on the Scene)",
                                _ => "nothing by that name",
                            }));
                        }
                    }
                }
            }
        });
    }
    // (A song heard on a hover: hushed when the pointer leaves it.)
    if !hovered_audio && !hovered_card {
        resources.picker.resting = None;
    }
    if !hovered_audio && resources.picker.hover_heard.take().is_some_and(|key| resources.heard.as_deref() == Some(key.as_str())) {
        hush(resources);
    }
    if let Some(asset) = written {
        if let Asset::Pokemon { word } = &asset {
            resources.import_if_missing(word);
        }
        let said = match scripts.open.as_mut() {
            Some(open) => open.put_asset(&asset, None, chosen, None),
            None => "no film open: open one first".to_string(),
        };
        resources.session.ask(Ask::Say(said));
    }
}

/// The Scene's chosen one followed: their faces and gestures offered; none chosen yet, the film's first.
fn follow(resources: &mut Resources, chosen: Option<&str>, cast: &[String]) {
    let picker = &mut resources.picker;
    if picker.followed.as_deref() != chosen {
        picker.followed = chosen.map(str::to_string);
        if let Some(chosen) = chosen.filter(|chosen| resources.session.tables().monsters.contains_key(*chosen)) {
            picker.who = Some(chosen.to_string());
        }
    }
    if picker.who.is_none() {
        picker.who = cast.iter().find(|one| resources.session.tables().monsters.contains_key(*one)).cloned();
    }
}

/// The tabs, the search, Stop.
fn head(ui: &mut egui::Ui, resources: &mut Resources) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for tab in Tab::ALL {
            let here = resources.picker.tab == tab;
            let word = RichText::new(tab.name()).size(theme::SMALL).color(if here { theme::GROUND } else { theme::TEXT });
            if ui.add(egui::Button::new(word).fill(if here { theme::FILM } else { theme::FIELD }).corner_radius(8)).on_hover_text(tab.about()).clicked() {
                resources.picker.tab = tab;
            }
        }
    });
    ui.horizontal(|ui| {
        let hint = match resources.picker.tab {
            Tab::Pokemon => "Search: name or number",
            Tab::Faces | Tab::Gestures => "Search this Pokémon's",
            _ => "Search",
        };
        let stop_room = if resources.heard.is_some() { 64.0 } else { 0.0 };
        ui.add(egui::TextEdit::singleline(&mut resources.picker.sought).hint_text(hint).desired_width((ui.available_width() - stop_room).max(60.0)));
        if resources.heard.is_some() && ui.button("■ Stop").on_hover_text("what plays, hushed").clicked() {
            hush(resources);
        }
    });
    if matches!(resources.picker.tab, Tab::Faces | Tab::Gestures) {
        ui.horizontal_wrapped(|ui| {
            let who = resources.picker.who.clone().unwrap_or_else(|| "—".to_string());
            ui.label(theme::text(format!("{} of", resources.picker.tab.name()), theme::SMALL, theme::WEAK));
            ui.label(theme::text(shown_name(resources, &who), theme::BODY, theme::STRONG));
            ui.label(theme::text("(a click on a Pokémon, or someone on the Scene)", theme::SMALL, theme::FAINT));
        });
    }
}

/// A Pokémon's name as the game says it.
fn shown_name(resources: &Resources, word: &str) -> String {
    resources.session.tables().monsters.get(word).map(|(_, name)| name.clone()).filter(|name| !name.is_empty()).unwrap_or_else(|| word.to_string())
}

/// Whether an item answers what is sought.
fn answers(item: &Item, sought: &str) -> bool {
    sought.is_empty() || [&item.word, &item.label, &item.about].iter().any(|said| film::words::plain(said).contains(sought))
}

/// The asset a thing of a shelf hands.
fn asset_of(item: &Item) -> Option<Asset> {
    Some(match item.shelf {
        Shelf::Pokemon => Asset::Pokemon { word: item.word.clone() },
        Shelf::Songs => Asset::Music { word: item.word.clone() },
        Shelf::Sounds => Asset::Sound { word: item.word.clone() },
        Shelf::Items => Asset::Thing { word: item.label.clone(), held: true },
        Shelf::Drawn => Asset::Thing { word: item.word.clone(), held: true },
        Shelf::Props => Asset::Thing { word: item.word.clone(), held: false },
        Shelf::Backdrops => Asset::Place { word: item.word.clone() },
        Shelf::Attacks => Asset::Effect { name: item.label.clone() },
        _ => return None,
    })
}

/// A card of a shelf's thing.
fn card_of(item: &Item) -> Option<Card> {
    Some(Card { key: item.key(), asset: asset_of(item)?, label: item.label.clone(), about: item.about.clone(), item: Some(item.clone()) })
}

/// The cards of the tab, by section (its title), as sought.
fn sections(resources: &mut Resources, decors: &Decors) -> Vec<(String, Vec<Card>)> {
    let sought = film::words::plain(resources.picker.sought.trim());
    let of = |resources: &mut Resources, shelf: Shelf| -> Vec<Card> {
        resources.list(shelf, decors).iter().filter(|item| answers(item, &sought)).filter_map(card_of).collect()
    };
    match resources.picker.tab {
        Tab::Pokemon | Tab::Music | Tab::Sounds => Vec::new(),
        Tab::Faces => vec![(String::new(), faces(resources, &sought))],
        Tab::Gestures => vec![(String::new(), gestures(resources, &sought))],
        Tab::Things => vec![
            ("ITEMS · HELD".to_string(), of(resources, Shelf::Items)),
            ("THE GAME'S OBJECTS · PUT DOWN".to_string(), of(resources, Shelf::Props)),
            ("DRAWN · HELD".to_string(), of(resources, Shelf::Drawn)),
        ],
        Tab::Places => {
            let decors_own: Vec<Card> = own_decors(resources).into_iter().filter(|card| sought.is_empty() || card.key.contains(&sought)).collect();
            vec![("YOUR DECORS".to_string(), decors_own), ("THE GAME'S PLACES".to_string(), of(resources, Shelf::Backdrops))]
        }
        Tab::Effects => vec![(String::new(), of(resources, Shelf::Attacks))],
    }
}

/// The Pokémon tab: the film's cast first, then the catalog (as sought: name, number, type, generation), each
/// species' forms under it; only the rows in sight drawn. Whether the pointer is on a card.
fn pokemon_tab(ui: &mut egui::Ui, resources: &mut Resources, cast: &[String], written: &mut Option<Asset>) -> bool {
    let catalog = resources.catalog();
    let sought = resources.picker.sought.trim().to_string();
    let size = vec2(108.0, 112.0);
    let mut hovered = false;
    let mut did = pokemon_cards::Did::default();
    let theirs: Vec<(usize, usize)> = cast.iter().filter_map(|one| catalog.find(one)).collect();
    if !theirs.is_empty() && sought.is_empty() {
        ui.label(theme::text("IN THIS FILM", theme::SMALL, theme::WEAK));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for (species, entry) in theirs {
                let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
                let one = &catalog.species[species];
                let chosen = resources.picker.who.as_deref() == Some(one.entries[entry].word.as_str());
                pokemon_cards::card(ui, resources, rect, one, &one.entries[entry], entry, &response, chosen, false);
                hovered |= response.hovered();
                if response.double_clicked() {
                    did.twice = Some((species, entry));
                } else if response.clicked() {
                    did.clicked = Some((species, entry));
                }
                if response.drag_started() {
                    did.dragged = Some((species, entry));
                }
            }
        });
        ui.add_space(4.0);
    }
    let found = (!sought.is_empty()).then(|| catalog.search(&sought));
    let shown = pokemon_cards::shown(&catalog, found.as_deref(), |species, by_itself| resources.species_open(species, by_itself));
    let columns = ((ui.available_width() - 6.0) / (size.x + 8.0)).floor().max(1.0) as usize;
    let rows = pokemon_cards::rows(&shown, columns);
    let count: usize = shown.len();
    ui.label(theme::text(
        if sought.is_empty() { format!("ALL · {count} species, {} with their forms", catalog.len()) } else { format!("{count} species found") },
        theme::SMALL,
        theme::WEAK,
    ));
    if rows.is_empty() {
        ui.label(theme::weak("nothing by that name, number, type or generation (gen 8, fire…)"));
    }
    let who = resources.picker.who.clone();
    let grid = pokemon_cards::grid(ui, resources, "assets pokemon", &catalog, &rows, size, who.as_deref(), None);
    hovered |= grid.hovered;
    did.clicked = did.clicked.or(grid.clicked);
    did.twice = did.twice.or(grid.twice);
    did.dragged = did.dragged.or(grid.dragged);
    if let Some(species) = grid.toggled {
        let open = shown.iter().find(|one| one.species == species).is_some_and(|one| one.open);
        resources.toggle_species(&catalog.species[species], open);
    }
    let word_of = |(species, entry): (usize, usize)| catalog.species[species].entries[entry].word.clone();
    if let Some(at) = did.clicked {
        let word = word_of(at);
        if !resources.import_if_missing(&word) {
            resources.picker.who = Some(word);
        }
    }
    if let Some(at) = did.twice {
        *written = Some(Asset::Pokemon { word: word_of(at) });
    }
    if let Some(at) = did.dragged {
        let word = word_of(at);
        resources.import_if_missing(&word);
        egui::DragAndDrop::set_payload(ui.ctx(), Asset::Pokemon { word });
    }
    dragged_label(ui);
    hovered
}

/// What is being dragged, said beside the pointer.
fn dragged_label(ui: &egui::Ui) {
    let Some(asset) = egui::DragAndDrop::payload::<Asset>(ui.ctx()) else { return };
    if !ui.input(|input| input.pointer.any_down()) {
        return;
    }
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    if let Some(at) = ui.input(|input| input.pointer.latest_pos()) {
        egui::Area::new(egui::Id::new("asset dragged")).order(egui::Order::Tooltip).fixed_pos(at + vec2(14.0, 10.0)).interactable(false).show(ui.ctx(), |ui| {
            egui::Frame::new().fill(Color32::from_black_alpha(220)).inner_margin(egui::Margin::symmetric(6, 3)).corner_radius(3).show(ui, |ui| {
                ui.label(theme::text(asset.said(), theme::SMALL, theme::FILM));
            });
        });
    }
}

/// The user's decors, each a place of their own.
fn own_decors(resources: &Resources) -> Vec<Card> {
    let mut words: Vec<String> = std::fs::read_dir(resources.session.library().decors())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            entry.path().file_stem().filter(|_| entry.path().extension().is_some_and(|own| own == "decor")).map(|stem| stem.to_string_lossy().into_owned())
        })
        .collect();
    words.sort();
    words
        .into_iter()
        .map(|word| Card { key: format!("decor:{word}"), asset: Asset::Place { word: word.clone() }, label: word, about: "your decor".to_string(), item: None })
        .collect()
}

/// The game's number of a Pokémon, by its word.
fn number_of(resources: &Resources, word: &str) -> Option<i64> {
    resources.session.tables().monsters.get(word).and_then(|(number, _)| number.parse().ok())
}

/// The faces of the Pokémon chosen, those a film can name.
fn faces(resources: &Resources, sought: &str) -> Vec<Card> {
    let Some(who) = resources.picker.who.clone() else { return Vec::new() };
    let Some(number) = number_of(resources, &who) else { return Vec::new() };
    faces_of(&resources.session, number)
        .into_iter()
        .map(|face| (face, face_name(face)))
        .filter(|(_, name)| name.parse::<u16>().is_err() && (sought.is_empty() || name.contains(sought)))
        .map(|(face, name)| Card {
            key: format!("face:{number}:{face}"),
            asset: Asset::Face { who: who.clone(), face: name.clone() },
            label: name.replace('_', " "),
            about: format!("{who} ({name}): …"),
            item: None,
        })
        .collect()
}

/// The gestures of the Pokémon chosen: each animation its sprite has, by the
/// names the films give it (`sit`, `pose look_up`), else by its own (`anim Swing`).
fn gestures(resources: &mut Resources, sought: &str) -> Vec<Card> {
    let Some(who) = resources.picker.who.clone() else { return Vec::new() };
    let Some(number) = number_of(resources, &who) else { return Vec::new() };
    let own = gesture_words_known();
    let mut cards = Vec::new();
    for (file, anim) in animations_of(&resources.session, number) {
        let names: Vec<&str> = film::POSES.iter().filter(|(_, named, _)| *named == anim).map(|(name, ..)| *name).collect();
        let said: Vec<(String, String)> = if names.is_empty() {
            vec![(anim.clone(), format!("anim {anim}"))]
        } else {
            names.iter().map(|name| (name.to_string(), if own.contains(name) { name.to_string() } else { format!("pose {name}") })).collect()
        };
        for (label, words) in said {
            if !sought.is_empty() && !film::words::plain(&label).contains(sought) && !film::words::plain(&anim).contains(sought) {
                continue;
            }
            cards.push(Card {
                key: format!("body:{number}:{file}"),
                about: format!("{who} {words} · its {anim} animation"),
                asset: Asset::Gesture { who: who.clone(), words },
                label: label.replace('_', " "),
                item: None,
            });
        }
    }
    cards
}

/// The words a gesture is said in alone (`pikachu sit`): those whose every
/// form is who, then words in brackets at most.
fn gesture_words() -> HashSet<&'static str> {
    film::every_word()
        .filter(|(_, _, forms, _)| {
            forms.iter().any(|form| {
                let mut words = form.split_whitespace();
                words.next() == Some("who:pokemon") && words.all(|word| word.starts_with('[') || word.ends_with(']'))
            })
        })
        .map(|(_, word, ..)| word)
        .collect()
}

/// The gesture words, read once.
pub(super) fn gesture_words_known() -> &'static HashSet<&'static str> {
    static KNOWN: std::sync::OnceLock<HashSet<&'static str>> = std::sync::OnceLock::new();
    KNOWN.get_or_init(gesture_words)
}

/// A card: its picture (playing while the pointer is on it), its name;
/// dragged, double-clicked, clicked. Whether the pointer is on it.
fn card_shown(ui: &mut egui::Ui, resources: &mut Resources, decors: &Decors, card: &Card, written: &mut Option<Asset>) -> bool {
    let size = match card.asset {
        Asset::Place { .. } => vec2(116.0, 92.0),
        _ => vec2(70.0, 84.0),
    };
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    if !ui.is_rect_visible(rect) {
        return false;
    }
    let chosen = matches!(&card.asset, Asset::Pokemon { word } if resources.picker.who.as_deref() == Some(word.as_str()));
    let hovered = response.hovered() || response.dragged();
    ui.painter().rect_filled(
        rect,
        3.0,
        if chosen {
            theme::CHOSEN
        } else if hovered {
            theme::HOVERED
        } else {
            theme::GROUND
        },
    );
    let room = Rect::from_min_size(rect.min + vec2(3.0, 3.0), vec2(rect.width() - 6.0, rect.height() - 22.0));
    // (Playing while hovered, from its start.)
    let since = if hovered {
        let started = match &resources.picker.resting {
            Some((key, when)) if *key == card.key => *when,
            _ => {
                resources.picker.resting = Some((card.key.clone(), Instant::now()));
                Instant::now()
            }
        };
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        Some(started.elapsed().as_secs_f64())
    } else {
        None
    };
    let context = ui.ctx().clone();
    match sheet(resources, decors, &context, card) {
        Some(sheet) => {
            let step = since.map_or(0, |seconds| sheet.step_at(seconds));
            // (A body's frames are wide round it: what is drawn in them shown, larger.)
            let (uv, cell) = if let Some((number, file)) = card.key.strip_prefix("body:").and_then(|rest| rest.split_once(':')) {
                if !resources.bounds.contains_key(&card.key)
                    && let (Ok(number), Ok(file)) = (number.parse::<i64>(), file.parse::<u8>())
                {
                    let bounds = super::pictures::body_bounds(&resources.session, number, file);
                    resources.bounds.insert(card.key.clone(), bounds);
                }
                super::pictures::cut_step(&sheet, step, resources.bounds.get(&card.key).copied().flatten())
            } else {
                (sheet.uv(step), sheet.cell)
            };
            ui.painter().image(sheet.texture.id(), fitted(cell, room), uv, Color32::WHITE);
        }
        None => {
            let first = card.label.chars().next().map(|letter| letter.to_string()).unwrap_or_default();
            ui.painter().text(room.center(), egui::Align2::CENTER_CENTER, first, FontId::proportional(20.0), theme::FAINT);
        }
    }
    let name = ui.painter().layout(card.label.clone(), FontId::proportional(theme::SMALL), theme::STRONG, rect.width() - 4.0);
    let name_at = egui::pos2(rect.center().x - name.size().x.min(rect.width() - 4.0) / 2.0, rect.bottom() - 17.0);
    ui.painter().with_clip_rect(rect).galley(name_at, name, theme::STRONG);
    if chosen {
        ui.painter().rect_stroke(rect, 3.0, Stroke::new(1.5_f32, theme::FILM), egui::StrokeKind::Inside);
    }
    handled(ui, resources, &response, card, written);
    hovered
}

/// What a card does: dragged, it hands its asset; double-clicked, written at
/// the cursor; clicked, a Pokémon chosen (its faces and gestures).
fn handled(ui: &egui::Ui, resources: &mut Resources, response: &egui::Response, card: &Card, written: &mut Option<Asset>) {
    if response.drag_started() {
        egui::DragAndDrop::set_payload(ui.ctx(), card.asset.clone());
    }
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        if let Some(at) = ui.input(|input| input.pointer.latest_pos()) {
            egui::Area::new(egui::Id::new("asset dragged")).order(egui::Order::Tooltip).fixed_pos(at + vec2(14.0, 10.0)).interactable(false).show(
                ui.ctx(),
                |ui| {
                    egui::Frame::new().fill(Color32::from_black_alpha(220)).inner_margin(egui::Margin::symmetric(6, 3)).corner_radius(3).show(ui, |ui| {
                        ui.label(theme::text(card.asset.said(), theme::SMALL, theme::FILM));
                    });
                },
            );
        }
    }
    if response.double_clicked() {
        *written = Some(card.asset.clone());
    } else if response.clicked()
        && let Asset::Pokemon { word } = &card.asset
    {
        resources.picker.who = Some(word.clone());
    }
    let hint = format!("{}\n{}\ndrag onto the Scene or the Script · double-click: at the Script's cursor", card.label, card.about);
    let _ = response.clone().on_hover_text(hint);
}

/// A card's picture: a thing's as the Resources draw it; a face; a body's animation.
fn sheet(resources: &mut Resources, decors: &Decors, context: &egui::Context, card: &Card) -> Option<Sheet> {
    if let Some(rest) = card.key.strip_prefix("face:") {
        let (number, face) = rest.split_once(':')?;
        let (number, face) = (number.to_string(), face.to_string());
        return cached(resources, context, &card.key, |resources, context| {
            let path = std::path::PathBuf::from(assets::data::path(resources.session.data(), &format!("portraits/{number}_{face}.png")));
            let texture = read_png(context, &path, &format!("picker face {number} {face}"), 0)?;
            let cell = texture.size_vec2();
            Some(Sheet { texture, cell, count: 1, ticks: vec![60] })
        });
    }
    if let Some(rest) = card.key.strip_prefix("body:") {
        let (number, file) = rest.split_once(':')?;
        let (number, file): (i64, u8) = (number.parse().ok()?, file.parse().ok()?);
        return cached(resources, context, &card.key, |resources, context| body_sheet(&resources.session, context, number, file));
    }
    if let Some(word) = card.key.strip_prefix("decor:") {
        // (A decor of one's own: its place's picture, if it is made of one.)
        if let Some(known) = resources.pictures.get(&card.key) {
            return known.clone();
        }
        let word = word.to_string();
        let backdrops = resources.list(Shelf::Backdrops, decors);
        return cached(resources, context, &card.key, |resources, context| {
            let text = std::fs::read_to_string(resources.session.library().decors().join(format!("{word}.decor"))).unwrap_or_default();
            let level = workshop_decor::own_place(&text)?;
            let item = backdrops.into_iter().find(|item| item.number == level)?;
            super::pictures::sheet_of(&resources.session, context, &item)
        });
    }
    resources.picture(context, card.item.as_ref()?)
}

/// A picture read once (so many each frame), kept with the Resources' own.
fn cached(resources: &mut Resources, context: &egui::Context, key: &str, make: impl FnOnce(&Resources, &egui::Context) -> Option<Sheet>) -> Option<Sheet> {
    if let Some(known) = resources.pictures.get(key) {
        return known.clone();
    }
    if resources.read_now >= READ_EACH_FRAME {
        context.request_repaint();
        return None;
    }
    resources.read_now += 1;
    let made = make(resources, context);
    resources.pictures.insert(key.to_string(), made.clone());
    made
}

/// The songs or the sounds, a row each: ▶, its name, its title or what it is, how long.
fn audio_rows(ui: &mut egui::Ui, resources: &mut Resources, decors: &Decors, tab: Tab, hovered_any: &mut bool, written: &mut Option<Asset>) {
    let shelf = if tab == Tab::Music { Shelf::Songs } else { Shelf::Sounds };
    if tab == Tab::Music {
        resources.song_lengths();
    }
    let sought = film::words::plain(resources.picker.sought.trim());
    let items: Vec<Item> = resources.list(shelf, decors).into_iter().filter(|item| answers(item, &sought)).collect();
    if items.is_empty() {
        ui.label(theme::weak("nothing by that name"));
    }
    let wide = ui.available_width();
    for item in items {
        let Some(card) = card_of(&item) else { continue };
        let (rect, response) = ui.allocate_exact_size(vec2(wide, 30.0), Sense::click_and_drag());
        if !ui.is_rect_visible(rect) {
            continue;
        }
        let heard = resources.heard.as_deref() == Some(card.key.as_str());
        let hovered = response.hovered();
        ui.painter().rect_filled(
            rect,
            2.0,
            if heard {
                theme::CHOSEN
            } else if hovered {
                theme::HOVERED
            } else {
                theme::GROUND
            },
        );
        let play = Rect::from_center_size(rect.left_center() + vec2(14.0, 0.0), vec2(16.0, 16.0));
        ui.painter().text(play.center(), egui::Align2::CENTER_CENTER, if heard { "■" } else { "▶" }, FontId::proportional(13.0), theme::FILM);
        let (name, under) = match tab {
            Tab::Music => {
                let title = rules::songs::SONGS.iter().find(|song| song.word == item.word).map_or(item.label.clone(), |song| song.title.to_string());
                (title, item.word.clone())
            }
            _ => (item.label.clone(), item.about.clone()),
        };
        let text_at = rect.left_top() + vec2(30.0, 2.0);
        let painter = ui.painter().with_clip_rect(rect.shrink(1.0));
        painter.text(text_at, egui::Align2::LEFT_TOP, &name, FontId::proportional(theme::BODY), theme::STRONG);
        painter.text(text_at + vec2(0.0, 14.0), egui::Align2::LEFT_TOP, &under, FontId::proportional(theme::SMALL - 1.0), theme::WEAK);
        if tab == Tab::Music
            && let Some((seconds, loops)) = resources.lengths.get(&(item.number as u16)).copied()
        {
            let said = format!("{}:{:02}{}", (seconds as u32) / 60, (seconds as u32) % 60, if loops { " ↻" } else { "" });
            painter.text(rect.right_center() - vec2(8.0, 0.0), egui::Align2::RIGHT_CENTER, said, FontId::proportional(theme::SMALL), theme::TEXT);
        }
        // (Resting on it a moment: heard; a click: heard on, or hushed.)
        if hovered {
            *hovered_any = true;
            let started = match &resources.picker.resting {
                Some((key, when)) if *key == card.key => *when,
                _ => {
                    resources.picker.resting = Some((card.key.clone(), Instant::now()));
                    if let Some(was) = resources.picker.hover_heard.take()
                        && resources.heard.as_deref() == Some(was.as_str())
                    {
                        hush(resources);
                    }
                    Instant::now()
                }
            };
            let rested = started.elapsed().as_secs_f64();
            if rested >= HEARD_AFTER && resources.picker.hover_heard.as_deref() != Some(card.key.as_str()) && !heard {
                hear_now(resources, &item);
                resources.picker.hover_heard = Some(card.key.clone());
            } else if rested < HEARD_AFTER {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
            }
        }
        if response.clicked() {
            if heard && resources.picker.hover_heard.as_deref() != Some(card.key.as_str()) {
                hush(resources);
            } else {
                // (Clicked: kept on when the pointer leaves.)
                if !heard {
                    hear_now(resources, &item);
                }
                resources.picker.hover_heard = None;
            }
        }
        handled(ui, resources, &response, &card, written);
    }
}

/// A song or a sound heard now (a song in place of the one heard).
pub(super) fn hear_now(resources: &mut Resources, item: &Item) {
    if item.shelf == Shelf::Songs && resources.heard.as_deref() == Some(item.key().as_str()) {
        return;
    }
    resources.hear(item);
}

/// What plays, hushed.
pub(super) fn hush(resources: &mut Resources) {
    if let Some(audio) = resources.audio.as_mut() {
        audio.song(0, None, 0.0, 0.0);
    }
    resources.heard = None;
    resources.picker.hover_heard = None;
}
