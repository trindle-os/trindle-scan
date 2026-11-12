//! USB scanners through SANE, loaded at run time; network names (net, airscan, escl) are left out of the list.

use crate::escl::{self, Caps, Fail, Settings};
use crate::settings::BASE_DPI;
use image::ExtendedColorType;
use std::cell::Cell;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use trindle_toolkit::t;

pub const PREFIX: &str = "sane:";

type Status = c_int;
const GOOD: Status = 0;
const DEVICE_BUSY: Status = 3;
const EOF: Status = 5;
const JAMMED: Status = 6;
const NO_DOCS: Status = 7;
const COVER_OPEN: Status = 8;
const IO_ERROR: Status = 9;
const ACCESS_DENIED: Status = 11;

const TYPE_BOOL: c_int = 0;
const TYPE_INT: c_int = 1;
const TYPE_FIXED: c_int = 2;
const UNIT_MM: c_int = 3;
const CONSTRAINT_RANGE: c_int = 1;
const CONSTRAINT_WORDS: c_int = 2;
const CONSTRAINT_STRINGS: c_int = 3;
const SET: c_int = 1;
const FRAME_GRAY: c_int = 0;
const FRAME_RGB: c_int = 1;

#[repr(C)]
struct Device {
    name: *const c_char,
    vendor: *const c_char,
    model: *const c_char,
    kind: *const c_char,
}

#[repr(C)]
struct OptionDesc {
    name: *const c_char,
    title: *const c_char,
    desc: *const c_char,
    kind: c_int,
    unit: c_int,
    size: c_int,
    cap: c_int,
    constraint_kind: c_int,
    constraint: *const c_void,
}

#[repr(C)]
struct Range {
    min: i32,
    max: i32,
    quant: i32,
}

#[repr(C)]
#[derive(Default)]
struct Params {
    format: c_int,
    last_frame: c_int,
    bytes_per_line: c_int,
    pixels_per_line: c_int,
    lines: c_int,
    depth: c_int,
}

type Handle = *mut c_void;

struct Lib {
    init: unsafe extern "C" fn(*mut c_int, *const c_void) -> Status,
    get_devices: unsafe extern "C" fn(*mut *const *const Device, c_int) -> Status,
    open: unsafe extern "C" fn(*const c_char, *mut Handle) -> Status,
    close: unsafe extern "C" fn(Handle),
    option_descriptor: unsafe extern "C" fn(Handle, c_int) -> *const OptionDesc,
    control_option: unsafe extern "C" fn(Handle, c_int, c_int, *mut c_void, *mut c_int) -> Status,
    get_parameters: unsafe extern "C" fn(Handle, *mut Params) -> Status,
    start: unsafe extern "C" fn(Handle) -> Status,
    read: unsafe extern "C" fn(Handle, *mut u8, c_int, *mut c_int) -> Status,
    cancel: unsafe extern "C" fn(Handle),
    strstatus: unsafe extern "C" fn(Status) -> *const c_char,
}

fn lib() -> Option<&'static Lib> {
    static LIB: OnceLock<Option<Lib>> = OnceLock::new();
    LIB.get_or_init(open_lib).as_ref()
}

#[allow(clippy::missing_transmute_annotations)]
fn open_lib() -> Option<Lib> {
    unsafe {
        let name = CString::new("libsane.so.1").ok()?;
        let h = libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
        if h.is_null() {
            return None;
        }
        macro_rules! sym {
            ($n:literal) => {{
                let s = CString::new($n).ok()?;
                let p = libc::dlsym(h, s.as_ptr());
                if p.is_null() {
                    return None;
                }
                std::mem::transmute(p)
            }};
        }
        let lib = Lib {
            init: sym!("sane_init"),
            get_devices: sym!("sane_get_devices"),
            open: sym!("sane_open"),
            close: sym!("sane_close"),
            option_descriptor: sym!("sane_get_option_descriptor"),
            control_option: sym!("sane_control_option"),
            get_parameters: sym!("sane_get_parameters"),
            start: sym!("sane_start"),
            read: sym!("sane_read"),
            cancel: sym!("sane_cancel"),
            strstatus: sym!("sane_strstatus"),
        };
        let mut version = 0;
        if (lib.init)(&mut version, std::ptr::null()) != GOOD {
            return None;
        }
        Some(lib)
    }
}

static BUSY: Mutex<()> = Mutex::new(());

const SETTLE: Duration = Duration::from_secs(5);
static LAST_SCAN: Mutex<Option<Instant>> = Mutex::new(None);

fn settle() {
    let last = *LAST_SCAN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(left) = last.and_then(|t| SETTLE.checked_sub(t.elapsed())) {
        std::thread::sleep(left);
    }
}

fn text(p: *const c_char) -> String {
    if p.is_null() { String::new() } else { unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned() }
}

fn missing() -> Fail {
    Fail::Other(t!("scan-sane-missing").into())
}

fn fail(lib: &Lib, status: Status) -> Fail {
    match status {
        DEVICE_BUSY => Fail::Busy,
        JAMMED => Fail::Jam,
        NO_DOCS => Fail::NoPaper,
        ACCESS_DENIED => Fail::Other(t!("scan-sane-denied").into()),
        COVER_OPEN => Fail::Other(t!("scan-sane-cover").into()),
        IO_ERROR => Fail::Offline(t!("scan-sane-io").into()),

        other => Fail::Other(text(unsafe { (lib.strstatus)(other) })),
    }
}

pub struct Found {
    pub name: String,

    pub device: String,
}

pub fn list() -> Vec<Found> {
    let Some(lib) = lib() else { return Vec::new() };
    let _one = BUSY.lock().unwrap_or_else(|e| e.into_inner());
    settle();
    let mut out = Vec::new();

    unsafe {
        let mut devices: *const *const Device = std::ptr::null();
        if (lib.get_devices)(&mut devices, 1) != GOOD || devices.is_null() {
            return out;
        }
        let mut i = 0;
        while !(*devices.add(i)).is_null() {
            let d = &**devices.add(i);
            if !is_local(&text(d.name)) {
                i += 1;
                continue;
            }
            let (vendor, model) = (text(d.vendor), text(d.model));
            let name = format!("{vendor} {model}").trim().to_string();
            out.push(Found { name: if name.is_empty() { text(d.name) } else { name }, device: text(d.name) });
            i += 1;
        }
    }
    out
}

fn is_local(device: &str) -> bool {
    let kind = device.split(':').next().unwrap_or("").to_lowercase();

    if matches!(kind.as_str(), "v4l" | "v4l2") {
        return false;
    }
    if kind == "escl" {
        return on_this_computer(device);
    }
    !matches!(kind.as_str(), "net" | "airscan" | "http" | "https")
}

fn on_this_computer(device: &str) -> bool {
    let rest = device.split_once("://").map_or("", |(_, r)| r);
    let host = rest.split(['/', ':']).next().unwrap_or("");
    let host = if rest.starts_with('[') { rest.trim_start_matches('[').split(']').next().unwrap_or("") } else { host };
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

struct Open {
    lib: &'static Lib,
    h: Handle,
    scanned: Cell<bool>,
    hardware: bool,
}

impl Drop for Open {
    fn drop(&mut self) {
        unsafe { (self.lib.close)(self.h) };
        if self.scanned.get() && self.hardware {
            *LAST_SCAN.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        }
    }
}

struct Opt {
    name: String,
    index: c_int,
    kind: c_int,
    unit: c_int,
    cap: c_int,
    size: usize,
    strings: Vec<String>,
    words: Vec<i32>,
    range: Option<(i32, i32, i32)>,
}

fn inside(v: i32, lo: i32, hi: i32) -> i32 {
    v.clamp(lo.min(hi), lo.max(hi))
}

const CAP_SOFT_SELECT: c_int = 1;
const CAP_INACTIVE: c_int = 32;

impl Opt {
    fn settable(&self) -> bool {
        self.cap & CAP_SOFT_SELECT != 0 && self.cap & CAP_INACTIVE == 0
    }
}

fn device_name(device: &str) -> Result<CString, Fail> {
    CString::new(device).map_err(|_| Fail::Other(t!("scan-sane-bad-device").into()))
}

impl Open {
    fn new(lib: &'static Lib, device: &str) -> Result<Open, Fail> {
        let name = device_name(device)?;
        let hardware = !device.starts_with("test:");
        if hardware {
            settle();
        }
        let mut h: Handle = std::ptr::null_mut();

        let st = unsafe { (lib.open)(name.as_ptr(), &mut h) };
        if st != GOOD {
            return Err(fail(lib, st));
        }
        Ok(Open { lib, h, scanned: Cell::new(false), hardware })
    }

    fn option(&self, name: &str) -> Option<Opt> {
        let mut i = 1;
        loop {
            let d = unsafe { (self.lib.option_descriptor)(self.h, i) };
            if d.is_null() {
                return None;
            }

            let d = unsafe { &*d };
            if text(d.name) == name {
                let mut o = Opt { name: name.to_string(), index: i, kind: d.kind, unit: d.unit, cap: d.cap, size: d.size.max(0) as usize, strings: Vec::new(), words: Vec::new(), range: None };

                unsafe {
                    match d.constraint_kind {
                        CONSTRAINT_STRINGS => {
                            let list = d.constraint as *const *const c_char;
                            let mut k = 0;
                            while !list.is_null() && !(*list.add(k)).is_null() {
                                o.strings.push(text(*list.add(k)));
                                k += 1;
                            }
                        }
                        CONSTRAINT_WORDS => {
                            let list = d.constraint as *const i32;
                            if !list.is_null() {
                                for k in 1..=usize::try_from(*list).unwrap_or(0) {
                                    o.words.push(*list.add(k));
                                }
                            }
                        }
                        CONSTRAINT_RANGE => {
                            let r = d.constraint as *const Range;
                            if !r.is_null() {
                                o.range = Some(((*r).min, (*r).max, (*r).quant));
                            }
                        }
                        _ => {}
                    }
                }
                return Some(o);
            }
            i += 1;
        }
    }

    fn set_string(&self, o: &Opt, value: &str) -> Result<(), Fail> {
        let mut buf = vec![0u8; o.size.max(value.len() + 1)];
        buf[..value.len()].copy_from_slice(value.as_bytes());
        let mut info = 0;

        match unsafe { (self.lib.control_option)(self.h, o.index, SET, buf.as_mut_ptr() as *mut c_void, &mut info) } {
            GOOD => Ok(()),
            _ => Err(refused(&o.name)),
        }
    }

    fn set_number(&self, o: &Opt, value: f64) -> Result<(), Fail> {
        let mut word: i32 = match o.kind {
            TYPE_FIXED => (value * 65536.0).round() as i32,
            TYPE_BOOL => (value != 0.0) as i32,
            TYPE_INT => value.round() as i32,
            _ => return Err(refused(&o.name)),
        };
        if let Some((lo, hi, _)) = o.range {
            word = inside(word, lo, hi);
        } else if !o.words.is_empty() {
            word = *o.words.iter().min_by_key(|w| (**w as i64 - word as i64).abs()).unwrap_or(&word);
        }
        let mut info = 0;

        match unsafe { (self.lib.control_option)(self.h, o.index, SET, &mut word as *mut i32 as *mut c_void, &mut info) } {
            GOOD => Ok(()),
            _ => Err(refused(&o.name)),
        }
    }
}

fn refused(name: &str) -> Fail {
    Fail::Other(t!("scan-sane-refused", what = name))
}

fn mm_to_300ths(mm: f64) -> u32 {
    (mm * BASE_DPI as f64 / 25.4).round().max(0.0) as u32
}

fn has(list: &[String], words: &[&str]) -> bool {
    list.iter().any(|s| words.iter().any(|w| s.to_lowercase().contains(w)))
}

fn pick<'a>(list: &'a [String], words: &[&str], not: &[&str]) -> Option<&'a String> {
    list.iter().find(|s| {
        let l = s.to_lowercase();
        words.iter().any(|w| l.contains(w)) && !not.iter().any(|n| l.contains(n))
    })
}

pub fn capabilities(device: &str) -> Result<Caps, Fail> {
    let lib = lib().ok_or_else(missing)?;
    let _one = BUSY.lock().unwrap_or_else(|e| e.into_inner());
    let dev = Open::new(lib, device)?;
    let mut caps = Caps { model: device.to_string(), ..Caps::default() };
    if let Some(src) = dev.option("source") {
        caps.feeder = has(&src.strings, &["adf", "feeder"]);
        caps.two_sided = has(&src.strings, &["duplex", "both"]);
    }
    match dev.option("mode") {
        Some(m) => {
            caps.colour = has(&m.strings, &["color", "colour"]);
            caps.grey = has(&m.strings, &["gray", "grey"]);
            caps.black_white = has(&m.strings, &["lineart", "binary", "halftone"]);
        }
        None => caps.colour = true,
    }
    if let Some(r) = dev.option("resolution") {
        let wanted = [75, 100, 150, 200, 300, 400, 600, 1200, 2400];
        let fixed = |w: i32| if r.kind == TYPE_FIXED { w / 65536 } else { w };
        caps.resolutions = if !r.words.is_empty() {
            r.words.iter().map(|w| fixed(*w).max(1) as u32).collect()
        } else if let Some((lo, hi, _)) = r.range {
            wanted.iter().copied().filter(|d| (fixed(lo)..=fixed(hi)).contains(&(*d as i32))).collect()
        } else {
            Vec::new()
        };
    }
    let size = |name: &str| dev.option(name).and_then(|o| o.range.map(|(_, hi, _)| (if o.kind == TYPE_FIXED { hi as f64 / 65536.0 } else { hi as f64 }, o.unit)));
    caps.glass = match (size("br-x"), size("br-y")) {
        (Some((x, UNIT_MM)), Some((y, UNIT_MM))) => Some((mm_to_300ths(x), mm_to_300ths(y))),
        _ => None,
    };
    Ok(caps)
}

fn apply_settings(dev: &Open, s: &Settings) -> Result<(), Fail> {
    let get = |name: &str| dev.option(name).filter(Opt::settable);
    if let Some(src) = get("source") {
        let choice = if s.feeder && s.duplex {
            pick(&src.strings, &["duplex", "both"], &[])
        } else if s.feeder {
            pick(&src.strings, &["adf", "feeder"], &["duplex", "both", "back"])
        } else {
            pick(&src.strings, &["flatbed", "glass", "platen"], &[])
        };
        if let Some(c) = choice {
            dev.set_string(&src, c)?;
        }
    }
    if let Some(m) = get("mode") {
        let words: &[&str] = match s.mode {
            escl::RGB => &["color", "colour"],
            escl::GREY => &["gray", "grey"],
            _ => &["lineart", "binary", "halftone"],
        };
        if let Some(c) = pick(&m.strings, words, &[]) {
            dev.set_string(&m, c)?;
        }
    }
    if let Some(r) = get("resolution") {
        dev.set_number(&r, s.dpi as f64)?;
    }
    let (w_mm, h_mm) = (s.width as f64 * 25.4 / BASE_DPI as f64, s.height as f64 * 25.4 / BASE_DPI as f64);
    for (name, value) in [("tl-x", 0.0), ("tl-y", 0.0), ("br-x", w_mm), ("br-y", h_mm)] {
        if let Some(o) = get(name) {
            let v = if o.unit == UNIT_MM { value } else { value / 25.4 * s.dpi as f64 };
            dev.set_number(&o, v)?;
        }
    }
    Ok(())
}

pub fn scan(device: &str, s: &Settings, stop: impl Fn() -> bool, mut page: impl FnMut(Vec<u8>)) -> Result<usize, Fail> {
    let lib = lib().ok_or_else(missing)?;
    let _one = BUSY.lock().unwrap_or_else(|e| e.into_inner());
    let dev = Open::new(lib, device)?;
    apply_settings(&dev, s)?;
    let mut n = 0;
    loop {
        if stop() {
            unsafe { (lib.cancel)(dev.h) };
            return Ok(n);
        }

        let st = unsafe { (lib.start)(dev.h) };
        if st == NO_DOCS && n > 0 {
            break;
        }
        if st != GOOD {
            return Err(fail(lib, st));
        }
        dev.scanned.set(true);
        let jpeg = read_page(&dev, &stop)?;
        match jpeg {
            Some(j) => {
                n += 1;
                page(j);
            }
            None => return Ok(n),
        }
        if !s.feeder {
            break;
        }
    }

    unsafe { (lib.cancel)(dev.h) };
    Ok(n)
}

fn read_page(dev: &Open, stop: &impl Fn() -> bool) -> Result<Option<Vec<u8>>, Fail> {
    let lib = dev.lib;
    let mut p = Params::default();

    let st = unsafe { (lib.get_parameters)(dev.h, &mut p) };
    if st != GOOD {
        return Err(fail(lib, st));
    }
    let mut data: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; 1 << 16];
    loop {
        if stop() {
            unsafe { (lib.cancel)(dev.h) };
            return Ok(None);
        }
        let mut got = 0;

        let st = unsafe { (lib.read)(dev.h, chunk.as_mut_ptr(), chunk.len() as c_int, &mut got) };
        data.extend_from_slice(&chunk[..got.max(0) as usize]);
        if data.len() > escl::LONGEST_REPLY {
            unsafe { (lib.cancel)(dev.h) };
            return Err(Fail::Other(t!("scan-sane-frame").into()));
        }
        match st {
            GOOD => {}
            EOF => break,
            other => return Err(fail(lib, other)),
        }
    }
    jpeg_of(&p, &data).map(Some)
}

fn jpeg_of(p: &Params, data: &[u8]) -> Result<Vec<u8>, Fail> {
    let (w, bpl) = (p.pixels_per_line.max(0) as usize, p.bytes_per_line.max(0) as usize);
    let channels = match p.format {
        FRAME_GRAY => 1,
        FRAME_RGB => 3,
        _ => return Err(Fail::Other(t!("scan-sane-frame").into())),
    };
    if p.depth == 1 && channels == 3 {
        return Err(Fail::Other(t!("scan-sane-frame").into()));
    }
    let needed = match p.depth {
        1 => w.div_ceil(8),
        16 => w * channels * 2,
        _ => w * channels,
    };
    if w == 0 || bpl < needed {
        return Err(Fail::Other(t!("scan-sane-frame").into()));
    }
    let h = if p.lines > 0 { (p.lines as usize).min(data.len() / bpl) } else { data.len() / bpl };
    let mut pixels = Vec::with_capacity(w * h * channels);
    for y in 0..h {
        let row = &data[y * bpl..(y + 1) * bpl];
        match p.depth {
            1 => {
                for x in 0..w {
                    let bit = (row[x / 8] >> (7 - x % 8)) & 1;
                    pixels.push(if bit == 1 { 0 } else { 255 });
                }
            }
            16 => {
                for x in 0..w * channels {
                    let at = x * 2;
                    let sample = u16::from_ne_bytes([row[at], row[at + 1]]);
                    pixels.push((sample >> 8) as u8);
                }
            }
            _ => pixels.extend_from_slice(&row[..(w * channels).min(row.len())]),
        }
    }
    let colour = if channels == 3 { ExtendedColorType::Rgb8 } else { ExtendedColorType::L8 };
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90).encode(&pixels, w as u32, h as u32, colour).map_err(|e| Fail::Other(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opt(cap: c_int) -> Opt {
        Opt { name: "source".into(), index: 1, kind: 3, unit: 0, cap, size: 0, strings: vec![], words: vec![], range: None }
    }

    #[test]
    fn an_inactive_or_fixed_option_is_not_set() {
        assert!(opt(CAP_SOFT_SELECT).settable());
        assert!(!opt(CAP_SOFT_SELECT | CAP_INACTIVE).settable());
        assert!(!opt(0).settable());
    }

    #[test]
    fn a_range_the_wrong_way_round_does_not_panic() {
        assert_eq!(inside(5, 10, 2), 5);
        assert_eq!(inside(50, 10, 2), 10);
        assert_eq!(inside(0, 2, 10), 2);
    }

    #[test]
    fn a_one_bit_colour_frame_is_refused() {
        crate::words::ready();
        let p = Params { format: FRAME_RGB, last_frame: 1, bytes_per_line: 2, pixels_per_line: 8, lines: 4, depth: 1 };
        assert!(jpeg_of(&p, &[0u8; 8]).is_err());
    }

    #[test]
    fn eight_bit_colour_and_one_bit_grey_become_pictures() {
        crate::words::ready();
        let rgb = Params { format: FRAME_RGB, last_frame: 1, bytes_per_line: 6, pixels_per_line: 2, lines: 1, depth: 8 };
        let j = jpeg_of(&rgb, &[255, 0, 0, 0, 0, 255]).unwrap();
        let img = image::load_from_memory(&j).unwrap();
        assert_eq!((img.width(), img.height()), (2, 1));
        let bits = Params { format: FRAME_GRAY, last_frame: 1, bytes_per_line: 1, pixels_per_line: 8, lines: 2, depth: 1 };
        let j = jpeg_of(&bits, &[0b1000_0000, 0b0111_1111]).unwrap();
        let img = image::load_from_memory(&j).unwrap().to_luma8();
        assert_eq!((img.width(), img.height()), (8, 2));
        assert!(img.get_pixel(0, 0)[0] < 60 && img.get_pixel(1, 0)[0] > 200, "a set bit is black");
    }

    #[test]
    fn a_short_row_is_refused_not_a_crash() {
        crate::words::ready();
        let bits = Params { format: FRAME_GRAY, last_frame: 1, bytes_per_line: 1, pixels_per_line: 16, lines: 1, depth: 1 };
        assert!(matches!(jpeg_of(&bits, &[0xff]), Err(Fail::Other(w)) if w.contains("can't read")));
        let deep = Params { format: FRAME_RGB, last_frame: 1, bytes_per_line: 5, pixels_per_line: 1, lines: 1, depth: 16 };
        assert!(jpeg_of(&deep, &[0; 5]).is_err());
    }

    #[test]
    fn network_names_are_not_local_and_a_nul_is_a_bad_device() {
        crate::words::ready();
        assert!(is_local("genesys:libusb:001:004") && is_local("test:0") && !is_local("v4l:/dev/video0"));
        assert!(!is_local("net:host:dev") && !is_local("airscan:e0:Printer") && !is_local("escl:http://x"));
        assert!(is_local("escl:http://localhost:60000/eSCL") && is_local("escl:http://127.0.0.1:60000") && is_local("escl:http://[::1]:60000") && !is_local("escl:http://localhost.example.com"));
        assert!(matches!(device_name("a\0b"), Err(Fail::Other(w)) if w.contains("can't use this scanner")));
    }

    #[test]
    fn a_split_colour_frame_is_refused_in_plain_words() {
        crate::words::ready();
        let red = Params { format: 2, last_frame: 0, bytes_per_line: 2, pixels_per_line: 2, lines: 1, depth: 8 };
        assert!(matches!(jpeg_of(&red, &[1, 2]), Err(Fail::Other(w)) if w.contains("can't read")));
    }

    #[test]
    #[ignore = "needs libsane and its test scanner; run alone: cargo test sane_test_scanner -- --ignored"]
    fn the_sane_test_scanner_scans_a_page() {
        crate::words::ready();
        let dir = crate::testdir::tmp("sane");
        std::fs::write(dir.join("dll.conf"), "test\n").unwrap();
        std::fs::write(dir.join("test.conf"), "").unwrap();

        unsafe { std::env::set_var("SANE_CONFIG_DIR", &*dir) };
        assert!(lib().is_some(), "libsane is needed for this test");
        let found = list();
        let f = found.first().expect("SANE has no test scanner here");
        let caps = capabilities(&f.device).unwrap();
        assert!(caps.colour && caps.glass.is_some());
        let settings = Settings { width: 600, height: 600, dpi: 75, mode: "RGB24", feeder: false, duplex: false };
        let mut pages = Vec::new();
        let n = scan(&f.device, &settings, || false, |j| pages.push(j)).unwrap();
        assert_eq!(n, 1);
        let img = image::load_from_memory(&pages[0]).unwrap();
        assert!(img.width() > 10 && img.height() > 10);
    }

    #[test]
    #[ignore = "needs a real Epson printer-scanner on USB; run alone: cargo test an_epson_scans_four_times_in_a_row -- --ignored --nocapture"]
    fn an_epson_scans_four_times_in_a_row() {
        crate::words::ready();
        for round in 0..4 {
            let found = list();
            let f = found.iter().find(|f| f.device.starts_with("epsonds")).unwrap_or_else(|| panic!("round {round}: no Epson found"));
            let st = Settings { width: 600, height: 600, dpi: 75, mode: "RGB24", feeder: false, duplex: false };
            let mut n = 0;
            let t = std::time::Instant::now();
            let r = scan(
                &f.device,
                &st,
                || false,
                |j| {
                    n += j.len();
                },
            );
            println!("ROUND {round} bytes {n} after {:?}", t.elapsed());
            assert!(r.is_ok() && n > 0, "round {round}: {:?}", r.map_err(|e| e.words()));
        }
    }
}
