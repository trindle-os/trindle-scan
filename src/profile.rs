//! Profiles: a named bundle of scan settings and what happens afterwards, kept in the program's config folder.

use crate::escl::{BLACK_WHITE, Caps, GREY, RGB, Settings};
use crate::settings::BASE_DPI;
use std::path::{Path, PathBuf};
use trindle_toolkit::t;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Glass,
    Feeder,
    FeederBoth,

    FeederManualBoth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Colour {
    Full,
    Grey,
    BlackWhite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    A4,
    A5,
    A3,
    Letter,
    Legal,
    Largest,
}

impl Size {
    pub const ALL: [Size; 6] = [Size::A4, Size::A5, Size::A3, Size::Letter, Size::Legal, Size::Largest];
    fn key(self) -> &'static str {
        match self {
            Size::A4 => "a4",
            Size::A5 => "a5",
            Size::A3 => "a3",
            Size::Letter => "letter",
            Size::Legal => "legal",
            Size::Largest => "largest",
        }
    }
    fn from_key(k: &str) -> Size {
        Size::ALL.into_iter().find(|s| s.key() == k).unwrap_or(Size::A4)
    }
    pub fn words(self) -> &'static str {
        match self {
            Size::A4 => t!("scan-profile-size-a4"),
            Size::A5 => t!("scan-profile-size-a5"),
            Size::A3 => t!("scan-profile-size-a3"),
            Size::Letter => t!("scan-profile-size-letter"),
            Size::Legal => t!("scan-profile-size-legal"),
            Size::Largest => t!("scan-profile-size-largest"),
        }
    }

    pub fn area(self) -> Option<(u32, u32)> {
        Some(match self {
            Size::A4 => (2480, 3508),
            Size::A5 => (1748, 2480),
            Size::A3 => (3508, 4961),
            Size::Letter => (2550, 3300),
            Size::Legal => (2550, 4200),
            Size::Largest => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Pdf,
    Jpeg,
    Png,
    Tiff,
}

impl Format {
    pub const ALL: [Format; 4] = [Format::Pdf, Format::Jpeg, Format::Png, Format::Tiff];
    fn key(self) -> &'static str {
        self.ext()
    }
    fn from_key(k: &str) -> Format {
        Format::ALL.into_iter().find(|f| f.key() == k).unwrap_or(Format::Pdf)
    }
    pub fn words(self) -> &'static str {
        match self {
            Format::Pdf => t!("scan-profile-format-pdf"),
            Format::Jpeg => t!("scan-profile-format-jpeg"),
            Format::Png => t!("scan-profile-format-png"),
            Format::Tiff => t!("scan-profile-format-tiff"),
        }
    }
    pub fn ext(self) -> &'static str {
        match self {
            Format::Pdf => "pdf",
            Format::Jpeg => "jpg",
            Format::Png => "png",
            Format::Tiff => "tif",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub name: String,
    pub scanner: String,
    pub host: String,
    pub port: u16,
    pub root: String,
    pub source: Source,
    pub size: Size,
    pub dpi: u32,
    pub colour: Colour,
    pub threshold: i32,
    pub brightness: i32,
    pub contrast: i32,
    pub rotate: u16,
    pub skip_blank: bool,
    pub ask_more: bool,
    pub auto_save: bool,
    pub folder: String,
    pub format: Format,
    pub pattern: String,
    pub clear_after: bool,
    pub trim: bool,
    pub straighten: bool,

    pub batch: u8,
    pub runs: u32,

    pub pause: u32,
    pub one_file: bool,

    pub searchable: bool,

    pub ocr_lang: String,
}

impl Profile {
    pub fn new(name: &str) -> Profile {
        Profile {
            name: name.into(),
            scanner: String::new(),
            host: String::new(),
            port: 80,
            root: "eSCL".into(),
            source: Source::Glass,
            size: Size::A4,
            dpi: BASE_DPI,
            colour: Colour::Full,
            threshold: 50,
            brightness: 0,
            contrast: 0,
            rotate: 0,
            skip_blank: false,
            ask_more: true,
            auto_save: false,
            folder: String::new(),
            format: Format::Pdf,
            pattern: crate::settings::default_pattern(),
            clear_after: true,
            trim: false,
            straighten: false,
            batch: 0,
            runs: 2,
            pause: 0,
            one_file: false,
            searchable: false,
            ocr_lang: "eng".into(),
        }
    }

    pub fn presets() -> Vec<Profile> {
        let mut bw = Profile::new(t!("scan-profile-preset-bw"));
        bw.colour = Colour::BlackWhite;
        let colour = Profile::new(t!("scan-profile-preset-colour"));
        let mut photo = Profile::new(t!("scan-profile-preset-photo"));
        photo.dpi = 600;
        photo.size = Size::Largest;
        photo.format = Format::Jpeg;
        vec![bw, colour, photo]
    }

    pub fn settings(&self, caps: &Caps) -> (Settings, Vec<String>) {
        let mut told = Vec::new();
        let by_feeder = self.source != Source::Glass && caps.feeder;
        let (gw, gh) = by_feeder.then_some(caps.feeder_area).flatten().or(caps.glass).unwrap_or((2480, 3508));
        if caps.glass.is_none() {
            told.push(t!("scan-profile-told-glass").to_string());
        }
        let (mut width, mut height) = self.size.area().unwrap_or((gw, gh));
        if width > gw || height > gh {
            width = width.min(gw);
            height = height.min(gh);
            told.push(t!("scan-profile-told-size").to_string());
        }
        let mut dpi = self.dpi;
        if !caps.resolutions.is_empty() && !caps.resolutions.contains(&dpi) {
            let near = *caps.resolutions.iter().min_by_key(|r| (**r as i32 - dpi as i32).abs()).unwrap();
            told.push(t!("scan-profile-told-dpi", dpi = dpi, near = near));
            dpi = near;
        }
        let want = match self.colour {
            Colour::Full => RGB,
            Colour::Grey | Colour::BlackWhite => GREY,
        };
        let mode = if want == RGB && caps.colour {
            RGB
        } else if caps.grey {
            GREY
        } else if caps.colour {
            RGB
        } else {
            BLACK_WHITE
        };
        let feeder = self.source != Source::Glass && caps.feeder;
        if self.source != Source::Glass && !caps.feeder {
            told.push(t!("scan-profile-told-feeder").to_string());
        }
        let duplex = self.source == Source::FeederBoth && caps.two_sided;
        if self.source == Source::FeederBoth && !caps.two_sided {
            told.push(t!("scan-profile-told-duplex").to_string());
        }
        (Settings { width, height, dpi, mode, feeder, duplex }, told)
    }

    pub fn encode(&self) -> String {
        let src = match self.source {
            Source::Glass => "glass",
            Source::Feeder => "feeder",
            Source::FeederBoth => "both",
            Source::FeederManualBoth => "manual",
        };
        let col = match self.colour {
            Colour::Full => "colour",
            Colour::Grey => "grey",
            Colour::BlackWhite => "bw",
        };
        let line = |k: &str, v: String| format!("{k}={}\n", v.replace('\n', " "));
        let mut out = String::from("[profile]\n");
        out += &line("name", self.name.clone());
        out += &line("scanner", self.scanner.clone());
        out += &line("host", self.host.clone());
        out += &line("port", self.port.to_string());
        out += &line("root", self.root.clone());
        out += &line("source", src.into());
        out += &line("size", self.size.key().into());
        out += &line("dpi", self.dpi.to_string());
        out += &line("colour", col.into());
        out += &line("threshold", self.threshold.to_string());
        out += &line("brightness", self.brightness.to_string());
        out += &line("contrast", self.contrast.to_string());
        out += &line("rotate", self.rotate.to_string());
        out += &line("skip_blank", self.skip_blank.to_string());
        out += &line("ask_more", self.ask_more.to_string());
        out += &line("auto_save", self.auto_save.to_string());
        out += &line("folder", self.folder.clone());
        out += &line("format", self.format.key().into());
        out += &line("pattern", self.pattern.clone());
        out += &line("clear_after", self.clear_after.to_string());
        out += &line("trim", self.trim.to_string());
        out += &line("straighten", self.straighten.to_string());
        out += &line("batch", self.batch.to_string());
        out += &line("runs", self.runs.to_string());
        out += &line("pause", self.pause.to_string());
        out += &line("one_file", self.one_file.to_string());
        out += &line("searchable", self.searchable.to_string());
        out += &line("ocr_lang", self.ocr_lang.clone());
        out
    }

    fn decode(text: &str) -> Profile {
        let mut p = Profile::new(t!("scan-profile-default-name"));
        for l in text.lines() {
            let Some((k, v)) = l.split_once('=') else { continue };
            let n = v.parse::<i64>().unwrap_or(0);
            let b = v == "true";
            match k {
                "name" => p.name = v.into(),
                "scanner" => p.scanner = v.into(),
                "host" => p.host = v.into(),
                "port" => p.port = v.parse().unwrap_or(80),
                "root" => p.root = v.into(),
                "source" => {
                    p.source = match v {
                        "feeder" => Source::Feeder,
                        "both" => Source::FeederBoth,
                        "manual" => Source::FeederManualBoth,
                        _ => Source::Glass,
                    }
                }
                "size" => p.size = Size::from_key(v),
                "dpi" => p.dpi = (n as u32).clamp(75, 1200),
                "colour" => {
                    p.colour = match v {
                        "grey" => Colour::Grey,
                        "bw" => Colour::BlackWhite,
                        _ => Colour::Full,
                    }
                }
                "threshold" => p.threshold = (n as i32).clamp(0, 100),
                "brightness" => p.brightness = (n as i32).clamp(-100, 100),
                "contrast" => p.contrast = (n as i32).clamp(-100, 100),
                "rotate" => p.rotate = if [90, 180, 270].contains(&(n as u16)) { n as u16 } else { 0 },
                "skip_blank" => p.skip_blank = b,
                "ask_more" => p.ask_more = b,
                "auto_save" => p.auto_save = b,
                "folder" => p.folder = v.into(),
                "format" => p.format = Format::from_key(v),
                "pattern" => p.pattern = v.into(),
                "clear_after" => p.clear_after = b,
                "trim" => p.trim = b,
                "straighten" => p.straighten = b,
                "batch" => p.batch = (n as u8).min(2),
                "runs" => p.runs = (n as u32).clamp(1, 99),
                "pause" => p.pause = (n as u32).min(600),
                "one_file" => p.one_file = b,
                "searchable" => p.searchable = b,
                "ocr_lang" if crate::ocr::valid_language(v) => p.ocr_lang = v.into(),
                _ => {}
            }
        }
        p
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Book {
    pub profiles: Vec<Profile>,
    pub current: usize,

    pub default: usize,
}

impl Book {
    pub fn first_run() -> Book {
        Book { profiles: Vec::new(), current: 0, default: 0 }
    }

    pub fn path() -> Option<PathBuf> {
        trindle_common::files::config_home().map(|base| base.join("trindle-scan").join("profiles.conf"))
    }

    pub fn load(path: &Path) -> Book {
        let Ok(text) = std::fs::read_to_string(path) else { return Book::first_run() };
        let mut book = Book::first_run();
        let mut chunks: Vec<String> = Vec::new();
        let mut head = String::new();
        for l in text.lines() {
            if l == "[profile]" {
                chunks.push(String::new());
            } else if let Some(c) = chunks.last_mut() {
                c.push_str(l);
                c.push('\n');
            } else {
                head.push_str(l);
                head.push('\n');
            }
        }
        book.profiles = chunks.iter().map(|c| Profile::decode(c)).collect();
        for l in head.lines().filter(|l| l.starts_with("current=") || l.starts_with("default=")) {
            let n = l.split_once('=').and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
            if l.starts_with("current") { book.current = n } else { book.default = n }
        }
        book.current = book.current.min(book.profiles.len().saturating_sub(1));
        book.default = book.default.min(book.profiles.len().saturating_sub(1));
        book
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let mut out = format!("current={}\ndefault={}\n", self.current, self.default);
        for p in &self.profiles {
            out += &p.encode();
        }
        trindle_common::files::write_private(path, out.as_bytes())
    }

    pub fn now(&self) -> Option<&Profile> {
        self.profiles.get(self.current)
    }

    pub fn name_free(&self, name: &str, except: usize) -> bool {
        !name.trim().is_empty() && !self.profiles.iter().enumerate().any(|(i, p)| i != except && p.name.eq_ignore_ascii_case(name.trim()))
    }

    pub fn unique(&self, base: &str) -> String {
        if self.name_free(base, usize::MAX) {
            return base.to_string();
        }
        (2..).map(|n| t!("scan-profile-numbered-name", name = base, n = n)).find(|c| self.name_free(c, usize::MAX)).unwrap()
    }

    pub fn add(&mut self, p: Profile) {
        self.profiles.push(p);
        self.current = self.profiles.len() - 1;
        if self.profiles.len() == 1 {
            self.default = 0;
        }
    }

    pub fn duplicate(&mut self, i: usize) {
        let Some(p) = self.profiles.get(i).cloned() else { return };
        let mut q = p;
        q.name = self.unique(&t!("scan-profile-copy-name", name = &q.name));
        self.profiles.insert(i + 1, q);
        if self.default > i {
            self.default += 1;
        }
        self.current = i + 1;
    }

    pub fn delete(&mut self, i: usize) {
        if i >= self.profiles.len() {
            return;
        }
        self.profiles.remove(i);
        if self.default == i {
            self.default = 0;
        } else if self.default > i {
            self.default -= 1;
        }
        if self.current > i {
            self.current -= 1;
        }
        self.current = self.current.min(self.profiles.len().saturating_sub(1));
    }

    pub fn move_by(&mut self, i: usize, by: isize) {
        let j = i as isize + by;
        if j < 0 || j as usize >= self.profiles.len() {
            return;
        }
        let j = j as usize;
        self.profiles.swap(i, j);
        for x in [&mut self.default, &mut self.current] {
            if *x == i {
                *x = j
            } else if *x == j {
                *x = i
            }
        }
    }
}

impl Profile {
    pub fn changes_pixels(&self) -> bool {
        self.rotate != 0 || self.straighten || self.trim || self.brightness != 0 || self.contrast != 0 || self.colour != Colour::Full || self.skip_blank
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testdir::tmp;

    #[test]
    fn profiles_survive_saving() {
        crate::words::ready();
        let d = tmp("book");
        let mut b = Book::first_run();
        let mut p = Profile::new("A");
        p.source = Source::FeederManualBoth;
        p.skip_blank = true;
        p.brightness = -20;
        b.add(p);
        b.add(Profile::new("B"));
        b.default = 1;
        b.save(&d.join("p.conf")).unwrap();
        assert_eq!(Book::load(&d.join("p.conf")), b);
    }

    #[test]
    fn names_are_unique_and_default_follows_deletes() {
        crate::words::ready();
        let mut b = Book::first_run();
        for n in ["A", "B", "C"] {
            b.add(Profile::new(n));
        }
        assert!(!b.name_free("b", 0));
        assert!(b.name_free("b", 1));
        assert_eq!(b.unique("A"), "A 2");
        b.default = 2;
        b.delete(0);
        assert_eq!(b.default, 1);
        b.delete(1);
        assert_eq!(b.default, 0);
    }

    #[test]
    fn deleting_a_profile_above_the_one_in_use_keeps_that_one_in_use() {
        crate::words::ready();
        let mut b = Book::first_run();
        for n in ["A", "B", "C"] {
            b.add(Profile::new(n));
        }
        b.current = 2;
        let name = b.now().unwrap().name.clone();
        b.delete(0);
        assert_eq!(b.now().unwrap().name, name);
        b.delete(1);
        assert_eq!(b.current, 0);
    }

    #[test]
    fn a_profile_called_profile_is_one_profile() {
        crate::words::ready();
        let d = tmp("tag");
        let mut b = Book::first_run();
        let mut p = Profile::new("a[profile]");
        p.size = Size::Legal;
        p.format = Format::Tiff;
        b.add(p);
        b.save(&d.join("p.conf")).unwrap();
        let back = Book::load(&d.join("p.conf"));
        assert_eq!(back.profiles.len(), 1);
        assert_eq!((back.profiles[0].size, back.profiles[0].format), (Size::Legal, Format::Tiff));
    }

    #[test]
    fn settings_fall_back_and_say_so() {
        crate::words::ready();
        let caps = Caps { glass: Some((2550, 3600)), resolutions: vec![200, 600], colour: true, ..Caps::default() };
        let mut p = Profile::new("x");
        p.source = Source::Feeder;
        p.dpi = 300;
        let (s, told) = p.settings(&caps);
        assert!(!s.feeder);
        assert_eq!(s.dpi, 200);
        assert_eq!(told.len(), 2);
    }

    #[test]
    fn a_legal_page_through_the_feeder_is_not_cut_to_the_glass() {
        crate::words::ready();
        let caps = Caps { glass: Some((2480, 3508)), feeder: true, feeder_area: Some((2550, 4200)), colour: true, ..Caps::default() };
        let mut p = Profile::new("x");
        p.size = Size::Legal;
        p.source = Source::Feeder;
        let (s, told) = p.settings(&caps);
        assert_eq!((s.width, s.height), Size::Legal.area().unwrap());
        assert!(told.is_empty());
        p.source = Source::Glass;
        assert_eq!(p.settings(&caps).0.height, 3508);
    }

    #[test]
    fn a_plain_profile_keeps_the_scanners_own_jpeg() {
        let mut p = Profile::new("t");
        p.rotate = 0;
        p.straighten = false;
        p.trim = false;
        p.brightness = 0;
        p.contrast = 0;
        p.colour = Colour::Full;
        p.skip_blank = false;
        assert!(!p.changes_pixels());
        p.trim = true;
        assert!(p.changes_pixels());
    }
}
