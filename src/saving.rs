//! Saving pages as PDF or pictures, opening pictures, and the picker and folder questions that go with them.

use super::*;

impl Scan {
    pub(super) fn chosen_pages(&self) -> Vec<usize> {
        let sel = self.list.selected();
        if sel.is_empty() { (0..self.list.pages.len()).collect() } else { sel }
    }

    pub(super) fn add_text_note(&mut self) {
        if !self.text_note.is_empty() {
            self.note = t!("scan-ocr-note-joined", saved = &self.note, text = &self.text_note);
            self.text_note.clear();
        }
    }

    pub(super) fn mark_saved(&mut self) {
        for i in self.chosen_pages() {
            self.list.pages[i].saved = true;
        }
    }

    pub(super) fn cuts(&self, which: &[usize]) -> Vec<Cut> {
        which.iter().map(|&i| self.list.pages[i].cut()).collect()
    }

    pub(super) fn ask_save(&mut self) {
        if self.list.pages.is_empty() || self.saving {
            return;
        }
        let format = self.book.now().map(|p| p.format).unwrap_or(Format::Pdf);
        let Some(bus) = self.bus.clone() else {
            self.open_browse(if format == Format::Pdf { Purpose::SavePdf } else { Purpose::SaveFolder });
            return;
        };
        let post = self.post.clone();
        let bus = &bus;
        self.asking = if format == Format::Pdf {
            let ask = Ask {
                title: t!("scan-browse-title-save-pdf").into(),
                folder: self.save_start(),
                name: Some(format!("{}.pdf", t!("scan-browse-default-name"))),
                filters: vec![(t!("scan-profile-format-pdf").into(), vec!["*.pdf".into()])],
            };
            chooser::save(bus, &self.events, ask, move |p| {
                if let Some(p) = p {
                    post.send(Event::SaveTo(p));
                }
            })
        } else {
            let ask = Ask { title: t!("scan-browse-title-save-folder").into(), folder: self.save_start(), name: None, filters: vec![] };
            chooser::open_folder(bus, &self.events, ask, move |p| {
                if let Some(p) = p {
                    post.send(Event::SaveIn(p));
                }
            })
        };
        if self.asking.is_none() {
            self.open_browse(if format == Format::Pdf { Purpose::SavePdf } else { Purpose::SaveFolder });
        }
    }

    pub(super) fn open_browse(&mut self, purpose: Purpose) {
        if self.screen != Screen::Browse {
            self.browse_back = self.screen;
        }
        self.browse = Some(Browse::new(purpose, self.save_start(), &self.roots));
        self.screen = Screen::Browse;
    }

    pub(super) fn browse_update(&mut self, m: BrowseMsg) {
        let Some(mut b) = self.browse.take() else { return };
        let mut keep = true;
        match m {
            BrowseMsg::Pick(i) => b.pick(i, &self.roots),
            BrowseMsg::Up => b.up(&self.roots),
            BrowseMsg::Name(t) => b.name = t,
            BrowseMsg::NewName(t) => b.new_folder = t,
            BrowseMsg::MakeFolder => b.make_folder(),
            BrowseMsg::Cancel => keep = false,
            BrowseMsg::Confirm => {
                if let Some(got) = b.confirm() {
                    let first = got[0].clone();
                    let ev = match b.purpose {
                        Purpose::SavePdf => Event::SaveTo(first),
                        Purpose::SaveFolder => Event::SaveIn(first),
                        Purpose::Pictures => Event::Opened(got),
                        Purpose::ProfileFolder => Event::Folder(first),
                        Purpose::SettingsFolder => Event::SettingsFolder(first, false),
                        Purpose::TempFolder => Event::SettingsFolder(first, true),
                    };
                    self.screen = self.browse_back;
                    self.event(ev);
                    return;
                }
            }
        }
        if keep {
            self.browse = Some(b);
        } else {
            self.screen = self.browse_back;
        }
    }

    pub(super) fn save_start(&self) -> Option<PathBuf> {
        self.last_saved_dir.clone().or_else(|| if self.settings.folder.is_empty() { None } else { Some(PathBuf::from(&self.settings.folder)) })
    }

    pub(super) fn ask_folder(&mut self, purpose: Purpose, title: &str, make: impl FnOnce(PathBuf) -> Event + 'static) {
        let Some(bus) = &self.bus else {
            self.open_browse(purpose);
            return;
        };
        let post = self.post.clone();
        let ask = Ask { title: title.into(), folder: None, name: None, filters: vec![] };
        self.asking = chooser::open_folder(bus, &self.events, ask, move |p| {
            if let Some(p) = p {
                post.send(make(p));
            }
        });
    }

    fn open_pictures(&mut self) {
        let Some(bus) = &self.bus else {
            self.open_browse(Purpose::Pictures);
            return;
        };
        let post = self.post.clone();
        let ask = Ask {
            title: t!("scan-browse-title-pictures").into(),
            folder: self.save_start(),
            name: None,
            filters: vec![(t!("scan-main-filter-pictures").into(), vec!["*.jpg".into(), "*.jpeg".into(), "*.png".into(), "*.tif".into(), "*.tiff".into()])],
        };
        self.asking = chooser::open_many(bus, &self.events, ask, move |p| {
            if !p.is_empty() {
                post.send(Event::Opened(p));
            }
        });
    }

    fn recover(&mut self) {
        let Some(d) = self.keep_dir.clone() else { return };
        let (post, post2) = (self.post.clone(), self.post.clone());
        worker(
            move || {
                let mut files: Vec<PathBuf> = std::fs::read_dir(&d).map(|r| r.flatten().map(|e| e.path()).filter(|p| p.extension().map(|x| x == "jpg").unwrap_or(false)).collect()).unwrap_or_default();
                files.sort();
                let mut items = Vec::new();
                let mut old = Vec::new();
                for f in files {
                    if let Ok(bytes) = std::fs::read(&f)
                        && let Ok(img) = image::load_from_memory(&bytes)
                    {
                        items.push(pages::loaded(bytes, &img));
                    }
                    old.push(f);
                }
                post.send(Event::Recovered(items, old));
            },
            move || {
                post2.send(Event::Recovered(Vec::new(), Vec::new()));
            },
        );
    }

    fn open_folder(&mut self) {
        if let Some(d) = &self.last_saved_dir {
            let post = self.post.clone();
            match std::process::Command::new("xdg-open").arg(d).spawn() {
                Ok(mut child) => {
                    std::thread::spawn(move || {
                        if !child.wait().map(|s| s.success()).unwrap_or(false) {
                            post.send(Event::NoOpen);
                        }
                    });
                }
                Err(_) => self.problem = Some(t!("scan-main-error-open-folder").into()),
            }
        }
    }

    pub(super) fn update_saving(&mut self, msg: Msg) {
        match msg {
            Msg::CloseSave => {
                self.screen = Screen::Main;
                self.ask_save();
            }
            Msg::Save => self.ask_save(),
            Msg::OpenPictures => self.open_pictures(),
            Msg::Recover => self.recover(),
            Msg::ForgetRecovery => {
                self.clear_keep();
                if let Some(d) = &self.keep_dir {
                    settings::private_folder(d);
                }
                self.recoverable = 0;
            }
            Msg::ChooseFolder => self.ask_folder(Purpose::ProfileFolder, t!("scan-browse-title-profile-folder"), Event::Folder),
            Msg::AskSettingsFolder(temp) => {
                let (purpose, title) = if temp { (Purpose::TempFolder, t!("scan-browse-title-temp-folder")) } else { (Purpose::SettingsFolder, t!("scan-browse-title-profile-folder")) };
                self.ask_folder(purpose, title, move |p| Event::SettingsFolder(p, temp));
            }
            Msg::OpenFolder => self.open_folder(),
            _ => {}
        }
    }

    fn save_to(&mut self, path: PathBuf) {
        let cuts = self.cuts(&self.chosen_pages());
        self.mark_saved();
        self.saving = true;
        let (post, post2) = (self.post.clone(), self.post.clone());
        let (opt, every) = (self.settings.pdf_options(), self.settings.split_every);
        let (searchable, language) = self.book.now().map(|p| (p.searchable, p.ocr_lang.clone())).unwrap_or_default();
        self.stop_reading = Arc::new(AtomicBool::new(false));
        let stop = self.stop_reading.clone();
        worker(
            move || {
                let mut pages = match pdf_parts(&cuts) {
                    Ok(p) => p,
                    Err(e) => {
                        post.send(Event::Saved(Err(e)));
                        return;
                    }
                };
                if searchable && !give_text(&mut pages, &language, &post, &stop) {
                    return;
                }
                let dir = path.parent().map(|d| d.to_path_buf()).unwrap_or_default();
                let r = write_pdfs(&pages, &opt, every, &path, false).map(|files| match files.as_slice() {
                    [one] => SavedTo { dir, words: one.display().to_string() },
                    _ => SavedTo { dir, words: t!("scan-main-save-files", path = &path.display().to_string(), n = files.len()) },
                });
                post.send(Event::Saved(r.map_err(|e| e.to_string())));
            },
            move || {
                post2.send(Event::Saved(Err(crashed())));
            },
        );
    }

    fn save_in(&mut self, dir: PathBuf) {
        let which = self.chosen_pages();
        let cuts = self.cuts(&which);
        let fmt = self.book.now().map(|p| p.format).unwrap_or(Format::Jpeg);
        self.mark_saved();
        self.saving = true;
        let (post, post2) = (self.post.clone(), self.post.clone());
        worker(
            move || {
                let r = picture_parts(&cuts).and_then(|pics| {
                    naming::write_pictures(&pics, fmt, &dir, t!("scan-browse-default-name")).map(|_| SavedTo { words: dir.display().to_string(), dir: dir.clone() }).map_err(|e| e.to_string())
                });
                post.send(Event::Saved(r));
            },
            move || {
                post2.send(Event::Saved(Err(crashed())));
            },
        );
    }

    fn saved(&mut self, r: Result<SavedTo, String>) {
        self.saving = false;
        self.reading = None;
        match r {
            Ok(to) => {
                self.last_saved_dir = Some(to.dir);
                self.note = t!("scan-main-note-saved", path = &to.words);
                self.add_text_note();
                if self.settings.remove_saved {
                    self.list.mark();
                    self.list.pages.retain(|p| !p.saved);
                }
            }
            Err(e) => {
                for p in &mut self.list.pages {
                    p.saved = false;
                }
                self.problem = Some(t!("scan-main-error-save", why = &e));
            }
        }
    }

    pub(super) fn event_saving(&mut self, e: Event) {
        match e {
            Event::SaveTo(path) => self.save_to(path),
            Event::SaveIn(dir) => self.save_in(dir),
            Event::Reading(i, n) => {
                self.reading = Some((i, n));
                self.announce = Some(t!("scan-ocr-reading", i = i, n = n));
            }
            Event::TextNote(w) => self.text_note = w,
            Event::SaveCancelled => {
                self.saving = false;
                self.reading = None;
                self.text_note.clear();
                for p in &mut self.list.pages {
                    p.saved = false;
                }
                self.problem = Some(t!("scan-ocr-cancelled").into());
            }
            Event::Saved(r) => self.saved(r),
            Event::AutoSaved(r, ids) => match r {
                Ok(p) => {
                    self.reading = None;
                    self.last_saved_dir = Some(PathBuf::from(&p));
                    self.note = if self.summary.is_empty() { t!("scan-main-note-saved", path = &p) } else { t!("scan-main-note-saved-summary", summary = &self.summary, path = &p) };
                    self.add_text_note();
                    let clear = self.book.now().map(|p| p.clear_after).unwrap_or(false);
                    if clear {
                        self.list.pages.retain(|p| !ids.contains(&p.id));
                        self.list.forget_history();
                    } else {
                        for p in self.list.pages.iter_mut().filter(|p| ids.contains(&p.id)) {
                            p.saved = true;
                        }
                    }
                }
                Err(e) => {
                    self.reading = None;
                    self.problem = Some(t!("scan-main-error-autosave", why = &e));
                }
            },
            _ => {}
        }
    }

    fn add_pages(&mut self, items: Vec<Loaded>) -> Vec<u32> {
        items
            .into_iter()
            .map(|(jpeg, thumb, w, h, grey)| {
                let id = self.list.new_id();
                let pg = Page::new(id, jpeg, thumb, w, h, grey);
                self.keep(&pg);
                self.list.pages.push(pg);
                id
            })
            .collect()
    }

    fn recovered(&mut self, items: Vec<Loaded>, old: Vec<PathBuf>) {
        self.list.mark();
        self.recoverable = 0;
        let now: Vec<String> = self.add_pages(items).into_iter().map(|id| format!("{id:06}.jpg")).collect();
        let recovered = now.len();

        let kept = self.keep_dir.as_ref().is_some_and(|d| now.iter().all(|n| d.join(n).is_file()));
        if kept {
            for f in old {
                if !f.file_name().is_some_and(|n| now.iter().any(|x| n == x.as_str())) {
                    let _ = std::fs::remove_file(f);
                }
            }
        }
        self.say_unkept();
        self.note = t!("scan-main-note-recovered", n = recovered);
    }

    pub(super) fn event_files(&mut self, e: Event) {
        match e {
            Event::Opened(paths) => {
                let (post, post2) = (self.post.clone(), self.post.clone());
                worker(
                    move || {
                        let (items, failed) = load_pictures(paths);
                        post.send(Event::Loaded(items, failed));
                    },
                    move || {
                        post2.send(Event::Loaded(Vec::new(), 1));
                    },
                );
            }
            Event::Loaded(items, failed) => {
                let added = items.len();
                if added > 0 {
                    self.list.mark();
                }
                self.add_pages(items);
                self.note = if failed > 0 { t!("scan-main-note-added-failed", n = added, failed = failed) } else { t!("scan-main-note-added", n = added) };
                self.say_unkept();
                self.announce = Some(self.note.clone());
            }
            Event::Languages(l) => self.ocr_langs = l,
            Event::NoOpen => self.problem = Some(t!("scan-main-error-open-folder").into()),
            Event::Recovered(items, old) => self.recovered(items, old),
            Event::SettingsFolder(dir, temp) => {
                let d = dir.display().to_string();
                if temp {
                    self.settings.temp_folder = d;
                } else {
                    self.settings.folder = d;
                }
                self.persist_settings();
            }
            Event::Folder(dir) => {
                let cur = self.book.current;
                if let Some(p) = self.book.profiles.get_mut(cur) {
                    p.folder = dir.display().to_string();
                }
                self.persist();
            }
            _ => {}
        }
    }
}

pub(super) fn give_text(pages: &mut [pdf::PdfPage], language: &str, post: &Post<Event>, stop: &AtomicBool) -> bool {
    let Some(engine) = ocr::Engine::find() else {
        post.send(Event::TextNote(t!("scan-ocr-missing").to_string()));
        return true;
    };
    match ocr::read_pages(
        &engine,
        pages,
        language,
        |i, n| {
            post.send(Event::Reading(i, n));
        },
        &|| stop.load(Ordering::Relaxed),
    ) {
        Ok(failed) if failed.is_empty() => true,
        Ok(failed) => {
            let list = failed.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ");
            post.send(Event::TextNote(t!("scan-ocr-unread", n = failed.len(), list = &list)));
            true
        }
        Err(_) => {
            post.send(Event::SaveCancelled);
            false
        }
    }
}

pub(super) fn write_pdfs(pages: &[pdf::PdfPage], opt: &pdf::Options, every: usize, path: &std::path::Path, exclusive: bool) -> std::io::Result<Vec<PathBuf>> {
    write_pdfs_using(pages, |c, w| pdf::write(c, opt, w), every, path, exclusive)
}

pub(super) fn write_pdfs_using(
    pages: &[pdf::PdfPage],
    write: impl Fn(&[pdf::PdfPage], std::io::BufWriter<&std::fs::File>) -> std::io::Result<()>,
    every: usize,
    path: &std::path::Path,
    exclusive: bool,
) -> std::io::Result<Vec<PathBuf>> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let dir = path.parent().map(|d| d.to_path_buf()).unwrap_or_default();
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| t!("scan-browse-default-name").into());
    let chunks: Vec<&[pdf::PdfPage]> = if every == 0 || pages.len() <= every { vec![pages] } else { pages.chunks(every).collect() };
    let many = chunks.len() > 1;
    let mut files = Vec::new();
    let done = write_chunks(&chunks, write, path, (&dir, &stem), exclusive, &mut files);
    if done.is_err() && (many || exclusive) {
        for f in &files {
            let _ = std::fs::remove_file(f);
        }
    }
    done.map(|_| files)
}

fn write_chunks(
    chunks: &[&[pdf::PdfPage]],
    write: impl Fn(&[pdf::PdfPage], std::io::BufWriter<&std::fs::File>) -> std::io::Result<()>,
    path: &std::path::Path,
    (dir, stem): (&std::path::Path, &str),
    exclusive: bool,
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    let many = chunks.len() > 1;
    for (i, c) in chunks.iter().enumerate() {
        if !many && !exclusive {
            replace_file(path, |f| write(c, std::io::BufWriter::new(f)))?;
            files.push(path.to_path_buf());
            continue;
        }
        let (target, f) = if many { naming::claim_name(dir, &t!("scan-main-part-name", stem = stem, n = i + 1), "pdf")? } else { naming::claim_name(dir, stem, "pdf")? };
        files.push(target);
        write(c, std::io::BufWriter::new(&f))?;
        f.sync_all()?;
    }
    Ok(())
}

pub(super) fn replace_file(path: &std::path::Path, fill: impl FnOnce(&std::fs::File) -> std::io::Result<()>) -> std::io::Result<()> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let temp = path.with_file_name(format!(".{name}.part"));
    let _ = std::fs::remove_file(&temp);
    let f = std::fs::OpenOptions::new().write(true).create_new(true).open(&temp)?;
    let r = fill(&f).and_then(|_| f.sync_all()).and_then(|_| std::fs::rename(&temp, path));
    if r.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    r
}

pub(super) fn pdf_parts(cuts: &[Cut]) -> Result<Vec<pdf::PdfPage>, String> {
    cuts.iter()
        .map(|c| {
            let (jpeg, width, height, picture) = c.final_with_picture(c.bw)?;
            let mono = match picture {
                Some(img) => Some(pdf::mono_of(&img).map_err(|e| e.to_string())?),
                None => None,
            };
            let f = 72.0 / c.dpi.max(1) as f32;
            Ok(pdf::PdfPage { words: Vec::new(), jpeg, mono, width, height, grey: c.grey, page: (width as f32 * f, height as f32 * f) })
        })
        .collect()
}

pub(super) fn picture_parts(cuts: &[Cut]) -> Result<Vec<(Vec<u8>, bool)>, String> {
    cuts.iter().map(|c| Ok((c.final_jpeg()?.0.as_ref().clone(), c.grey))).collect()
}

pub(super) fn auto_save(p: &Profile, cuts: &[Cut], opt: &pdf::Options, every: usize, post: &Post<Event>, stop: &AtomicBool) -> Option<Result<String, String>> {
    if p.format == Format::Pdf {
        let mut pages = match pdf_parts(cuts) {
            Ok(x) => x,
            Err(e) => return Some(Err(e)),
        };
        if p.searchable && !give_text(&mut pages, &p.ocr_lang, post, stop) {
            return None;
        }
        Some(save_to_folder(p, Some(&pages), &[], opt, every).map_err(|e| e.to_string()))
    } else {
        Some(picture_parts(cuts).and_then(|pics| save_to_folder(p, None, &pics, opt, every).map_err(|e| e.to_string())))
    }
}

pub(super) fn save_to_folder(p: &Profile, pdf_pages: Option<&[pdf::PdfPage]>, pics: &[(Vec<u8>, bool)], opt: &pdf::Options, every: usize) -> std::io::Result<String> {
    let mut dir = PathBuf::from(&p.folder);
    let parts: Vec<&str> = p.pattern.split('/').filter(|s| !s.is_empty() && *s != "." && *s != "..").collect();
    let (last, folders) = parts.split_last().map(|(l, f)| (*l, f.to_vec())).unwrap_or((t!("scan-browse-default-name"), vec![]));
    let clock = naming::now();
    for f in folders {
        let part = naming::name_from(f, &p.name, &clock, 1);
        if part != "." && part != ".." {
            dir.push(part);
        }
    }
    std::fs::create_dir_all(&dir)?;
    let counter = naming::counter_prefix(last, &p.name, &clock).map(|prefix| naming::next_counter(&dir, &prefix)).unwrap_or(1);
    let stem = naming::name_from(last, &p.name, &clock, counter);
    match pdf_pages {
        Some(pages) => {
            write_pdfs(pages, opt, every, &dir.join(format!("{stem}.pdf")), true)?;
        }
        None => {
            naming::write_pictures(pics, p.format, &dir, &stem)?;
        }
    }
    Ok(dir.display().to_string())
}

pub(super) fn load_pictures(paths: Vec<PathBuf>) -> (Vec<Loaded>, usize) {
    let mut items = Vec::new();
    let mut failed = 0;
    for p in paths {
        let Ok(img) = image::open(&p) else {
            failed += 1;
            continue;
        };
        let Ok(jpeg) = tidy::jpeg(&img) else {
            failed += 1;
            continue;
        };
        items.push(pages::loaded(jpeg, &img));
    }
    (items, failed)
}
