//! A small libretro frontend: a core the user chose (a melonDS or DeSmuME
//! libretro core, never bundled) loaded at run time, a game loaded in it and
//! run a frame at a time. What it draws comes out as RGBA, what it plays as
//! stereo samples; the keys and the touch screen go in each frame.
//!
//! A libretro core is one per process (its state is the library's own), and
//! its callbacks carry no pointer of ours: what they share with the frontend
//! is kept in one place (`SHARED`), and only one `Core` is open at a time.
//!
//! - `ffi`: the libretro API spoken.
//! - `Core`: a core and its game: run, reset, savestates, options.

pub mod ffi;

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The buttons of a joypad, as libretro numbers them (`RETRO_DEVICE_ID_JOYPAD_*`): bit n of `Input::buttons`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    B = 0,
    Y = 1,
    Select = 2,
    Start = 3,
    Up = 4,
    Down = 5,
    Left = 6,
    Right = 7,
    A = 8,
    X = 9,
    L = 10,
    R = 11,
}

/// What the player holds this frame: the buttons, and where the screen is
/// touched (in the frame's pixels, both screens as drawn), if it is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pub buttons: u16,
    pub touch: Option<(f32, f32)>,
}

impl Input {
    pub fn with(mut self, button: Button) -> Input {
        self.buttons |= 1 << button as u16;
        self
    }

    pub fn held(&self, button: Button) -> bool {
        self.buttons & (1 << button as u16) != 0
    }
}

/// A frame drawn: its size and its pixels, RGBA, row after row.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Frame {
    /// Written as a PNG.
    pub fn save_png(&self, path: &Path) -> Result<(), String> {
        let picture = image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone()).ok_or("no frame drawn yet")?;
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|why| format!("{}: {why}", folder.display()))?;
        }
        picture.save(path).map_err(|why| format!("{}: {why}", path.display()))
    }
}

/// One of the core's options: its key, what it says it is, its values (the first, its default), the one set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variable {
    pub key: String,
    pub said: String,
    pub values: Vec<String>,
    pub value: String,
}

/// Where a core is and what it is given: its folders, the options wanted
/// (a value the core does not offer is left at its default).
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub core: PathBuf,
    /// Its system folder (BIOS and firmware files, when it wants them).
    pub system: PathBuf,
    /// Where it writes the games' saves.
    pub saves: PathBuf,
    pub wanted: Vec<(String, String)>,
}

/// What the callbacks and the frontend share.
struct Shared {
    format: u32,
    frame: Frame,
    fresh: bool,
    audio: Vec<i16>,
    input: Input,
    /// The size of the frame the touch is in.
    drawn: (u32, u32),
    variables: Vec<Variable>,
    /// Each value handed to the core, kept alive while it may read it.
    handed: HashMap<String, CString>,
    wanted: HashMap<String, String>,
    updated: bool,
    system: CString,
    saves: CString,
    av: Option<ffi::SystemAvInfo>,
    messages: Vec<String>,
}

static SHARED: Mutex<Option<Shared>> = Mutex::new(None);
static OPEN: AtomicBool = AtomicBool::new(false);
/// The cores loaded so far: never unloaded (a core's library is not made to be loaded twice).
static LOADED: Mutex<Vec<(PathBuf, &'static ffi::Api)>> = Mutex::new(Vec::new());

fn shared() -> MutexGuard<'static, Option<Shared>> {
    SHARED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the core said, as a string (none: empty).
fn said(text: *const c_char) -> String {
    if text.is_null() {
        return String::new();
    }
    // SAFETY: the core hands NUL-terminated strings.
    unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
}

/// `"Description; one|two|three"`: the description and the values.
fn read_variable(key: String, text: &str) -> Variable {
    let (said, values) = text.split_once("; ").unwrap_or((text, ""));
    let values: Vec<String> = values.split('|').map(str::to_string).collect();
    Variable { key, said: said.to_string(), value: values.first().cloned().unwrap_or_default(), values }
}

unsafe extern "C" fn environment(cmd: u32, data: *mut c_void) -> bool {
    let mut guard = shared();
    let Some(shared) = guard.as_mut() else {
        return false;
    };
    // SAFETY (each arm): `data` points at what libretro.h says that command passes.
    unsafe {
        match cmd {
            ffi::GET_CAN_DUPE => *(data as *mut bool) = true,
            ffi::GET_OVERSCAN => *(data as *mut bool) = false,
            ffi::GET_SYSTEM_DIRECTORY => *(data as *mut *const c_char) = shared.system.as_ptr(),
            ffi::GET_SAVE_DIRECTORY => *(data as *mut *const c_char) = shared.saves.as_ptr(),
            ffi::SET_PIXEL_FORMAT => {
                let format = *(data as *const u32);
                if format > ffi::PIXEL_RGB565 {
                    return false;
                }
                shared.format = format;
            }
            ffi::GET_CORE_OPTIONS_VERSION => *(data as *mut u32) = 0,
            ffi::SET_VARIABLES => {
                let mut at = data as *const ffi::Variable;
                shared.variables.clear();
                while !at.is_null() && !(*at).key.is_null() {
                    let mut variable = read_variable(said((*at).key), &said((*at).value));
                    if let Some(wanted) = shared.wanted.get(&variable.key)
                        && variable.values.contains(wanted)
                    {
                        variable.value = wanted.clone();
                    }
                    shared.variables.push(variable);
                    at = at.add(1);
                }
            }
            ffi::GET_VARIABLE => {
                let asked = &mut *(data as *mut ffi::Variable);
                let key = said(asked.key);
                let Some(variable) = shared.variables.iter().find(|variable| variable.key == key) else {
                    asked.value = std::ptr::null();
                    return false;
                };
                let value = CString::new(variable.value.clone()).unwrap_or_default();
                asked.value = value.as_ptr();
                shared.handed.insert(key, value);
            }
            ffi::SET_VARIABLE => {
                if data.is_null() {
                    return true;
                }
                let set = &*(data as *const ffi::Variable);
                let (key, value) = (said(set.key), said(set.value));
                match shared.variables.iter_mut().find(|variable| variable.key == key) {
                    Some(variable) => variable.value = value,
                    None => return false,
                }
            }
            ffi::GET_VARIABLE_UPDATE => *(data as *mut bool) = std::mem::take(&mut shared.updated),
            ffi::SET_SYSTEM_AV_INFO => shared.av = Some(*(data as *const ffi::SystemAvInfo)),
            ffi::SET_GEOMETRY => {
                if let Some(av) = &mut shared.av {
                    av.geometry = *(data as *const ffi::GameGeometry);
                }
            }
            ffi::SET_MESSAGE => {
                let message = &*(data as *const ffi::Message);
                shared.messages.push(said(message.msg));
            }
            ffi::GET_INPUT_BITMASKS => {}
            ffi::GET_AUDIO_VIDEO_ENABLE => *(data as *mut i32) = 3,
            ffi::GET_FASTFORWARDING => *(data as *mut bool) = false,
            ffi::GET_MESSAGE_INTERFACE_VERSION => *(data as *mut u32) = 0,
            // (Told, nothing to do with it here.)
            ffi::SET_ROTATION
            | ffi::SET_PERFORMANCE_LEVEL
            | ffi::SET_INPUT_DESCRIPTORS
            | ffi::SET_SUPPORT_NO_GAME
            | ffi::SET_SUBSYSTEM_INFO
            | ffi::SET_CONTROLLER_INFO
            | ffi::SET_MEMORY_MAPS
            | ffi::SET_SUPPORT_ACHIEVEMENTS
            | ffi::SET_SERIALIZATION_QUIRKS
            | ffi::SET_CORE_OPTIONS_DISPLAY => {}
            _ => return false,
        }
    }
    true
}

unsafe extern "C" fn video_refresh(data: *const c_void, width: u32, height: u32, pitch: usize) {
    let mut guard = shared();
    let Some(shared) = guard.as_mut() else {
        return;
    };
    // (No data: the same frame again.)
    if data.is_null() || width == 0 || height == 0 {
        return;
    }
    let (wide, high) = (width as usize, height as usize);
    let frame = &mut shared.frame;
    frame.width = width;
    frame.height = height;
    frame.rgba.resize(wide * high * 4, 0);
    let bytes = data as *const u8;
    for row in 0..high {
        // SAFETY: the core's frame has `height` rows of `pitch` bytes.
        let line = unsafe { bytes.add(row * pitch) };
        let out = &mut frame.rgba[row * wide * 4..(row + 1) * wide * 4];
        match shared.format {
            ffi::PIXEL_XRGB8888 => {
                // SAFETY: a row of `width` pixels of 4 bytes.
                let pixels = unsafe { std::slice::from_raw_parts(line, wide * 4) };
                for (rgba, xrgb) in out.as_chunks_mut::<4>().0.iter_mut().zip(pixels.as_chunks::<4>().0) {
                    // (Little-endian 0xXXRRGGBB: B, G, R, X in memory.)
                    *rgba = [xrgb[2], xrgb[1], xrgb[0], 255];
                }
            }
            format => {
                // SAFETY: a row of `width` pixels of 2 bytes.
                let pixels = unsafe { std::slice::from_raw_parts(line, wide * 2) };
                for (rgba, pair) in out.as_chunks_mut::<4>().0.iter_mut().zip(pixels.as_chunks::<2>().0) {
                    let pixel = u16::from_le_bytes([pair[0], pair[1]]) as u32;
                    let (r, g, b) = match format {
                        ffi::PIXEL_RGB565 => ((pixel >> 11) & 31, ((pixel >> 5) & 63) >> 1, pixel & 31),
                        _ => ((pixel >> 10) & 31, (pixel >> 5) & 31, pixel & 31),
                    };
                    let wide8 = |five: u32| ((five << 3) | (five >> 2)) as u8;
                    *rgba = [wide8(r), wide8(g), wide8(b), 255];
                }
            }
        }
    }
    shared.fresh = true;
}

unsafe extern "C" fn audio_sample(left: i16, right: i16) {
    if let Some(shared) = shared().as_mut() {
        shared.audio.extend([left, right]);
    }
}

unsafe extern "C" fn audio_sample_batch(data: *const i16, frames: usize) -> usize {
    if let Some(shared) = shared().as_mut()
        && !data.is_null()
    {
        // SAFETY: `frames` stereo frames, two samples each.
        shared.audio.extend_from_slice(unsafe { std::slice::from_raw_parts(data, frames * 2) });
    }
    frames
}

unsafe extern "C" fn input_poll() {}

unsafe extern "C" fn input_state(port: u32, device: u32, _index: u32, id: u32) -> i16 {
    let guard = shared();
    let Some(shared) = guard.as_ref() else {
        return 0;
    };
    if port != 0 {
        return 0;
    }
    let input = shared.input;
    match device {
        ffi::DEVICE_JOYPAD if id == ffi::JOYPAD_MASK => input.buttons as i16,
        ffi::DEVICE_JOYPAD if id < 16 => ((input.buttons >> id) & 1) as i16,
        ffi::DEVICE_POINTER => {
            let Some((x, y)) = input.touch else {
                return 0;
            };
            // (From -0x7fff, the left or top edge of the frame, to 0x7fff, the other.)
            let scaled = |at: f32, side: u32| ((at / side.max(1) as f32).clamp(0.0, 1.0) * 2.0 - 1.0) * 32767.0;
            match id {
                ffi::POINTER_X => scaled(x, shared.drawn.0) as i16,
                ffi::POINTER_Y => scaled(y, shared.drawn.1) as i16,
                ffi::POINTER_PRESSED => 1,
                _ => 0,
            }
        }
        _ => 0,
    }
}

/// The core of that path's library: loaded the first time, kept after.
fn api_of(path: &Path) -> Result<&'static ffi::Api, String> {
    let mut loaded = LOADED.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((_, api)) = loaded.iter().find(|(known, _)| known == path) {
        return Ok(api);
    }
    if !path.is_file() {
        return Err(format!("no libretro core at {}", path.display()));
    }
    // SAFETY: loading a library runs its initialisers; the user chose it as a libretro core.
    let library = unsafe { libloading::Library::new(path) }.map_err(|why| format!("{}: {why}", path.display()))?;
    let library: &'static libloading::Library = Box::leak(Box::new(library));
    // SAFETY: a libretro core's symbols have libretro's signatures.
    let api: &'static ffi::Api = Box::leak(Box::new(unsafe { ffi::Api::of(library) }?));
    // SAFETY: a function of no argument.
    let version = unsafe { (api.api_version)() };
    if version != ffi::API_VERSION {
        return Err(format!("{}: libretro API {version}, not {}", path.display(), ffi::API_VERSION));
    }
    loaded.push((path.to_path_buf(), api));
    Ok(api)
}

/// A core with a game loaded in it.
pub struct Core {
    api: &'static ffi::Api,
    /// The core's name and version, as it says them.
    pub name: String,
    pub version: String,
    /// The game's path, and its bytes and path kept while the core may read them.
    pub game: PathBuf,
    _bytes: Vec<u8>,
    _path: CString,
}

impl Core {
    /// That core loaded, the game at `rom` loaded in it.
    pub fn open(options: &Options, rom: &Path) -> Result<Core, String> {
        let api = api_of(&options.core)?;
        if OPEN.swap(true, Ordering::SeqCst) {
            return Err("a core is already open".to_string());
        }
        match Core::start(api, options, rom) {
            Ok(core) => Ok(core),
            Err(why) => {
                *shared() = None;
                OPEN.store(false, Ordering::SeqCst);
                Err(why)
            }
        }
    }

    fn start(api: &'static ffi::Api, options: &Options, rom: &Path) -> Result<Core, String> {
        for folder in [&options.system, &options.saves] {
            std::fs::create_dir_all(folder).map_err(|why| format!("{}: {why}", folder.display()))?;
        }
        let text = |path: &Path| CString::new(path.display().to_string()).map_err(|_| format!("{}: a NUL in the path", path.display()));
        *shared() = Some(Shared {
            format: ffi::PIXEL_0RGB1555,
            frame: Frame::default(),
            fresh: false,
            audio: Vec::new(),
            input: Input::default(),
            drawn: (1, 1),
            variables: Vec::new(),
            handed: HashMap::new(),
            wanted: options.wanted.iter().cloned().collect(),
            updated: false,
            system: text(&options.system)?,
            saves: text(&options.saves)?,
            av: None,
            messages: Vec::new(),
        });
        let bytes = std::fs::read(rom).map_err(|why| format!("{}: {why}", rom.display()))?;
        let path = text(rom)?;
        let mut info = ffi::SystemInfo {
            library_name: std::ptr::null(),
            library_version: std::ptr::null(),
            valid_extensions: std::ptr::null(),
            need_fullpath: false,
            block_extract: false,
        };
        // SAFETY: libretro's order: callbacks set, init, then the game loaded.
        let loaded = unsafe {
            (api.set_environment)(environment);
            (api.set_video_refresh)(video_refresh);
            (api.set_audio_sample)(audio_sample);
            (api.set_audio_sample_batch)(audio_sample_batch);
            (api.set_input_poll)(input_poll);
            (api.set_input_state)(input_state);
            (api.get_system_info)(&mut info);
            (api.init)();
            let game = ffi::GameInfo { path: path.as_ptr(), data: bytes.as_ptr() as *const c_void, size: bytes.len(), meta: std::ptr::null() };
            (api.load_game)(&game)
        };
        let (name, version) = (said(info.library_name), said(info.library_version));
        if !loaded {
            // SAFETY: init was called.
            unsafe { (api.deinit)() };
            let told = shared().as_ref().map(|shared| shared.messages.join("; ")).unwrap_or_default();
            return Err(format!("{name} could not load {}{}", rom.display(), if told.is_empty() { String::new() } else { format!(": {told}") }));
        }
        let mut av = ffi::SystemAvInfo::default();
        // SAFETY: a game is loaded.
        unsafe {
            (api.get_system_av_info)(&mut av);
            (api.set_controller_port_device)(0, ffi::DEVICE_JOYPAD);
        }
        if let Some(shared) = shared().as_mut() {
            shared.av = Some(av);
        }
        Ok(Core { api, name, version, game: rom.to_path_buf(), _bytes: bytes, _path: path })
    }

    fn av(&self) -> ffi::SystemAvInfo {
        shared().as_ref().and_then(|shared| shared.av).unwrap_or_default()
    }

    /// Frames a second (59.83 for a DS).
    pub fn fps(&self) -> f64 {
        Some(self.av().timing.fps).filter(|fps| *fps > 0.0).unwrap_or(60.0)
    }

    /// Sound samples a second (each a left and a right).
    pub fn sample_rate(&self) -> u32 {
        self.av().timing.sample_rate.round() as u32
    }

    /// One frame run, with that held.
    pub fn run(&mut self, input: Input) {
        if let Some(shared) = shared().as_mut() {
            shared.drawn = (shared.frame.width.max(1), shared.frame.height.max(1));
            shared.input = input;
        }
        // SAFETY: a game is loaded; the callbacks lock `SHARED` themselves (not held here).
        unsafe { (self.api.run)() };
    }

    /// The last frame drawn, if one was since last asked.
    pub fn fresh_frame(&mut self) -> Option<Frame> {
        let mut guard = shared();
        let shared = guard.as_mut()?;
        std::mem::take(&mut shared.fresh).then(|| shared.frame.clone())
    }

    /// The last frame drawn.
    pub fn frame(&self) -> Frame {
        shared().as_ref().map(|shared| shared.frame.clone()).unwrap_or_default()
    }

    /// The sound made since last asked: left, right, left…
    pub fn take_audio(&mut self) -> Vec<i16> {
        shared().as_mut().map(|shared| std::mem::take(&mut shared.audio)).unwrap_or_default()
    }

    /// What the core said on the screen (`SET_MESSAGE`) since last asked.
    pub fn take_messages(&mut self) -> Vec<String> {
        shared().as_mut().map(|shared| std::mem::take(&mut shared.messages)).unwrap_or_default()
    }

    /// As if the console's power was switched off and on.
    pub fn reset(&mut self) {
        // SAFETY: a game is loaded.
        unsafe { (self.api.reset)() };
    }

    /// The whole machine, as bytes.
    pub fn save_state(&mut self) -> Result<Vec<u8>, String> {
        // SAFETY: a game is loaded; the buffer is as long as the core asks.
        unsafe {
            let size = (self.api.serialize_size)();
            if size == 0 {
                return Err(format!("{} makes no savestates", self.name));
            }
            let mut bytes = vec![0u8; size];
            match (self.api.serialize)(bytes.as_mut_ptr() as *mut c_void, size) {
                true => Ok(bytes),
                false => Err(format!("{} could not make a savestate", self.name)),
            }
        }
    }

    /// The machine as those bytes had it.
    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), String> {
        // SAFETY: a game is loaded; the bytes are read only.
        match unsafe { (self.api.unserialize)(bytes.as_ptr() as *const c_void, bytes.len()) } {
            true => Ok(()),
            false => Err(format!("{} could not read that savestate", self.name)),
        }
    }

    /// The core's options, as set.
    pub fn variables(&self) -> Vec<Variable> {
        shared().as_ref().map(|shared| shared.variables.clone()).unwrap_or_default()
    }

    /// An option set (read by the core on its next frame); false if it has no such value.
    pub fn set_variable(&mut self, key: &str, value: &str) -> bool {
        let mut guard = shared();
        let Some(shared) = guard.as_mut() else {
            return false;
        };
        let Some(variable) = shared.variables.iter_mut().find(|variable| variable.key == key && variable.values.iter().any(|known| known == value)) else {
            return false;
        };
        variable.value = value.to_string();
        shared.updated = true;
        true
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        // SAFETY: the game loaded is unloaded, then the core put away, as libretro asks.
        unsafe {
            (self.api.unload_game)();
            (self.api.deinit)();
        }
        *shared() = None;
        OPEN.store(false, Ordering::SeqCst);
    }
}

/// The options that make melonDS show both screens one above the other,
/// with no gap, touched with the pointer, booting the game without BIOS or
/// firmware files (its own FreeBIOS) and drawing in software. Other cores
/// ignore keys they do not have.
pub fn ds_options() -> Vec<(String, String)> {
    [
        ("melonds_screen_layout", "Top/Bottom"),
        ("melonds_screen_gap", "0"),
        ("melonds_touch_mode", "Touch"),
        ("melonds_boot_directly", "enabled"),
        ("melonds_console_mode", "DS"),
        ("melonds_opengl_renderer", "disabled"),
        // (DeSmuME's: the same layout, the pointer as the stylus.)
        ("desmume_screens_layout", "top/bottom"),
        ("desmume_screens_gap", "0"),
        ("desmume_pointer_type", "touch"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_variable_is_read_as_libretro_writes_it() {
        let variable = read_variable("melonds_screen_layout".to_string(), "Screen Layout; Top/Bottom|Bottom/Top|Left/Right");
        assert_eq!(variable.said, "Screen Layout");
        assert_eq!(variable.values, ["Top/Bottom", "Bottom/Top", "Left/Right"]);
        assert_eq!(variable.value, "Top/Bottom");
    }

    #[test]
    fn buttons_are_bits_as_libretro_numbers_them() {
        let input = Input::default().with(Button::A).with(Button::Start);
        assert_eq!(input.buttons, (1 << 8) | (1 << 3));
        assert!(input.held(Button::A) && !input.held(Button::B));
    }

    #[test]
    fn no_core_there_is_said() {
        let options = Options { core: PathBuf::from("no/such/core.dll"), ..Options::default() };
        let said = Core::open(&options, Path::new("no.nds")).err().unwrap_or_default();
        assert!(said.contains("no libretro core"), "{said}");
    }
}
