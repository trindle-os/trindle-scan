//! The program's own words: every file of share/lang/en is built in as the English every other language falls back to.

use trindle_toolkit::host::{self, Host};

const ENGLISH: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/english.rs"));

struct Words;

impl Host for Words {
    fn words(&self) -> &'static [(&'static str, &'static str)] {
        ENGLISH
    }
}

static WORDS: Words = Words;

pub fn ready() {
    host::set(&WORDS);
}

#[cfg(test)]
pub static PATH_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use trindle_toolkit::lang::check::{Domain, check};
    use trindle_toolkit::lang::{Bundle, Catalogue, with_catalogue};
    use trindle_toolkit::t;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn sources() -> Vec<PathBuf> {
        let mut all: Vec<PathBuf> = std::fs::read_dir(root().join("src")).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "rs")).collect();
        all.sort();
        all
    }

    #[test]
    fn english_and_the_code_agree() {
        let domain = Domain { name: "scan".into(), lang_dir: root().join("share/lang"), sources: sources(), skip: vec![] };
        let report = check(&[domain]);
        assert!(report.problems.is_empty(), "{}", report.text());
        assert!(report.english_keys > 200, "{}", report.english_keys);
    }

    #[test]
    fn the_words_follow_the_language_and_fall_back_word_by_word() {
        super::ready();
        let french = "scan-bar-scan = Numériser\nscan-main-note-added =\n    { $n ->\n        [one] { $n } image ajoutée\n       *[other] { $n } images ajoutées\n    }\n";
        with_catalogue(Catalogue::new(Bundle::from_text("fr", french), trindle_toolkit::lang::english()), || {
            assert_eq!(t!("scan-bar-scan"), "Numériser");
            assert_eq!(t!("scan-bar-save"), "Save", "a word French lacks is English");
            assert_eq!(t!("scan-main-note-added", n = 1), "1 image ajoutée");
            assert_eq!(t!("scan-main-note-added", n = 3), "3 images ajoutées");
        });
        assert_eq!(t!("scan-main-note-added", n = 1), "Added 1 picture");
        assert_eq!(t!("scan-main-note-added", n = 2), "Added 2 pictures");
    }

    const SHOWING: [&str; 22] = [
        "button(",
        "label(",
        "note(",
        ".spoken(",
        "heading(",
        "title(",
        "form_label(",
        "section(",
        "word_button(",
        "main_word(",
        "lead(",
        "entry(",
        "switch(",
        "slider(",
        "drawing(",
        "choice(",
        "one_of(",
        "item(",
        ".detail(",
        "remove(",
        ".announce(",
        "Ask {",
    ];

    const NOT_WORDS: [&str; 2] = ["{year} {month} {day} {hour} {minute} {second} {counter:3} {profile}", "{year} {month} {day} {hour} {minute} {second} {profile}"];

    fn literals(line: &str) -> Vec<String> {
        let chars: Vec<char> = line.chars().collect();
        let (mut out, mut i) = (Vec::new(), 0);
        while i < chars.len() {
            if chars[i] == '\'' && chars.get(i + 1) == Some(&'"') && chars.get(i + 2) == Some(&'\'') {
                i += 3;
            } else if chars[i] == '"' {
                let mut text = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        text.push(chars[i]);
                        i += 1;
                    }
                    if i < chars.len() {
                        text.push(chars[i]);
                    }
                    i += 1;
                }
                out.push(text);
                i += 1;
            } else {
                i += 1;
            }
        }
        out
    }

    fn a_key_or_name(s: &str) -> bool {
        !s.chars().any(|c| c.is_ascii_uppercase() || c == ' ') && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-.:/@%{}=#*".contains(c))
    }

    #[test]
    fn nothing_the_program_shows_is_written_inline() {
        let mut found = Vec::new();
        for file in sources() {
            let name = file.file_name().unwrap().to_string_lossy().into_owned();
            if name == "shots.rs" || name == "words.rs" || name == "tests.rs" {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap();
            let live = text.split("mod tests {").next().unwrap_or("");
            for (n, line) in live.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }
                let quiet = ["eprintln!", "println!", "panic!", "assert", "expect(", "unreachable!", ".context(", "write!(", "writeln!("].iter().any(|q| line.contains(q));
                let showing = SHOWING.iter().any(|c| line.contains(c));
                let mut without_keys = line.to_string();
                while let Some(a) = without_keys.find("t!(\"") {
                    let end = without_keys[a + 4..].find('"').map_or(without_keys.len(), |e| a + 4 + e + 1);
                    without_keys.replace_range(a + 3..end, "");
                }
                for lit in literals(&without_keys) {
                    if NOT_WORDS.contains(&lit.as_str()) || lit.is_empty() {
                        continue;
                    }
                    let has_letters = lit.chars().filter(|c| c.is_ascii_alphabetic()).count() >= 2;
                    let sentence = lit.chars().next().is_some_and(|c| c.is_ascii_uppercase()) && lit.contains(' ') && lit.chars().any(|c| c.is_ascii_lowercase());
                    if (showing && has_letters && !a_key_or_name(&lit)) || (sentence && !quiet) {
                        found.push(format!("{name}:{}: {lit}", n + 1));
                    }
                }
            }
        }
        assert!(found.is_empty(), "words written inline, which belong in share/lang/en:\n{}", found.join("\n"));
    }
}
