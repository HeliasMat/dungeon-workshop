//! The command palette (Ctrl+Shift+P, as VS Code's): every action of the
//! menus, the spaces, the panels to open or close, the last files, found by a
//! few letters of their names; Enter (or a click) does it.

use eframe::egui::{self, Key, Modifiers, RichText};
use workshop_base::theme;

/// An action, as the menus and the palette ask it of the window (`App::run`).
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    NewFilm,
    NewDecor,
    Save,
    Undo,
    Redo,
    ReloadProject,
    Quit,
    Go(workshop_base::ask::Space),
    /// A panel of the space looked at opened, or closed (its title).
    Panel(&'static str),
    DefaultLayout,
    Shortcuts,
    FirstFilm,
    GameSpaces,
    Tilequant,
    SpriteCollab,
    ExportVideo,
    Open(std::path::PathBuf),
}

/// An action offered: how it is said, its key, whether it can be done now.
#[derive(Clone, Debug)]
pub struct Offered {
    pub said: String,
    pub key: &'static str,
    pub command: Command,
    pub can: bool,
}

impl Offered {
    pub fn new(said: impl Into<String>, key: &'static str, command: Command, can: bool) -> Offered {
        Offered { said: said.into(), key, command, can }
    }
}

/// How well words sought name an action (lower is better), if each of them is in its name (or its key): a word at
/// the start of the name best, then at the start of one of its words, then anywhere.
fn score(said: &str, key: &str, sought: &str) -> Option<usize> {
    let said = said.to_lowercase();
    let key = key.to_lowercase();
    let mut total = 0;
    for word in sought.to_lowercase().split_whitespace() {
        total += match said.find(word) {
            Some(0) => 0,
            Some(at) if said[..at].ends_with([' ', '›', '(', '-', ':']) => 1,
            Some(_) => 3,
            None if key.contains(word) => 4,
            None => return None,
        };
    }
    Some(total)
}

/// The actions found for those words, the best first (those that can be done before those that can't, the
/// shortest names first); all of them, in their order, for none.
pub fn found<'a>(offered: &'a [Offered], sought: &str) -> Vec<&'a Offered> {
    let mut found: Vec<(usize, &Offered)> = offered.iter().filter_map(|one| score(&one.said, one.key, sought).map(|score| (score, one))).collect();
    if !sought.trim().is_empty() {
        found.sort_by_key(|(score, one)| (!one.can, *score, one.said.len()));
    }
    found.into_iter().map(|(_, one)| one).collect()
}

/// The palette while open: what is sought, the one chosen (by the arrows).
#[derive(Default)]
pub struct Palette {
    pub sought: String,
    pub chosen: usize,
    pub fresh: bool,
}

/// The palette drawn over the window: a field, the actions found; the one asked (Enter, a click), and whether it
/// is to close (Esc, a click elsewhere, an action done).
pub fn palette(context: &egui::Context, palette: &mut Palette, offered: &[Offered]) -> (Option<Command>, bool) {
    let mut asked = None;
    // (Just opened by a click in a menu: that click is not one elsewhere.)
    let opening = palette.fresh;
    let mut close = context.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
    let shown = found(offered, &palette.sought);
    let (down, up, enter) = context.input_mut(|input| {
        (input.consume_key(Modifiers::NONE, Key::ArrowDown), input.consume_key(Modifiers::NONE, Key::ArrowUp), input.consume_key(Modifiers::NONE, Key::Enter))
    });
    if down {
        palette.chosen = (palette.chosen + 1).min(shown.len().saturating_sub(1));
    }
    if up {
        palette.chosen = palette.chosen.saturating_sub(1);
    }
    palette.chosen = palette.chosen.min(shown.len().saturating_sub(1));
    if enter && let Some(one) = shown.get(palette.chosen).filter(|one| one.can) {
        asked = Some(one.command.clone());
    }
    let screen = context.screen_rect();
    let area = egui::Area::new(egui::Id::new("command palette")).order(egui::Order::Foreground).fixed_pos(egui::pos2(screen.center().x - 310.0, 60.0));
    let inside = area.show(context, |ui| {
        egui::Frame::popup(ui.style()).fill(theme::PANEL).inner_margin(8).show(ui, |ui| {
            ui.set_width(620.0);
            let field = ui.add(egui::TextEdit::singleline(&mut palette.sought).hint_text("a command, a space, a panel, a file…").desired_width(f32::INFINITY));
            if palette.fresh {
                field.request_focus();
                palette.fresh = false;
            }
            if field.changed() {
                palette.chosen = 0;
            }
            ui.add_space(4.0);
            egui::ScrollArea::vertical().max_height(420.0).auto_shrink([false, true]).show(ui, |ui| {
                if shown.is_empty() {
                    ui.label(theme::weak("nothing by that name"));
                }
                for (at, one) in shown.iter().enumerate() {
                    let here = at == palette.chosen;
                    let colour = if !one.can {
                        theme::FAINT
                    } else if here {
                        theme::STRONG
                    } else {
                        theme::TEXT
                    };
                    let row = ui
                        .horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            let label = ui.selectable_label(here, RichText::new(&one.said).color(colour));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(one.key).monospace().size(theme::SMALL).color(theme::WEAK));
                            });
                            label
                        })
                        .inner;
                    if here && (down || up) {
                        row.scroll_to_me(None);
                    }
                    if row.clicked() && one.can {
                        asked = Some(one.command.clone());
                    }
                    if !one.can {
                        row.on_hover_text("not now (nothing open for it here)");
                    }
                }
            });
        });
    });
    if inside.response.clicked_elsewhere() && !opening {
        close = true;
    }
    close |= asked.is_some();
    (asked, close)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The palette finds by a few letters: each word sought in the name (or the key), the start of the name best,
    /// what can be done before what can't; nothing sought: all, in their order.
    #[test]
    fn the_palette_finds_by_a_few_letters() {
        let offered = vec![
            Offered::new("File › Save", "Ctrl+S", Command::Save, false),
            Offered::new("File › New Film…", "", Command::NewFilm, true),
            Offered::new("File › Export Video", "", Command::ExportVideo, true),
            Offered::new("Go to: Film", "", Command::Go(workshop_base::ask::Space::Film), true),
            Offered::new("View › Panel: Timeline", "", Command::Panel("Timeline"), true),
            Offered::new("Edit › Undo", "Ctrl+Z", Command::Undo, true),
        ];
        let said = |sought: &str| found(&offered, sought).into_iter().map(|one| one.said.as_str()).collect::<Vec<_>>();
        assert_eq!(said("film"), vec!["Go to: Film", "File › New Film…"], "a word's start, the shorter first");
        assert_eq!(said("new fi"), vec!["File › New Film…"]);
        assert_eq!(said("vid"), vec!["File › Export Video"]);
        assert_eq!(said("ctrl+z"), vec!["Edit › Undo"], "by its key");
        assert_eq!(said("time"), vec!["View › Panel: Timeline"]);
        assert_eq!(
            said("e"),
            vec!["Edit › Undo", "File › New Film…", "File › Export Video", "View › Panel: Timeline", "File › Save"],
            "what can't be done last"
        );
        assert_eq!(said("").len(), 6, "nothing sought: all");
        assert!(said("zzz").is_empty());
    }
}
