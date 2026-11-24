//! The Library: the shelves, how many on each.

use eframe::egui::{self, Color32, FontId, Sense, pos2, vec2};

use super::{Resources, Shelf};
use crate::decors::Decors;
use workshop_base::theme;

pub fn tree(ui: &mut egui::Ui, resources: &mut Resources, decors: &mut Decors) {
    resources.read_now = 0;
    ui.add_space(4.0);
    let groups = Shelf::GROUPS;
    let row = |ui: &mut egui::Ui, said: &str, count: Option<usize>, chosen: bool, deep: f32, colour: Color32, sign: Option<bool>| -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::click());
        if chosen {
            ui.painter().rect_filled(rect, 0.0, theme::CHOSEN);
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, theme::HOVERED);
        }
        if let Some(open) = sign {
            workshop_base::icons::chevron(ui.painter(), pos2(rect.min.x + 10.0 + deep, rect.center().y), open, theme::WEAK);
        }
        ui.painter().text(
            pos2(rect.min.x + 20.0 + deep, rect.center().y),
            egui::Align2::LEFT_CENTER,
            said,
            FontId::proportional(theme::BODY),
            if chosen { theme::STRONG } else { colour },
        );
        if let Some(count) = count {
            ui.painter().text(
                pos2(rect.max.x - 8.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                count.to_string(),
                FontId::proportional(theme::SMALL),
                if chosen { theme::STRONG } else { theme::FAINT },
            );
        }
        response
    };
    egui::ScrollArea::vertical().id_salt("resource library").auto_shrink(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for (group, shelves) in groups {
            if !group.is_empty() {
                let open = !resources.folded.contains(group);
                if row(ui, group, None, false, 0.0, theme::TEXT, Some(open)).clicked() {
                    if open {
                        resources.folded.insert(group);
                    } else {
                        resources.folded.remove(group);
                    }
                }
                if !open {
                    continue;
                }
            }
            for shelf in shelves {
                let count = resources.list(*shelf, decors).len();
                let chosen = resources.shelf == *shelf && resources.sought.trim().is_empty();
                let deep = if group.is_empty() { 0.0 } else { 16.0 };
                if row(ui, shelf.name(), Some(count), chosen, deep, theme::TEXT, None).clicked() {
                    resources.shelf = *shelf;
                    resources.sought.clear();
                    resources.scroll_wanted = true;
                }
            }
        }
        ui.add_space(10.0);
        let count = resources.list(Shelf::Favourites, decors).len();
        let chosen = resources.shelf == Shelf::Favourites && resources.sought.trim().is_empty();
        if row(ui, "★ Favourites", Some(count), chosen, 0.0, theme::FILM, None).clicked() {
            resources.shelf = Shelf::Favourites;
            resources.sought.clear();
            resources.scroll_wanted = true;
        }
    });
}
