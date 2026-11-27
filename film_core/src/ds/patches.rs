//! The mega patch as the export sees it (tools/ds/patches.toml): which of
//! the community's patches the DS export's base has, as `megapatch.py`
//! picks them (each one `on`, then `DW_PATCHES`: `+Name`, `-Name`,
//! `only:A,B`; what they need with them), and the ids its special processes
//! get (`ProcessSpecial(id, a, b)`: 64 on, in the list's order). An export
//! for sharing (`DW_SHARE=1`, as `megapatch.py --share` makes the
//! shareable patch) has only those with a stated licence.

/// The repository's list of patches.
pub const PATCHES: &str = include_str!("../../../tools/ds/patches.toml");

/// The first id Sky leaves to added special processes (its own: 0-63).
const FIRST_PROCESS: u16 = 64;

/// The patches the text tags and special processes the export writes come from.
pub const FACE_MID_TEXT: &str = "ChangePortraitMidText";
pub const BOLD: &str = "BoldText";
pub const PORTRAIT_ALONE: &str = "set-portrait-monster";
pub const SEE_THROUGH: &str = "set_textbox_transparency";
pub const DYNAMIC_ACTOR: &str = "dynamic_actor";

/// One patch of the list, as far as choosing it goes.
struct Entry {
    name: String,
    kind: String,
    on: bool,
    shared: bool,
    depends: Vec<String>,
}

/// The patches a base has, in the order they are applied.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Patches {
    applied: Vec<String>,
    /// The special processes, by name, with their ids.
    processes: Vec<(String, u16)>,
    /// The patches left out because the base is for sharing.
    unshared: Vec<String>,
}

fn find<'a>(all: &'a [Entry], name: &str) -> Option<&'a Entry> {
    all.iter().find(|entry| entry.name == name)
}

fn entries(list: &str) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    for line in list.lines().map(str::trim) {
        if line == "[[patch]]" {
            out.push(Entry { name: String::new(), kind: String::new(), on: false, shared: false, depends: Vec::new() });
            continue;
        }
        let (Some(entry), Some((key, value))) = (out.last_mut(), line.split_once(" = ")) else { continue };
        let text = value.trim().trim_matches('"').to_string();
        match key.trim() {
            "name" => entry.name = text,
            "kind" => entry.kind = text,
            "default" => entry.on = text == "on",
            "share" => entry.shared = text == "yes",
            "depends" => {
                entry.depends =
                    value.trim().trim_matches(['[', ']']).split(',').map(|one| one.trim().trim_matches('"').to_string()).filter(|one| !one.is_empty()).collect()
            }
            _ => {}
        }
    }
    out
}

impl Patches {
    /// None: the base without the mega patch (the export as before it).
    pub fn none() -> Patches {
        Patches::default()
    }

    /// The base's patches: the list's, the choice (`DW_PATCHES`), for sharing or not.
    pub fn read(list: &str, choice: &str, share: bool) -> Patches {
        let all = entries(list);
        let by_name = |name: &str| find(&all, name);
        let mut wanted: Vec<String> = all.iter().filter(|entry| entry.on).map(|entry| entry.name.clone()).collect();
        let choice = choice.trim();
        if let Some(only) = choice.strip_prefix("only:") {
            wanted = only.split(',').map(str::trim).filter(|name| !name.is_empty()).map(str::to_string).collect();
        } else {
            for one in choice.split(',').map(str::trim) {
                if let Some(name) = one.strip_prefix('+')
                    && !wanted.iter().any(|had| had == name)
                {
                    wanted.push(name.to_string());
                } else if let Some(name) = one.strip_prefix('-') {
                    wanted.retain(|had| had != name);
                }
            }
        }
        // (Each after what it needs, as megapatch.py's `chosen`.)
        let mut order: Vec<String> = Vec::new();
        fn visit(name: &str, order: &mut Vec<String>, depth: usize, all: &[Entry]) {
            if order.iter().any(|had| had == name) || depth > 32 {
                return;
            }
            let Some(entry) = find(all, name) else { return };
            for need in &entry.depends {
                visit(need, order, depth + 1, all);
            }
            order.push(name.to_string());
        }
        for name in &wanted {
            visit(name, &mut order, 0, &all);
        }
        let mut unshared = Vec::new();
        if share {
            let shareable =
                |name: &str| by_name(name).is_some_and(|entry| entry.shared && entry.depends.iter().all(|need| by_name(need).is_some_and(|need| need.shared)));
            unshared = order.iter().filter(|name| !shareable(name)).cloned().collect();
            order.retain(|name| shareable(name));
        }
        let processes = order.iter().filter(|name| by_name(name).is_some_and(|entry| entry.kind == "sp")).cloned().zip(FIRST_PROCESS..).collect();
        Patches { applied: order, processes, unshared }
    }

    /// The export's base now: the repository's list, `DW_PATCHES`, and
    /// `DW_SHARE` (set: for sharing).
    pub fn current() -> Patches {
        let share = std::env::var("DW_SHARE").is_ok_and(|value| !value.is_empty() && value != "0");
        Patches::read(PATCHES, &std::env::var("DW_PATCHES").unwrap_or_default(), share)
    }

    /// Whether the base has that patch.
    pub fn has(&self, name: &str) -> bool {
        self.applied.iter().any(|one| one == name)
    }

    /// The id a special process of the base is called by.
    pub fn process(&self, name: &str) -> Option<u16> {
        self.processes.iter().find(|(one, _)| one == name).map(|(_, id)| *id)
    }

    /// Whether that patch was left out because the base is for sharing.
    pub fn unshared(&self, name: &str) -> bool {
        self.unshared.iter().any(|one| one == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "[[patch]]\nname = \"ExtractSPCode\"\nkind = \"skytemple\"\nshare = \"yes\"\ndefault = \"on\"\n\n\
        [[patch]]\nname = \"set-portrait-monster\"\nkind = \"sp\"\ndepends = [\"ExtractSPCode\"]\nshare = \"yes\"\ndefault = \"on\"\n\n\
        [[patch]]\nname = \"dynamic_actor\"\nkind = \"sp\"\ndepends = [\"ExtractSPCode\"]\nshare = \"no\"\ndefault = \"on\"\n\n\
        [[patch]]\nname = \"BoldText\"\nkind = \"skytemple\"\nshare = \"yes\"\ndefault = \"off\"\n";

    #[test]
    fn the_base_has_the_patches_on_and_what_they_need() {
        let patches = Patches::read(LIST, "", false);
        assert!(patches.has("ExtractSPCode") && patches.has(DYNAMIC_ACTOR) && !patches.has(BOLD));
        assert_eq!((patches.process(PORTRAIT_ALONE), patches.process(DYNAMIC_ACTOR)), (Some(64), Some(65)));
        assert!(Patches::read(LIST, "+BoldText", false).has(BOLD));
        assert_eq!(Patches::read(LIST, "-set-portrait-monster", false).process(DYNAMIC_ACTOR), Some(64));
        // (A special process brings what it needs.)
        assert!(Patches::read(LIST, "only:dynamic_actor", false).has("ExtractSPCode"));
        assert!(!Patches::read(LIST, "only:BoldText", false).has(DYNAMIC_ACTOR));
    }

    #[test]
    fn a_base_for_sharing_has_only_the_licensed() {
        let shared = Patches::read(LIST, "", true);
        assert!(!shared.has(DYNAMIC_ACTOR) && shared.unshared(DYNAMIC_ACTOR));
        assert_eq!(shared.process(PORTRAIT_ALONE), Some(64));
    }

    #[test]
    fn the_repository_s_special_processes_are_64_to_67() {
        let patches = Patches::read(PATCHES, "", false);
        assert_eq!([PORTRAIT_ALONE, "set_text_SE", SEE_THROUGH, DYNAMIC_ACTOR].map(|name| patches.process(name)), [Some(64), Some(65), Some(66), Some(67)]);
        assert!(patches.has(FACE_MID_TEXT) && patches.has(BOLD) && patches.has("GroundSpriteRoom"));
        assert!(Patches::none().process(PORTRAIT_ALONE).is_none());
    }
}
