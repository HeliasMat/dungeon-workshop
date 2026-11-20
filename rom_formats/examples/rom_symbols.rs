//! Writes `rom_formats/src/sym.rs`: the addresses of the game's code and
//! data that the ROM's readers use, by pmdsky-debug symbol name.
//!
//!     cargo run --offline -p rom --example rom_symbols -- <pmdsky-debug folder>
//!
//! The names wanted are those the code says: `sym::NAME` or
//! `symbol("NAME", ...)` in `rom_formats/src` and `extractor/src`. Addresses
//! and lengths come from `<pmdsky-debug>/symbols/*.yml` (pmdsky-debug,
//! GPL-3, github.com/UsernameFodder/pmdsky-debug); SkyTemple's own additions
//! (after skytemple-files' `ppmdu_config/pmdsky_debug/extras.py`, GPL-3) are
//! written in `EXTRAS` here. Run it again when the code names a new symbol.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const REGIONS: [&str; 3] = ["EU", "NA", "JP"];

/// Symbols of SkyTemple's patches, not in pmdsky-debug: (EU, NA, JP)
/// addresses and lengths.
const EXTRAS: [(&str, [u32; 3], [u32; 3]); 2] = [
    // Tables the EditGuestPokemon patch adds to the ARM9 binary.
    ("GUEST_MONSTER_DATA2", [0x204F148, 0x204EE10, 0x204F168], [0x1D0, 0x1D0, 0x1D0]),
    ("EXTRA_DUNGEON_DATA", [0x204EFE0, 0x204ECA8, 0x204F000], [0x168, 0x168, 0x168]),
];

/// A symbol as found: its binary, its address and length per region.
struct Found {
    binary: String,
    address: [u32; 3],
    length: [u32; 3],
}

fn main() {
    let Some(reference) = std::env::args().nth(1) else {
        eprintln!("usage: rom_symbols <pmdsky-debug folder>");
        std::process::exit(2);
    };
    let symbols = Path::new(&reference).join("symbols");
    if !symbols.is_dir() {
        eprintln!("{}: not a pmdsky-debug checkout (no symbols/ folder)", reference);
        std::process::exit(2);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the workspace").to_path_buf();
    let out = root.join("rom_formats/src/sym.rs");
    let names = used(&root, &out);
    let known = read_symbols(&symbols, &names);
    let missing: Vec<&str> = names.iter().filter(|name| !known.contains_key(*name)).map(String::as_str).collect();
    if !missing.is_empty() {
        eprintln!("not in pmdsky-debug: {}", missing.join(", "));
        std::process::exit(1);
    }
    let mut lines: Vec<String> = [
        "//! Where the game keeps what the ROM's readers read, by pmdsky-debug",
        "//! symbol name (GPL-3; SkyTemple's patches for a few).",
        "//!",
        "//! Written by `cargo run -p rom --example rom_symbols -- <pmdsky-debug>`,",
        "//! which takes the names the code uses (`sym::NAME`,",
        "//! `symbol(\"NAME\", region)`): do not edit by hand.",
        "",
        "/// A symbol of the game's binaries: its address in each release (EU, NA,",
        "/// JP, the order of `NdsRom::region`; 0: unknown) and how many bytes it",
        "/// takes there (0: unknown).",
        "#[derive(Clone, Copy, Debug)]",
        "pub struct Symbol {",
        "    pub name: &'static str,",
        "    pub address: [u32; 3],",
        "    pub length: [u32; 3],",
        "}",
        "",
        "/// The address of a symbol in a release, by name; `None` where it is not",
        "/// known there.",
        "pub fn symbol(name: &str, region: usize) -> Option<u32> {",
        "    SYMBOLS.iter().find(|symbol| symbol.name == name).and_then(|symbol| symbol.address.get(region).copied()).filter(|&address| address != 0)",
        "}",
        "",
    ]
    .map(String::from)
    .to_vec();
    let words = |values: &[u32; 3]| values.iter().map(|&value| hex_word(value)).collect::<Vec<_>>().join(", ");
    for name in &names {
        let found = &known[name];
        lines.push(format!("/// In {}.", found.binary));
        lines.push(format!("pub const {name}: Symbol = Symbol {{"));
        lines.push(format!("    name: \"{name}\","));
        lines.push(format!("    address: [{}],", words(&found.address)));
        lines.push(format!("    length: [{}],", words(&found.length)));
        lines.push("};".into());
    }
    lines.push(String::new());
    lines.push("/// Every symbol above.".into());
    lines.push(format!("pub const SYMBOLS: [Symbol; {}] = [", names.len()));
    lines.extend(names.iter().map(|name| format!("    {name},")));
    lines.push("];".into());
    std::fs::write(&out, lines.join("\n") + "\n").expect("sym.rs written");
    // Laid out as the rest of the code (`rustfmt.toml`).
    match std::process::Command::new("rustfmt").args(["--edition", "2024"]).arg(&out).current_dir(&root).status() {
        Ok(_) => {}
        Err(_) => println!("rustfmt not found: the file is not laid out"),
    }
    println!("rom_formats/src/sym.rs: {} symbols", names.len());
}

/// The names the code uses, sorted.
fn used(root: &Path, out: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for crate_folder in ["rom_formats", "extractor"] {
        let mut files = Vec::new();
        rust_files(&root.join(crate_folder).join("src"), &mut files);
        for path in files {
            if path == out {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            names.extend(names_in(&text));
        }
    }
    names
}

fn rust_files(folder: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

/// `sym::NAME` (an upper-case name) and `symbol("NAME"` in a source.
fn names_in(text: &str) -> Vec<String> {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut names = Vec::new();
    for (at, _) in text.match_indices("sym::") {
        if text[..at].chars().next_back().is_some_and(word) {
            continue;
        }
        let rest = &text[at + 5..];
        let name: String = rest.chars().take_while(|&c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_').collect();
        let next = rest[name.len()..].chars().next();
        if name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) && !next.is_some_and(word) {
            names.push(name);
        }
    }
    for (at, _) in text.match_indices("symbol(\"") {
        if text[..at].chars().next_back().is_some_and(word) {
            continue;
        }
        let rest = &text[at + 8..];
        let name: String = rest.chars().take_while(|&c| word(c)).collect();
        if !name.is_empty() && rest[name.len()..].starts_with('"') {
            names.push(name);
        }
    }
    names
}

/// Every symbol of the `*.yml` files: name -> where it is. A name found
/// twice with other addresses keeps the first, and is told when wanted.
fn read_symbols(folder: &Path, wanted: &BTreeSet<String>) -> BTreeMap<String, Found> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
        .expect("pmdsky-debug's symbols")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "yml"))
        .collect();
    files.sort();
    let mut found = BTreeMap::new();
    for path in files {
        let text = std::fs::read_to_string(&path).expect("a symbols file");
        for (name, symbol) in parse_symbols(&text) {
            match found.get(&name) {
                Some(Found { binary, address, .. }) if *address != symbol.address => {
                    if wanted.contains(&name) {
                        println!("{name}: in {binary} and {}; the first is kept", symbol.binary);
                    }
                }
                Some(_) => {}
                None => {
                    found.insert(name, symbol);
                }
            }
        }
    }
    for (name, address, length) in EXTRAS {
        found.entry(name.to_string()).or_insert(Found { binary: "arm9 (SkyTemple's patches)".into(), address, length });
    }
    found
}

/// The symbols of one pmdsky-debug file, read line by line: the binary at
/// the left edge, `functions:` / `data:` under it, `- name:` items, and their
/// `address:` / `length:` maps of regions (a list of addresses: the first).
fn parse_symbols(text: &str) -> Vec<(String, Found)> {
    #[derive(PartialEq)]
    enum Key {
        Address,
        Length,
        Other,
    }
    let mut symbols: Vec<(String, Found)> = Vec::new();
    let mut binary = String::new();
    let mut in_list = false;
    let mut key = Key::Other;
    let mut region: Option<usize> = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let content = line.trim();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        match indent {
            0 => {
                binary = content.trim_end_matches(':').to_string();
                in_list = false;
                key = Key::Other;
            }
            2 => {
                in_list = content == "functions:" || content == "data:";
                key = Key::Other;
            }
            4 if in_list => {
                key = Key::Other;
                if let Some(name) = content.strip_prefix("- name:") {
                    let name = name.trim().trim_matches('"').to_string();
                    symbols.push((name, Found { binary: binary.clone(), address: [0; 3], length: [0; 3] }));
                }
            }
            6 if in_list => {
                key = match content {
                    "address:" => Key::Address,
                    "length:" => Key::Length,
                    _ => Key::Other,
                };
                region = None;
            }
            8 if in_list && key != Key::Other => {
                let Some((name, value)) = content.split_once(':') else { continue };
                region = REGIONS.iter().position(|&known| known == name.trim());
                if let (Some(at), Some(value)) = (region, number(value.trim())) {
                    set(&mut symbols, key == Key::Address, at, value);
                    region = None;
                }
            }
            _ if in_list && key != Key::Other && indent > 8 => {
                // A list of addresses under a region: the first is kept.
                if let (Some(at), Some(value)) = (region, content.strip_prefix('-').and_then(|value| number(value.trim()))) {
                    set(&mut symbols, key == Key::Address, at, value);
                    region = None;
                }
            }
            _ => {}
        }
    }
    fn set(symbols: &mut [(String, Found)], address: bool, at: usize, value: u32) {
        if let Some((_, found)) = symbols.last_mut() {
            if address { found.address[at] = value } else { found.length[at] = value }
        }
    }
    symbols
}

fn number(text: &str) -> Option<u32> {
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

fn hex_word(value: u32) -> String {
    let text = format!("{value:X}");
    if text.len() > 4 { format!("0x{}_{}", &text[..text.len() - 4], &text[text.len() - 4..]) } else { format!("0x{text}") }
}
