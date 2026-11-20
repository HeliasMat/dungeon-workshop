//! Tools › SpriteCollab: the community's sprites and portraits
//! (PMDCollab/SpriteCollab), looked through from the clone kept in
//! `sources/spritecollab/`: a Pokémon sought by its name or number, its
//! faces, the first picture of each animation, who made them; imported into
//! data/extra by `spritecollab` (the credits kept in
//! data/extra/spritecollab_credits.txt). What is not in the clone yet is
//! fetched by git, which needs the network.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use eframe::egui::{self, RichText, TextureHandle, vec2};
use spritecollab::{FACES, LAST_IN_GAME, NEW, Places};

use crate::theme;
use crate::tool_run::Run;

/// A Pokémon of the community's, by its Pokédex number.
struct Entry {
    dex: u32,
    name: String,
    /// The game's monsters of that number (none: not in the game).
    monsters: Vec<u32>,
    /// Its folders are in the clone.
    here: bool,
}

/// The one chosen, as read from the clone.
struct Shown {
    dex: u32,
    faces: Vec<(String, TextureHandle)>,
    anims: Vec<(String, TextureHandle, egui::Vec2)>,
    credits: Vec<String>,
    fetched: bool,
}

pub struct Browser {
    places: Places,
    sought: String,
    entries: Vec<Entry>,
    chosen: Option<u32>,
    shown: Option<Shown>,
    replace: bool,
    run: Option<Run>,
    said: String,
    /// Something was imported (said under it); just now (the Resources to read their Pokémon again).
    pub imported: bool,
    pub fresh: bool,
}

impl Browser {
    /// Opened (on a Pokédex number, if one is given).
    pub fn new(session: &crate::session::Session, dex: Option<u32>) -> Browser {
        let mut browser = Browser {
            places: crate::tool_run::places(session),
            sought: String::new(),
            entries: Vec::new(),
            chosen: None,
            shown: None,
            replace: false,
            run: None,
            said: String::new(),
            imported: false,
            fresh: false,
        };
        browser.read();
        if let Some(dex) = dex {
            browser.chosen = Some(dex);
            browser.sought = dex.to_string();
        }
        browser
    }

    /// The Pokémon there are: the game's (their names), the community's (tracker.json), the clone's folders.
    fn read(&mut self) {
        let data = &self.places.data;
        let names: HashMap<u32, String> = assets::data::text(data, "monsters.txt")
            .lines()
            .filter_map(|line| {
                let (number, name) = line.split_once('\t')?;
                Some((number.trim().parse().ok()?, name.trim().to_string()))
            })
            .collect();
        let mut by_dex: BTreeMap<u32, Entry> = BTreeMap::new();
        let entry = |dex: u32| Entry { dex, name: String::new(), monsters: Vec::new(), here: false };
        for (monster, dex) in spritecollab::dex_of(data) {
            if dex == 0 {
                continue;
            }
            let found = by_dex.entry(dex).or_insert_with(|| entry(dex));
            found.monsters.push(monster);
            if found.name.is_empty()
                && let Some(name) = names.get(&monster).filter(|name| !name.is_empty() && !name.starts_with('?'))
            {
                found.name = name.clone();
            }
        }
        // (The community's own list: every Pokémon by its name.)
        for (dex, name) in spritecollab::tracker_names(&self.places.clone) {
            let found = by_dex.entry(dex).or_insert_with(|| entry(dex));
            if found.name.is_empty() {
                found.name = name;
            }
        }
        for kind in ["sprite", "portrait"] {
            for folder in std::fs::read_dir(self.places.clone.join(kind)).into_iter().flatten().flatten() {
                if let Ok(dex) = folder.file_name().to_string_lossy().parse::<u32>()
                    && dex > 0
                {
                    by_dex.entry(dex).or_insert_with(|| entry(dex)).here = true;
                }
            }
        }
        for found in by_dex.values_mut() {
            found.monsters.sort();
            // (A monster of one's own, 2000 and on, is in the game too.)
            if found.monsters.is_empty() && names.contains_key(&(NEW + found.dex)) {
                found.monsters.push(NEW + found.dex);
            }
            if found.name.is_empty() {
                found.name = format!("No. {:04}", found.dex);
            }
        }
        self.entries = by_dex.into_values().collect();
    }

    /// The one chosen read from the clone: its faces, its animations' first pictures, its credits.
    fn show(&mut self, context: &egui::Context, dex: u32) {
        let sprites = self.places.clone.join("sprite").join(format!("{dex:04}"));
        let portraits = self.places.clone.join("portrait").join(format!("{dex:04}"));
        let texture = |name: String, picture: image::RgbaImage| {
            let size = [picture.width() as usize, picture.height() as usize];
            context.load_texture(name, egui::ColorImage::from_rgba_unmultiplied(size, picture.as_raw()), egui::TextureOptions::NEAREST)
        };
        let faces: Vec<(String, TextureHandle)> = FACES
            .iter()
            .filter_map(|face| {
                let picture = image::open(portraits.join(format!("{face}.png"))).ok()?.to_rgba8();
                Some((face.to_string(), texture(format!("spritecollab {dex} {face}"), picture)))
            })
            .collect();
        let anims: Vec<(String, TextureHandle, egui::Vec2)> = spritecollab::animations(&sprites)
            .into_iter()
            .filter(|(name, anim)| *name == anim.file)
            .filter_map(|(name, anim)| {
                let sheet = image::open(sprites.join(format!("{name}-Anim.png"))).ok()?.to_rgba8();
                let first = image::imageops::crop_imm(&sheet, 0, 0, anim.wide.min(sheet.width()), anim.high.min(sheet.height())).to_image();
                let size = vec2(first.width() as f32, first.height() as f32);
                Some((name.clone(), texture(format!("spritecollab {dex} {name}"), first), size))
            })
            .collect();
        let fetched = sprites.join("AnimData.xml").is_file() || portraits.join("Normal.png").is_file();
        let credits = credits(&self.places.clone, &sprites, &portraits);
        self.shown = Some(Shown { dex, faces, anims, credits, fetched });
    }

    /// Some work started, its end told.
    fn start(&mut self, doing: &str, work: impl FnOnce(Places) -> Result<String, String> + Send + 'static) {
        self.said.clear();
        let places = self.places.clone();
        self.run = Some(Run::spawn(doing, move || work(places)));
    }
}

/// Who made them: the names of credits.txt's contributors (by credit_names.txt), the sprites', the portraits'.
fn credits(clone: &Path, sprites: &Path, portraits: &Path) -> Vec<String> {
    let mut known: HashMap<String, String> = HashMap::new();
    for line in std::fs::read_to_string(clone.join("credit_names.txt")).unwrap_or_default().lines().skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        if let Some(name) = fields.first().filter(|name| !name.is_empty()) {
            for key in fields.iter().take(2) {
                known.insert(key.to_string(), name.to_string());
            }
        }
    }
    let mut said = Vec::new();
    for (kind, folder) in [("Sprites", sprites), ("Portraits", portraits)] {
        let mut names: Vec<String> = Vec::new();
        for line in std::fs::read_to_string(folder.join("credits.txt")).unwrap_or_default().lines() {
            let Some(who) = line.split('\t').nth(1) else { continue };
            let name = known.get(who).cloned().unwrap_or_else(|| who.to_string());
            if !names.contains(&name) {
                names.push(name);
            }
        }
        if !names.is_empty() {
            said.push(format!("{kind}: {}", names.join(", ")));
        }
    }
    said
}

/// The window, while it is open.
pub fn show(context: &egui::Context, browser: &mut Option<Browser>) {
    let Some(made) = browser.as_mut() else {
        return;
    };
    // (The work's end: read again what it brought.)
    let ended = made.run.as_mut().and_then(|run| run.poll().cloned().map(|done| (run.doing.clone(), done)));
    if let Some((doing, done)) = ended {
        made.run = None;
        made.said = match done {
            Ok(said) => {
                if doing.starts_with("import") {
                    (made.imported, made.fresh) = (true, true);
                }
                made.read();
                made.shown = None;
                format!("done: {said}")
            }
            Err(why) => why,
        };
    } else if made.run.is_some() {
        context.request_repaint_after(std::time::Duration::from_millis(250));
    }
    let mut open = true;
    egui::Window::new("SpriteCollab").open(&mut open).default_size([860.0, 560.0]).show(context, |ui| {
        ui.label(theme::text(
            "the PMDCollab community's sprites and portraits (CC BY-NC 4.0: non-commercial use, credit given); imported into data/extra",
            theme::SMALL,
            theme::WEAK,
        ));
        if !made.places.clone.is_dir() {
            ui.label(theme::text(
                format!("no copy of SpriteCollab yet ({}): \"Fetch\" makes it (network, git)", made.places.clone.display()),
                theme::SMALL,
                theme::WRONG,
            ));
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut made.sought).hint_text("Search a Pokémon: name or number…").desired_width(260.0));
            ui.label(theme::text(format!("{} Pokémon", made.entries.len()), theme::SMALL, theme::FAINT));
            let busy = made.run.is_some();
            if ui.add_enabled(!busy, egui::Button::new("Update the List")).on_hover_text("the community's list (tracker.json), over the network").clicked() {
                made.start("list", |places| spritecollab::fetch(&places, &[], true).map(|()| "the community's list fetched".to_string()));
            }
        });
        ui.separator();
        let sought = film::words::plain(made.sought.trim());
        let wanted_number = sought.trim_start_matches('0').parse::<u32>().ok();
        let shown: Vec<usize> = made
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                sought.is_empty() || wanted_number.is_some_and(|number| entry.dex == number) || film::words::plain(&entry.name).contains(&sought)
            })
            .map(|(at, _)| at)
            .take(400)
            .collect();
        ui.horizontal_top(|ui| {
            // --- The list ---
            ui.vertical(|ui| {
                ui.set_width(230.0);
                egui::ScrollArea::vertical().id_salt("spritecollab list").max_height(470.0).auto_shrink([false, false]).show(ui, |ui| {
                    for at in &shown {
                        let entry = &made.entries[*at];
                        let mark = if entry.monsters.is_empty() { "  (not in the game)" } else { "" };
                        let colour = if entry.here { theme::TEXT } else { theme::FAINT };
                        let said = RichText::new(format!("{:04}  {}{mark}", entry.dex, entry.name)).size(theme::SMALL).color(colour);
                        if ui.selectable_label(made.chosen == Some(entry.dex), said).clicked() {
                            made.chosen = Some(entry.dex);
                        }
                    }
                    // (A number not listed: it can still be fetched.)
                    if let Some(number) = wanted_number.filter(|number| *number > 0 && !made.entries.iter().any(|entry| entry.dex == *number))
                        && ui.selectable_label(made.chosen == Some(number), RichText::new(format!("{number:04}  (to fetch)")).size(theme::SMALL)).clicked()
                    {
                        made.chosen = Some(number);
                    }
                });
            });
            ui.separator();
            // --- The one chosen ---
            ui.vertical(|ui| {
                ui.set_max_width(640.0);
                let Some(dex) = made.chosen else {
                    ui.label(theme::weak("Choose a Pokémon: its faces, its animations, who made them."));
                    return;
                };
                if made.shown.as_ref().is_none_or(|shown| shown.dex != dex) {
                    made.show(ui.ctx(), dex);
                }
                chosen(ui, made, dex);
            });
        });
        if let Some(run) = &made.run {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(theme::text(format!("{}…", run.doing), theme::SMALL, theme::WEAK));
            });
        } else if !made.said.is_empty() {
            ui.label(theme::text(&made.said, theme::SMALL, theme::TEXT));
        }
    });
    if !open {
        *browser = None;
    }
}

/// The chosen one's part of the window: what it is, its faces and animations, its credits, what to do.
fn chosen(ui: &mut egui::Ui, made: &mut Browser, dex: u32) {
    let entry = made.entries.iter().find(|entry| entry.dex == dex);
    let name = entry.map_or(format!("No. {dex:04}"), |entry| entry.name.clone());
    let monsters = entry.map(|entry| entry.monsters.clone()).unwrap_or_default();
    ui.label(RichText::new(format!("{name}  ·  Pokédex No. {dex:04}")).size(theme::TITLE).color(theme::STRONG));
    let in_game = if monsters.is_empty() {
        if dex > LAST_IN_GAME { format!("not in the game: it will join as No. {} (m{})", NEW + dex, NEW + dex) } else { "not in the game's tables".to_string() }
    } else {
        format!("in the game: {}", monsters.iter().map(|number| format!("No. {number}")).collect::<Vec<_>>().join(", "))
    };
    ui.label(theme::text(in_game, theme::SMALL, theme::WEAK));
    let busy = made.run.is_some();
    let Some(shown) = &made.shown else {
        return;
    };
    if !shown.fetched {
        ui.add_space(6.0);
        ui.label(theme::text("not in the copy of SpriteCollab yet", theme::SMALL, theme::WEAK));
        if ui.add_enabled(!busy, egui::Button::new("Fetch from SpriteCollab (network)")).clicked() {
            made.start("fetching", move |places| spritecollab::fetch(&places, &[dex], true).map(|()| format!("No. {dex:04} fetched")));
        }
        return;
    }
    egui::ScrollArea::vertical().id_salt("spritecollab chosen").max_height(380.0).auto_shrink([false, true]).show(ui, |ui| {
        ui.label(theme::text(format!("Faces ({})", shown.faces.len()), theme::SMALL, theme::STRONG));
        ui.horizontal_wrapped(|ui| {
            for (face, texture) in &shown.faces {
                ui.image((texture.id(), vec2(48.0, 48.0))).on_hover_text(face);
            }
        });
        ui.add_space(4.0);
        ui.label(theme::text(format!("Animations ({}): their first picture", shown.anims.len()), theme::SMALL, theme::STRONG));
        ui.horizontal_wrapped(|ui| {
            for (anim, texture, size) in &shown.anims {
                let scale = (64.0 / size.x.max(size.y)).clamp(0.5, 2.0);
                ui.vertical(|ui| {
                    ui.image((texture.id(), *size * scale));
                    ui.label(theme::text(anim, theme::SMALL - 1.0, theme::FAINT));
                });
            }
        });
        ui.add_space(4.0);
        ui.label(theme::text("Credits", theme::SMALL, theme::STRONG));
        if shown.credits.is_empty() {
            ui.label(theme::text("no credits in the copy", theme::SMALL, theme::FAINT));
        }
        for said in &shown.credits {
            ui.label(theme::text(said, theme::SMALL, theme::TEXT));
        }
    });
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut made.replace, "also in place of the game's").on_hover_text("without: only the animations and faces it lacks");
        let said = if monsters.is_empty() { "Add to the Project" } else { "Import" };
        let hover = "into data/extra (sprites, portraits, sprites.txt); the credits in data/extra/spritecollab_credits.txt";
        if ui
            .add_enabled(
                !busy && (!monsters.is_empty() || dex > LAST_IN_GAME),
                egui::Button::new(RichText::new(said).strong().color(theme::GROUND)).fill(theme::ASSETS),
            )
            .on_hover_text(hover)
            .clicked()
        {
            let replace = made.replace;
            if monsters.is_empty() {
                made.start("import", move |places| spritecollab::import_new(&places, &[dex]));
            } else {
                made.start("import", move |places| spritecollab::import_monsters(&places, &monsters, replace));
            }
        }
        if ui.add_enabled(!busy, egui::Button::new("Fetch Again")).on_hover_text("its folders updated from SpriteCollab (network)").clicked() {
            made.start("fetching", move |places| spritecollab::fetch(&places, &[dex], true).map(|()| format!("No. {dex:04} fetched")));
        }
    });
    if made.imported {
        ui.label(theme::text("imported: the new Pokémon and animations show everywhere once the Workshop is started again", theme::SMALL, theme::FAINT));
    }
}
