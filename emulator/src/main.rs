//! `emulator <core> <rom> [frames] [out.png] [--press A:90]`: a game run
//! without a window, for that many frames (600), its last frame written as a
//! PNG (`target/emulator/shot.png`), the core's options told.
//! `--press A:90[:1400]` holds A for three frames every 90 (until frame 1400): through a title screen;
//! `--every N` also writes a PNG every N frames (`<out>_<frame>.png`).
//! `--state-in <file>` starts from that savestate (after `--pokes <file>`, a
//! film's `.pokes` of `tools/ds/ds_cache.py`, is written in memory);
//! `--state-out <file>` keeps the last frame's.
//! The core and the ROM are the user's own: nothing of them is kept.

use std::path::PathBuf;

use emulator::{Button, Core, Input, Options};

fn main() -> std::process::ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut option = |name: &str| {
        let at = args.iter().position(|arg| arg == name)?;
        let value = args.get(at + 1).cloned();
        args.drain(at..(at + 2).min(args.len()));
        value
    };
    let press: Option<(u32, u32)> = option("--press").and_then(|said| {
        let mut parts = said.trim_start_matches("A:").split(':').map(|part| part.parse::<u32>().ok());
        Some((parts.next()??, parts.next().flatten().unwrap_or(u32::MAX)))
    });
    let every: Option<u32> = option("--every").and_then(|said| said.parse().ok());
    let state_in = option("--state-in");
    let state_out = option("--state-out");
    let pokes = option("--pokes");
    if args.len() < 2 {
        eprintln!("emulator <core> <rom> [frames] [out.png] [--press A:90[:until]] [--every N] [--state-in f] [--pokes f] [--state-out f]");
        return std::process::ExitCode::from(2);
    }
    let frames: u32 = args.get(2).and_then(|said| said.parse().ok()).unwrap_or(600);
    let out = PathBuf::from(args.get(3).cloned().unwrap_or_else(|| "target/emulator/shot.png".to_string()));
    let folder = PathBuf::from("target/emulator");
    let options = Options { core: PathBuf::from(&args[0]), system: folder.join("system"), saves: folder.join("saves"), wanted: emulator::ds_options() };
    let mut core = match Core::open(&options, &PathBuf::from(&args[1])) {
        Ok(core) => core,
        Err(why) => {
            eprintln!("{why}");
            return std::process::ExitCode::FAILURE;
        }
    };
    println!("{} {}: {:.2} frames a second, {} Hz", core.name, core.version, core.fps(), core.sample_rate());
    for variable in core.variables() {
        println!("  {} = {}   ({}: {})", variable.key, variable.value, variable.said, variable.values.join("|"));
    }
    if let Some(state) = &state_in {
        let loaded = std::fs::read(state).map_err(|why| format!("{state}: {why}")).and_then(|bytes| core.load_state(&bytes));
        if let Err(why) = loaded {
            eprintln!("{why}");
            return std::process::ExitCode::FAILURE;
        }
    }
    if let Some(pokes) = &pokes {
        let text = std::fs::read_to_string(pokes).unwrap_or_default();
        match core.memory().ok_or("the core shares no memory".to_string()).and_then(|memory| emulator::poke_film(memory, &text)) {
            Ok(count) => println!("{count} actors poked"),
            Err(why) => {
                eprintln!("{why}");
                return std::process::ExitCode::FAILURE;
            }
        }
    }
    let started = std::time::Instant::now();
    let mut samples = 0;
    for frame in 1..=frames {
        let input = match press {
            Some((period, until)) if frame < until && frame % period.max(1) < 3 => Input::default().with(Button::A),
            _ => Input::default(),
        };
        core.run(input);
        samples += core.take_audio().len() / 2;
        if every.is_some_and(|every| frame % every.max(1) == 0) {
            let shot = out.with_file_name(format!("{}_{frame}.png", out.file_stem().unwrap_or_default().to_string_lossy()));
            if let Err(why) = core.frame().save_png(&shot) {
                eprintln!("{why}");
            }
        }
    }
    let took = started.elapsed().as_secs_f64();
    let frame = core.frame();
    println!("{frames} frames in {took:.1} s ({:.0} a second), {samples} sound samples, frame {}x{}", frames as f64 / took, frame.width, frame.height);
    if let Some(state) = &state_out
        && let Err(why) = core.save_state().and_then(|bytes| std::fs::write(state, bytes).map_err(|why| format!("{state}: {why}")))
    {
        eprintln!("{why}");
    }
    for message in core.take_messages() {
        println!("  core: {message}");
    }
    match frame.save_png(&out) {
        Ok(()) => {
            println!("{}", out.display());
            std::process::ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("{why}");
            std::process::ExitCode::FAILURE
        }
    }
}
