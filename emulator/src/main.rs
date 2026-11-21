//! `emulator <core> <rom> [frames] [out.png] [--press A:90]`: a game run
//! without a window, for that many frames (600), its last frame written as a
//! PNG (`target/emulator/shot.png`), the core's options told.
//! `--press A:90` holds A for three frames every 90 (through a title screen).
//! The core and the ROM are the user's own: nothing of them is kept.

use std::path::PathBuf;

use emulator::{Button, Core, Input, Options};

fn main() -> std::process::ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut press: Option<u32> = None;
    if let Some(at) = args.iter().position(|arg| arg == "--press") {
        press = args.get(at + 1).and_then(|said| said.trim_start_matches("A:").parse().ok());
        args.drain(at..(at + 2).min(args.len()));
    }
    if args.len() < 2 {
        eprintln!("emulator <core> <rom> [frames] [out.png] [--press A:90]");
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
    let started = std::time::Instant::now();
    let mut samples = 0;
    for frame in 1..=frames {
        let input = match press {
            Some(period) if frame % period < 3 => Input::default().with(Button::A),
            _ => Input::default(),
        };
        core.run(input);
        samples += core.take_audio().len() / 2;
    }
    let took = started.elapsed().as_secs_f64();
    let frame = core.frame();
    println!("{frames} frames in {took:.1} s ({:.0} a second), {samples} sound samples, frame {}x{}", frames as f64 / took, frame.width, frame.height);
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
