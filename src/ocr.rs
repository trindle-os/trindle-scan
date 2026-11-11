//! Reading the text of a page with Tesseract, run as its own program, so a saved PDF can be searched.

use crate::pdf::{PdfPage, Word};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const PAGE_SECONDS: u64 = 180;

pub struct Engine {
    exe: PathBuf,

    data: Option<PathBuf>,
}

fn search_path() -> Option<std::ffi::OsString> {
    #[cfg(test)]
    if let Some(p) = TEST_PATH.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return Some(p);
    }
    std::env::var_os("PATH")
}

#[cfg(test)]
static TEST_PATH: std::sync::Mutex<Option<std::ffi::OsString>> = std::sync::Mutex::new(None);

#[cfg(test)]
pub fn set_test_path(path: Option<&std::ffi::OsStr>) {
    *TEST_PATH.lock().unwrap_or_else(|e| e.into_inner()) = path.map(Into::into);
}

fn on_path(path: &std::ffi::OsStr) -> Option<PathBuf> {
    std::env::split_paths(path).filter(|d| d.is_absolute()).map(|d| d.join("tesseract")).find(|p| p.is_file())
}

#[derive(Debug, PartialEq)]
pub enum Fail {
    Stopped,

    Page(String),
}

impl Engine {
    pub fn find() -> Option<Engine> {
        let here = std::env::current_exe().ok()?.parent()?.to_path_buf();
        let own = here.join("tesseract");
        if own.is_file() {
            let data = here.parent().map(|p| p.join("share").join("tessdata")).filter(|d| d.is_dir());
            return Some(Engine { exe: own, data });
        }
        on_path(&search_path()?).map(|exe| Engine { exe, data: None })
    }

    #[cfg(test)]
    pub fn at(exe: PathBuf) -> Engine {
        Engine { exe, data: None }
    }

    fn command(&self) -> Command {
        let mut c = Command::new(&self.exe);

        c.env("LC_ALL", "C");
        if let Some(d) = &self.data {
            c.env("TESSDATA_PREFIX", d);
        }
        c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        c
    }

    pub fn languages(&self) -> Vec<String> {
        let Ok(out) = self.command().arg("--list-langs").output() else { return Vec::new() };
        String::from_utf8_lossy(&out.stdout).lines().skip(1).map(|l| l.trim().to_string()).filter(|l| !l.is_empty() && l != "osd" && valid_language(l)).collect()
    }

    pub fn read(&self, picture: &[u8], language: &str, stop: &dyn Fn() -> bool) -> Result<Vec<Word>, Fail> {
        if !valid_language(language) {
            return Err(Fail::Page(trindle_toolkit::t!("scan-ocr-bad-language").to_string()));
        }
        let mut child = self.command().args(["stdin", "stdout", "-l", language, "tsv"]).spawn().map_err(|e| Fail::Page(e.to_string()))?;
        let mut stdin = child.stdin.take().ok_or_else(|| Fail::Page(String::new()))?;
        let mut stdout = child.stdout.take().ok_or_else(|| Fail::Page(String::new()))?;
        let bytes = picture.to_vec();
        let feeder = std::thread::spawn(move || {
            let _ = stdin.write_all(&bytes);
        });
        let reader = std::thread::spawn(move || {
            let mut b = Vec::new();
            let _ = stdout.read_to_end(&mut b);
            String::from_utf8_lossy(&b).into_owned()
        });
        let end = Instant::now() + Duration::from_secs(PAGE_SECONDS);
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) => {}
                Err(e) => return Err(Fail::Page(e.to_string())),
            }
            if stop() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Fail::Stopped);
            }
            if Instant::now() > end {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Fail::Page(trindle_toolkit::t!("scan-ocr-too-slow").to_string()));
            }
            std::thread::sleep(Duration::from_millis(40));
        };
        let _ = feeder.join();
        let text = reader.join().unwrap_or_default();
        if !status.success() {
            return Err(Fail::Page(trindle_toolkit::t!("scan-ocr-failed").to_string()));
        }
        Ok(parse_tsv(&text))
    }
}

pub fn language_name(code: &str) -> String {
    const NAMES: [(&str, &str); 23] = [
        ("eng", "English"),
        ("fra", "Fran\u{e7}ais"),
        ("deu", "Deutsch"),
        ("spa", "Espa\u{f1}ol"),
        ("ita", "Italiano"),
        ("por", "Portugu\u{ea}s"),
        ("nld", "Nederlands"),
        ("pol", "Polski"),
        ("swe", "Svenska"),
        ("dan", "Dansk"),
        ("nor", "Norsk"),
        ("fin", "Suomi"),
        ("ces", "\u{10c}e\u{161}tina"),
        ("hun", "Magyar"),
        ("ron", "Rom\u{e2}n\u{103}"),
        ("tur", "T\u{fc}rk\u{e7}e"),
        ("ell", "\u{395}\u{3bb}\u{3bb}\u{3b7}\u{3bd}\u{3b9}\u{3ba}\u{3ac}"),
        ("rus", "\u{420}\u{443}\u{441}\u{441}\u{43a}\u{438}\u{439}"),
        ("ukr", "\u{423}\u{43a}\u{440}\u{430}\u{457}\u{43d}\u{441}\u{44c}\u{43a}\u{430}"),
        ("cym", "Cymraeg"),
        ("gle", "Gaeilge"),
        ("cat", "Catal\u{e0}"),
        ("ara", "\u{627}\u{644}\u{639}\u{631}\u{628}\u{64a}\u{629}"),
    ];
    NAMES.iter().find(|(c, _)| *c == code).map(|(_, n)| n.to_string()).unwrap_or_else(|| code.to_string())
}

pub fn valid_language(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '+')
}

pub fn parse_tsv(text: &str) -> Vec<Word> {
    text.lines()
        .filter_map(|line| {
            let c: Vec<&str> = line.split('\t').collect();
            if c.len() < 12 || c[0] != "5" {
                return None;
            }
            let conf: f32 = c[10].parse().ok()?;
            let word = c[11].trim();
            if conf < 0.0 || word.is_empty() {
                return None;
            }
            let n = |i: usize| c[i].parse::<f32>().ok();
            Some(Word { text: word.to_string(), x: n(6)?, y: n(7)?, w: n(8)?, h: n(9)? })
        })
        .collect()
}

pub fn read_pages(engine: &Engine, pages: &mut [PdfPage], language: &str, mut progress: impl FnMut(usize, usize), stop: &dyn Fn() -> bool) -> Result<Vec<usize>, Fail> {
    let n = pages.len();
    let mut failed = Vec::new();
    for (i, page) in pages.iter_mut().enumerate() {
        if stop() {
            return Err(Fail::Stopped);
        }
        progress(i + 1, n);
        match engine.read(&page.jpeg, language, stop) {
            Ok(words) => page.words = words,
            Err(Fail::Stopped) => return Err(Fail::Stopped),
            Err(Fail::Page(_)) => failed.push(i + 1),
        }
    }
    Ok(failed)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn the_engine_is_asked_in_the_plain_locale() {
        let e = Engine::at("tesseract".into());
        let c = e.command();
        let env: Vec<_> = c.get_envs().filter(|(k, v)| *k == "LC_ALL" && *v == Some(std::ffi::OsStr::new("C"))).collect();
        assert_eq!(env.len(), 1);
    }
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Arc;

    pub fn stand_in(dir: &std::path::Path) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let exe = dir.join("tesseract");
        let script = "#!/bin/sh\nif [ \"$1\" = \"--list-langs\" ]; then printf 'List of available languages (3):\\neng\\nfra\\nosd\\n'; exit 0; fi\nin=$(/bin/cat)\ncase \"$in\" in *BAD*) exit 1;; *SLOW*) /bin/sleep 20;; *LATIN*) printf '5\\t1\\t1\\t1\\t1\\t1\\t10\\t20\\t30\\t12\\t96.5\\tcaf\\351\\n'; exit 0;; esac\nprintf 'level\\tpage_num\\tblock_num\\tpar_num\\tline_num\\tword_num\\tleft\\ttop\\twidth\\theight\\tconf\\ttext\\n'\nprintf '5\\t1\\t1\\t1\\t1\\t1\\t10\\t20\\t30\\t12\\t96.5\\tHello\\n'\nprintf '5\\t1\\t1\\t1\\t1\\t2\\t50\\t20\\t30\\t12\\t-1\\t\\n'\nprintf '5\\t1\\t1\\t1\\t1\\t3\\t50\\t20\\t30\\t12\\t90\\tw\\303\\266rld\\n'\n";
        std::fs::write(&exe, script).unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();

        while std::process::Command::new(&exe).arg("--list-langs").output().is_err_and(|e| e.raw_os_error() == Some(26)) {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        exe
    }

    fn page(jpeg: &[u8]) -> PdfPage {
        PdfPage { words: Vec::new(), jpeg: Arc::new(jpeg.to_vec()), mono: None, width: 100, height: 100, grey: false, page: (200.0, 200.0) }
    }

    #[test]
    fn the_table_gives_words_with_boxes_and_leaves_out_unsure_rows() {
        let table = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n4\t1\t1\t1\t1\t0\t5\t5\t90\t20\t-1\t\n5\t1\t1\t1\t1\t1\t10\t20\t30\t12\t96.5\tHello\n5\t1\t1\t1\t1\t2\t50\t20\t30\t12\t-1\t \n";
        assert_eq!(parse_tsv(table), vec![Word { text: "Hello".into(), x: 10.0, y: 20.0, w: 30.0, h: 12.0 }]);
    }

    #[test]
    fn languages_come_from_the_engine_without_the_direction_finder() {
        crate::words::ready();
        let dir = crate::testdir::tmp("ocr");
        let engine = Engine::at(stand_in(&dir));
        assert_eq!(engine.languages(), vec!["eng".to_string(), "fra".to_string()]);
        assert!(engine.read(b"x", "eng; rm", &|| false).is_err(), "a language is not a command line");
    }

    #[test]
    fn pages_are_read_one_by_one_and_a_page_that_fails_is_listed() {
        crate::words::ready();
        let dir = crate::testdir::tmp("ocr");
        let engine = Engine::at(stand_in(&dir));
        let mut pages = vec![page(b"fine"), page(b"BAD page"), page(b"fine")];
        let mut seen = Vec::new();
        let failed = read_pages(&engine, &mut pages, "eng", |i, n| seen.push((i, n)), &|| false).unwrap();
        assert_eq!(failed, vec![2]);
        assert_eq!(seen, vec![(1, 3), (2, 3), (3, 3)]);
        assert_eq!(pages[0].words.len(), 2);
        assert_eq!(pages[0].words[1].text, "w\u{f6}rld");
        assert!(pages[1].words.is_empty());
    }

    #[test]
    fn cancel_stops_the_reading_while_the_engine_works() {
        crate::words::ready();
        let dir = crate::testdir::tmp("ocr");
        let engine = Engine::at(stand_in(&dir));
        let mut pages = vec![page(b"SLOW"), page(b"fine")];
        let asked = std::sync::atomic::AtomicUsize::new(0);
        let stop = || asked.fetch_add(1, std::sync::atomic::Ordering::Relaxed) > 0;
        let began = Instant::now();
        assert_eq!(read_pages(&engine, &mut pages, "eng", |_, _| {}, &stop), Err(Fail::Stopped));
        assert!(began.elapsed() < Duration::from_secs(10), "the running engine was stopped, not waited for");
        assert!(pages[0].words.is_empty());
    }

    #[test]
    fn text_that_is_not_utf8_is_still_read() {
        crate::words::ready();
        let dir = crate::testdir::tmp("ocr");
        let engine = Engine::at(stand_in(&dir));
        let words = engine.read(b"LATIN", "eng", &|| false).unwrap();
        assert_eq!(words.len(), 1);
        assert!(words[0].text.starts_with("caf"));
    }

    #[test]
    fn only_absolute_folders_on_the_path_are_searched() {
        let dir = crate::testdir::tmp("ocr");
        let exe = stand_in(&dir);
        let up = "../".repeat(std::env::current_dir().unwrap().components().count() - 1);
        let relative = PathBuf::from(format!("{up}{}", dir.strip_prefix("/").unwrap().display()));
        assert!(relative.join("tesseract").is_file(), "the relative form reaches the file");
        let bad = std::env::join_paths([PathBuf::new(), relative]).unwrap();
        assert_eq!(on_path(&bad), None);
        assert_eq!(on_path(&std::env::join_paths([dir.to_path_buf()]).unwrap()), Some(exe));
    }
}
