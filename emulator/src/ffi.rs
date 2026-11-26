//! The libretro API, as much of it as this frontend speaks (`libretro.h`,
//! API version 1): its structures, its numbers, the functions a core exports.

use std::ffi::{c_char, c_void};

pub const API_VERSION: u32 = 1;
/// The machine's main memory, for `retro_get_memory_data`.
pub const MEMORY_SYSTEM_RAM: u32 = 2;

// The environment's commands answered (`RETRO_ENVIRONMENT_*`).
pub const EXPERIMENTAL: u32 = 0x10000;
pub const SET_ROTATION: u32 = 1;
pub const GET_OVERSCAN: u32 = 2;
pub const GET_CAN_DUPE: u32 = 3;
pub const SET_MESSAGE: u32 = 6;
pub const SET_PERFORMANCE_LEVEL: u32 = 8;
pub const GET_SYSTEM_DIRECTORY: u32 = 9;
pub const SET_PIXEL_FORMAT: u32 = 10;
pub const SET_INPUT_DESCRIPTORS: u32 = 11;
pub const GET_VARIABLE: u32 = 15;
pub const SET_VARIABLES: u32 = 16;
pub const GET_VARIABLE_UPDATE: u32 = 17;
pub const SET_SUPPORT_NO_GAME: u32 = 18;
pub const GET_SAVE_DIRECTORY: u32 = 31;
pub const SET_SYSTEM_AV_INFO: u32 = 32;
pub const SET_SUBSYSTEM_INFO: u32 = 34;
pub const SET_CONTROLLER_INFO: u32 = 35;
pub const SET_MEMORY_MAPS: u32 = 36 | EXPERIMENTAL;
pub const SET_GEOMETRY: u32 = 37;
pub const SET_SUPPORT_ACHIEVEMENTS: u32 = 42 | EXPERIMENTAL;
pub const SET_SERIALIZATION_QUIRKS: u32 = 44;
pub const GET_AUDIO_VIDEO_ENABLE: u32 = 47 | EXPERIMENTAL;
pub const GET_FASTFORWARDING: u32 = 49 | EXPERIMENTAL;
pub const GET_INPUT_BITMASKS: u32 = 51 | EXPERIMENTAL;
pub const GET_CORE_OPTIONS_VERSION: u32 = 52;
pub const SET_CORE_OPTIONS_DISPLAY: u32 = 55;
pub const GET_MESSAGE_INTERFACE_VERSION: u32 = 59;
pub const SET_VARIABLE: u32 = 70;

// The pixel formats (`enum retro_pixel_format`).
pub const PIXEL_0RGB1555: u32 = 0;
pub const PIXEL_XRGB8888: u32 = 1;
pub const PIXEL_RGB565: u32 = 2;

// The devices and their ids (`RETRO_DEVICE_*`).
pub const DEVICE_JOYPAD: u32 = 1;
pub const DEVICE_POINTER: u32 = 6;
pub const JOYPAD_MASK: u32 = 256;
pub const POINTER_X: u32 = 0;
pub const POINTER_Y: u32 = 1;
pub const POINTER_PRESSED: u32 = 2;

#[repr(C)]
pub struct SystemInfo {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GameGeometry {
    pub base_width: u32,
    pub base_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub aspect_ratio: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemTiming {
    pub fps: f64,
    pub sample_rate: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemAvInfo {
    pub geometry: GameGeometry,
    pub timing: SystemTiming,
}

#[repr(C)]
pub struct GameInfo {
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}

#[repr(C)]
pub struct Variable {
    pub key: *const c_char,
    pub value: *const c_char,
}

#[repr(C)]
pub struct Message {
    pub msg: *const c_char,
    pub frames: u32,
}

pub type Environment = unsafe extern "C" fn(cmd: u32, data: *mut c_void) -> bool;
pub type VideoRefresh = unsafe extern "C" fn(data: *const c_void, width: u32, height: u32, pitch: usize);
pub type AudioSample = unsafe extern "C" fn(left: i16, right: i16);
pub type AudioSampleBatch = unsafe extern "C" fn(data: *const i16, frames: usize) -> usize;
pub type InputPoll = unsafe extern "C" fn();
pub type InputState = unsafe extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16;

/// The functions every core exports, looked up once.
pub struct Api {
    pub set_environment: unsafe extern "C" fn(Environment),
    pub set_video_refresh: unsafe extern "C" fn(VideoRefresh),
    pub set_audio_sample: unsafe extern "C" fn(AudioSample),
    pub set_audio_sample_batch: unsafe extern "C" fn(AudioSampleBatch),
    pub set_input_poll: unsafe extern "C" fn(InputPoll),
    pub set_input_state: unsafe extern "C" fn(InputState),
    pub init: unsafe extern "C" fn(),
    pub deinit: unsafe extern "C" fn(),
    pub api_version: unsafe extern "C" fn() -> u32,
    pub get_system_info: unsafe extern "C" fn(*mut SystemInfo),
    pub get_system_av_info: unsafe extern "C" fn(*mut SystemAvInfo),
    pub set_controller_port_device: unsafe extern "C" fn(u32, u32),
    pub reset: unsafe extern "C" fn(),
    pub run: unsafe extern "C" fn(),
    pub serialize_size: unsafe extern "C" fn() -> usize,
    pub serialize: unsafe extern "C" fn(*mut c_void, usize) -> bool,
    pub unserialize: unsafe extern "C" fn(*const c_void, usize) -> bool,
    pub load_game: unsafe extern "C" fn(*const GameInfo) -> bool,
    pub unload_game: unsafe extern "C" fn(),
    pub get_memory_data: unsafe extern "C" fn(u32) -> *mut c_void,
    pub get_memory_size: unsafe extern "C" fn(u32) -> usize,
}

impl Api {
    /// Each function of the library, or the first one missing.
    ///
    /// # Safety
    /// The library must be a libretro core: its symbols of these names have these signatures.
    pub unsafe fn of(library: &libloading::Library) -> Result<Api, String> {
        macro_rules! get {
            ($name:literal) => {
                // SAFETY: the signature is libretro's for that symbol (the caller's promise).
                *unsafe { library.get(concat!($name, "\0").as_bytes()) }.map_err(|why| format!("not a libretro core: no {} ({why})", $name))?
            };
        }
        Ok(Api {
            set_environment: get!("retro_set_environment"),
            set_video_refresh: get!("retro_set_video_refresh"),
            set_audio_sample: get!("retro_set_audio_sample"),
            set_audio_sample_batch: get!("retro_set_audio_sample_batch"),
            set_input_poll: get!("retro_set_input_poll"),
            set_input_state: get!("retro_set_input_state"),
            init: get!("retro_init"),
            deinit: get!("retro_deinit"),
            api_version: get!("retro_api_version"),
            get_system_info: get!("retro_get_system_info"),
            get_system_av_info: get!("retro_get_system_av_info"),
            set_controller_port_device: get!("retro_set_controller_port_device"),
            reset: get!("retro_reset"),
            run: get!("retro_run"),
            serialize_size: get!("retro_serialize_size"),
            serialize: get!("retro_serialize"),
            unserialize: get!("retro_unserialize"),
            load_game: get!("retro_load_game"),
            unload_game: get!("retro_unload_game"),
            get_memory_data: get!("retro_get_memory_data"),
            get_memory_size: get!("retro_get_memory_size"),
        })
    }
}
