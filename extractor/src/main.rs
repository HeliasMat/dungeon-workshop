//! `extractor <rom> [folder]`: reads the game's data out of your own copy
//! of the ROM, once, into `data/rom/` (see the library); `extractor dump
//! <what> <rom> ...`, pictures and scripts written out to look at
//! (`dump.rs`).

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.first().is_none_or(|word| matches!(word.as_str(), "--help" | "-h")) {
        println!("extractor <rom> [folder] [--tables-only] [--no-pictures] [--steps-only]   writes data/rom from your own copy of the game");
        println!("{}", extractor::dump::HELP);
        std::process::exit(if arguments.is_empty() { 2 } else { 0 });
    }
    if arguments[0] == "dump" {
        if let Err(why) = extractor::dump::run(&arguments[1..]) {
            eprintln!("{why}");
            std::process::exit(2);
        }
        return;
    }
    extractor::run(&arguments);
}
