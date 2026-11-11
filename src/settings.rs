//! Settings of the program itself, kept in ~/.config/trindle-scan/settings.conf and saved as they are changed.

use crate::profile::Size;
use std::path::{Path, PathBuf};
use trindle_toolkit::t;

pub fn private_folder(dir: &Path) -> bool {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    let mut b = std::fs::DirBuilder::new();
    b.recursive(true).mode(0o700);
    let _ = b.create(dir);
    match std::fs::symlink_metadata(dir) {
        Ok(m) => m.is_dir() && m.uid() == unsafe { libc::geteuid() } && m.mode() & 0o077 == 0,
        Err(_) => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Units {
    Mm,
    Inches,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub folder: String,

    pub pattern: String,

    pub thumb: u32,
    pub units: Units,

    pub temp_folder: String,

    pub remove_saved: bool,

    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
    pub password: String,
    pub split_every: usize,
}

pub const SPLITS: [usize; 6] = [0, 2, 5, 10, 20, 50];

pub const THUMB_MIN: u32 = 90;
pub const THUMB_MAX: u32 = 300;

pub const BASE_DPI: u32 = 300;

pub fn default_pattern() -> String {
    t!("scan-profile-default-pattern", date = "{year}-{month}-{day}", time = "{hour}.{minute}.{second}")
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            folder: String::new(),
            pattern: default_pattern(),
            thumb: 150,
            units: Units::Mm,
            temp_folder: String::new(),
            remove_saved: false,
            title: String::new(),
            author: String::new(),
            subject: String::new(),
            keywords: String::new(),
            password: String::new(),
            split_every: 0,
        }
    }
}

impl Settings {
    pub fn path() -> Option<PathBuf> {
        crate::profile::Book::path().map(|p| p.with_file_name("settings.conf"))
    }

    pub fn load(path: &Path) -> Settings {
        let mut s = Settings::default();
        let Ok(text) = std::fs::read_to_string(path) else { return s };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match k {
                "folder" => s.folder = v.into(),
                "pattern" if !v.is_empty() => s.pattern = v.into(),
                "thumb" => s.thumb = v.parse().unwrap_or(150).clamp(THUMB_MIN, THUMB_MAX),
                "units" => s.units = if v == "in" { Units::Inches } else { Units::Mm },
                "temp_folder" => s.temp_folder = v.into(),
                "remove_saved" => s.remove_saved = v == "yes",
                "title" => s.title = v.into(),
                "author" => s.author = v.into(),
                "subject" => s.subject = v.into(),
                "keywords" => s.keywords = v.into(),
                "password" => s.password = v.into(),
                "split_every" => s.split_every = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        s
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let out = format!(
            "folder={}\npattern={}\nthumb={}\nunits={}\ntemp_folder={}\nremove_saved={}\ntitle={}\nauthor={}\nsubject={}\nkeywords={}\npassword={}\nsplit_every={}\n",
            one_line(&self.folder),
            one_line(&self.pattern),
            self.thumb,
            if self.units == Units::Inches { "in" } else { "mm" },
            one_line(&self.temp_folder),
            if self.remove_saved { "yes" } else { "no" },
            one_line(&self.title),
            one_line(&self.author),
            one_line(&self.subject),
            one_line(&self.keywords),
            one_line(&self.password),
            self.split_every,
        );
        trindle_common::files::write_private(path, out.as_bytes())
    }

    pub fn keep_folder(&self) -> PathBuf {
        if !self.temp_folder.is_empty() {
            return Path::new(&self.temp_folder).join("trindle-scan-pages");
        }
        let base = trindle_common::files::runtime_dir().unwrap_or_else(std::env::temp_dir);
        base.join("trindle-scan-pages")
    }

    pub fn pdf_options(&self) -> crate::pdf::Options {
        crate::pdf::Options {
            title: if self.title.is_empty() { t!("scan-settings-pdf-default-title").into() } else { self.title.clone() },
            author: self.author.clone(),
            subject: self.subject.clone(),
            keywords: self.keywords.clone(),
            password: self.password.clone(),
        }
    }

    pub fn size_words(&self, size: Size) -> String {
        let Some((w, h)) = size.area() else { return size.words().to_string() };
        match self.units {
            Units::Mm => {
                t!("scan-settings-size-mm", name = size.words(), w = &format!("{}", (w as f32 * 25.4 / BASE_DPI as f32).round()), h = &format!("{}", (h as f32 * 25.4 / BASE_DPI as f32).round()))
            }
            Units::Inches => t!("scan-settings-size-in", name = size.words(), w = &format!("{:.1}", w as f32 / BASE_DPI as f32), h = &format!("{:.1}", h as f32 / BASE_DPI as f32)),
        }
    }
}

fn one_line(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_clamp() {
        crate::words::ready();
        let dir = crate::testdir::tmp("settings");
        let path = dir.join("settings.conf");
        let s = Settings {
            folder: "/home/john/Scans".into(),
            pattern: "Doc {counter:3}".into(),
            thumb: 220,
            units: Units::Inches,
            temp_folder: "/var/tmp".into(),
            remove_saved: true,
            title: "Bills".into(),
            author: "John Smith".into(),
            subject: "2026".into(),
            keywords: "a, b".into(),
            password: "open sesame".into(),
            split_every: 10,
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        std::fs::write(&path, "thumb=9999\nunits=junk\n").unwrap();
        let l = Settings::load(&path);
        assert_eq!(l.thumb, THUMB_MAX);
        assert_eq!(l.units, Units::Mm);
    }

    #[test]
    fn page_sizes_follow_the_units() {
        crate::words::ready();
        let mut s = Settings::default();
        assert_eq!(s.size_words(Size::A4), "A4, 210 by 297 mm");
        s.units = Units::Inches;
        assert_eq!(s.size_words(Size::Letter), "Letter, 8.5 by 11.0 in");
        assert_eq!(s.size_words(Size::Largest), "Largest the scanner can do");
    }
}
