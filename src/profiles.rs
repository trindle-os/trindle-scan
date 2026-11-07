//! Changing the scanning profiles.

use super::*;

impl Scan {

    pub(super) fn follow_profile(&mut self) {
        let Some(p) = self.book.now().cloned() else { return };
        if p.scanner.is_empty() && p.host.is_empty() {
            return;
        }
        let i = self.scanners.iter().position(|s| s.name == p.scanner).or_else(|| self.scanners.iter().position(|s| s.host == p.host && s.port == p.port));
        match i {
            Some(i) if self.chosen != Some(i) => self.choose(i, false),
            Some(_) => {}
            None if !p.host.is_empty() => {
                self.scanners.push(Scanner { name: p.scanner.clone(), host: p.host.clone(), port: p.port, root: p.root.clone() });
                let i = self.scanners.len() - 1;
                self.remembered = Some(i);
                self.choose(i, false);
            }
            None => {}
        }
    }

    pub(super) fn edit(&mut self, e: Edit) {
        let cur = self.book.current;
        let name_free = match &e {
            Edit::Name(n) => self.book.name_free(n, cur),
            _ => true,
        };
        let Some(p) = self.book.profiles.get_mut(cur) else { return };
        self.name_error = None;
        match e {
            Edit::Name(n) => {
                if name_free {
                    p.name = n.trim().to_string();
                } else {
                    self.name_error = Some(if n.trim().is_empty() { t!("scan-profiles-error-name-empty").into() } else { t!("scan-profiles-error-name-taken").into() });
                    return;
                }
            }
            Edit::Source(i) => p.source = [Source::Glass, Source::Feeder, Source::FeederBoth, Source::FeederManualBoth][i.min(3)],
            Edit::Size(i) => p.size = Size::ALL[i.min(Size::ALL.len() - 1)],
            Edit::Dpi(i) => p.dpi = DPIS[i.min(DPIS.len() - 1)],
            Edit::Colour(i) => p.colour = [Colour::Full, Colour::Grey, Colour::BlackWhite][i.min(2)],
            Edit::Threshold(v) => p.threshold = v as i32,
            Edit::Brightness(v) => p.brightness = v as i32,
            Edit::Contrast(v) => p.contrast = v as i32,
            Edit::Rotate(i) => p.rotate = [0, 90, 180, 270][i.min(3)],
            Edit::SkipBlank(b) => p.skip_blank = b,
            Edit::AskMore(b) => p.ask_more = b,
            Edit::AutoSave(b) => p.auto_save = b,
            Edit::Pattern(t) => p.pattern = t,
            Edit::Format(i) => p.format = Format::ALL[i.min(3)],
            Edit::ClearAfter(b) => p.clear_after = b,
            Edit::Trim(b) => p.trim = b,
            Edit::Straighten(b) => p.straighten = b,
            Edit::Batch(i) => p.batch = i.min(2) as u8,
            Edit::Runs(i) => p.runs = RUNS[i.min(RUNS.len() - 1)],
            Edit::Pause(i) => p.pause = PAUSES[i.min(PAUSES.len() - 1)],
            Edit::OneFile(b) => p.one_file = b,
            Edit::Searchable(b) => p.searchable = b,
            Edit::OcrLang(i) => set_first_language(p, &self.ocr_langs, i),
            Edit::OcrLang2(i) => set_second_language(p, &self.ocr_langs, i),
            Edit::ResetTone => {
                p.brightness = 0;
                p.contrast = 0;
            }
        }
        self.persist();
    }

    pub(super) fn update_profiles(&mut self, msg: Msg) {
        match msg {
            Msg::Profiles => self.screen = if self.book.profiles.is_empty() { Screen::Presets } else { Screen::Profiles },
            Msg::UseProfile(i) => {
                self.book.current = i;
                self.persist();
                self.follow_profile();
                self.screen = Screen::Main;
            }
            Msg::NewProfile => self.screen = Screen::Presets,
            Msg::Preset(i) => {
                let mut p = Profile::presets().remove(i);
                p.name = self.book.unique(&p.name);
                p.folder = self.settings.folder.clone();
                p.pattern = self.settings.pattern.clone();
                if let Some(s) = self.scanner() {
                    p.scanner = s.name.clone();
                    p.host = s.host.clone();
                    p.port = s.port;
                    p.root = s.root.clone();
                }
                self.book.add(p);
                self.persist();
                self.screen = Screen::Main;

                if self.book.profiles.len() == 1 {
                    self.begin_scan();
                }
            }
            Msg::EditProfile(i) => {
                self.book.current = i;
                self.persist();
                self.name_error = None;
                self.ocr_langs = ocr::Engine::find().map(|e| e.languages()).unwrap_or_default();
                self.screen = Screen::Editor(i);
            }
            Msg::DuplicateProfile(i) => {
                self.book.duplicate(i);
                self.persist();
            }
            Msg::DeleteProfile(i) => {
                self.book.delete(i);
                self.persist();
                if self.book.profiles.is_empty() {
                    self.screen = Screen::Main;
                }
            }
            Msg::MoveProfile(i, by) => {
                self.book.move_by(i, by);
                self.persist();
            }
            Msg::DefaultProfile(i) => {
                self.book.default = i;
                self.persist();
            }
            Msg::Edit(e) => self.edit(e),
            _ => {}
        }
    }
}

fn set_first_language(p: &mut Profile, langs: &[String], i: usize) {
    let second = p.ocr_lang.split('+').nth(1).map(str::to_string);
    if let Some(l) = langs.get(i) {
        p.ocr_lang = match second.filter(|s| s != l) {
            Some(s) => format!("{l}+{s}"),
            None => l.clone(),
        };
    }
}

fn set_second_language(p: &mut Profile, langs: &[String], i: usize) {
    let first = p.ocr_lang.split('+').next().unwrap_or("eng").to_string();
    p.ocr_lang = match i.checked_sub(1).and_then(|k| langs.iter().filter(|l| **l != first).nth(k)) {
        Some(l) => format!("{first}+{l}"),
        None => first,
    };
}
