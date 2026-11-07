//! Finding a scanner, running a scan and the batch that follows it.

use super::*;

impl Scan {

    pub(super) fn on_open(&mut self) {
        if self.chosen.is_none() {
            self.screen = Screen::Scanners;
            self.opening = true;
        }
        self.start_search();
    }

    pub(super) fn start_search(&mut self) {
        if self.searching {
            return;
        }
        self.searching = true;
        self.stop_search = Arc::new(AtomicBool::new(false));
        let (post, stop) = (self.post.clone(), self.stop_search.clone());
        let post2 = post.clone();
        worker(move || {
            for f in sane::list() {
                post.send(Event::Found(Scanner { name: f.name, host: format!("{}{}", sane::PREFIX, f.device), port: 0, root: String::new() }));
            }
            let p = post.clone();
            let _ = mdns::search(12, || stop.load(Ordering::Relaxed), move |f| {
                p.send(Event::Found(Scanner { name: f.name, host: f.host, port: f.port, root: f.root }));
            });
            post.send(Event::SearchDone);
        }, move || {
            post2.send(Event::SearchDone);
        });
    }

    pub(super) fn add_scanner(&mut self, s: Scanner) -> usize {
        if let Some(i) = self.scanners.iter().position(|x| x.host == s.host && x.port == s.port) {
            self.scanners[i].root = s.root;
            return i;
        }

        if let Some(i) = self.remembered.filter(|i| self.scanners.get(*i).is_some_and(|x| x.name == s.name)) {
            self.remembered = None;
            self.scanners[i] = s;
            return i;
        }
        self.scanners.push(s);
        self.scanners.len() - 1
    }

    pub(super) fn choose(&mut self, i: usize, remember: bool) {
        self.chosen = Some(i);
        self.caps = None;
        self.problem = None;
        let s = self.scanners[i].clone();
        if remember {
            let cur = self.book.current;
            if let Some(p) = self.book.profiles.get_mut(cur) {
                p.scanner = s.name.clone();
                p.host = s.host.clone();
                p.port = s.port;
                p.root = s.root.clone();
            }
            self.persist();
        }
        let (post, post2, s2) = (self.post.clone(), self.post.clone(), s.clone());
        worker(move || {
            let r = s.capabilities();
            post.send(Event::Caps(s, r));
        }, move || {
            post2.send(Event::Caps(s2, Err(Fail::Other(crashed()))));
        });
    }

    pub(super) fn begin_scan(&mut self) {
        if self.scanning {
            return;
        }
        if self.book.profiles.is_empty() {
            self.screen = Screen::Presets;
            return;
        }
        self.more = false;
        self.no_paper = false;
        self.offline = false;
        if self.batch.runs_done == 0 {
            self.summary.clear();
        }
        let mut prof = self.book.now().cloned().unwrap();
        if self.force_glass {
            prof.source = Source::Glass;
        }
        let (Some(s), Some(c)) = (self.scanner().cloned(), self.caps.clone()) else {
            self.problem = Some(if self.scanners.is_empty() { t!("scan-main-error-no-scanner").into() } else { t!("scan-main-error-wait-caps").into() });
            if self.scanners.is_empty() {
                self.screen = Screen::Scanners;
                self.start_search();
            }
            return;
        };
        let (settings, told) = prof.settings(&c);
        self.note = told.join(" ");
        self.scanning = true;
        self.arrived = 0;
        self.problem = None;
        if self.session.is_none() {
            self.session = Some(Vec::new());
            self.blanks = 0;
        }
        self.stop_scan = Arc::new(AtomicBool::new(false));
        let (post, stop, post2) = (self.post.clone(), self.stop_scan.clone(), self.post.clone());
        let bw = prof.colour == Colour::BlackWhite;
        worker(move || {
            let p = post.clone();
            let r = s.scan(&settings, || stop.load(Ordering::Relaxed), move |jpeg| take_scanned(&p, &prof, bw, settings.dpi, jpeg));
            post.send(Event::ScanDone(r));
        }, move || {
            post2.send(Event::ScanDone(Err(Fail::Other(crashed()))));
        });
    }

    pub(super) fn begin_test(&mut self) {
        if self.testing || self.scanning {
            return;
        }
        self.test_problem = None;
        let (Some(prof), Some(s), Some(c)) = (self.book.now().cloned(), self.scanner().cloned(), self.caps.clone()) else {
            self.test_problem = Some(t!("scan-main-test-choose-scanner").into());
            return;
        };
        let (settings, _) = prof.settings(&c);
        self.testing = true;
        self.test_img = None;
        let (post, post2) = (self.post.clone(), self.post.clone());
        worker(move || {
            let stop = Arc::new(AtomicBool::new(false));
            let (p, flag, stop2) = (post.clone(), Arc::new(AtomicBool::new(false)), stop.clone());
            let r = s.scan(&settings, || stop.load(Ordering::Relaxed), move |jpeg| {
                if flag.swap(true, Ordering::Relaxed) {
                    return;
                }
                stop2.store(true, Ordering::Relaxed);
                match image::load_from_memory(&jpeg) {
                    Ok(img) => {
                        let img = tidy::tidy(&prof, img.clone()).unwrap_or(img);
                        p.send(Event::TestPage(img));
                    }
                    Err(_) => {
                        p.send(Event::Damaged);
                    }
                }
            });
            post.send(Event::TestDone(r));
        }, move || {
            post2.send(Event::TestDone(Err(Fail::Other(crashed()))));
        });
    }

    pub(super) fn after_run(&mut self) {
        let Some(prof) = self.book.now().cloned() else { return };
        let session_pages = self.session.as_ref().map(|s| s.len()).unwrap_or(0);
        let first = self.session.as_ref().and_then(|s| s.first()).and_then(|id| self.list.pages.iter().position(|p| p.id == *id)).unwrap_or(0);
        if prof.source == Source::FeederManualBoth && !self.force_glass {
            match self.turn_over {
                None if self.arrived > 0 => {
                    self.turn_over = Some(session_pages);
                    self.note = t!("scan-main-note-turn-over").into();
                    return;
                }
                Some(fronts) => {
                    self.list.interleave(first, fronts, true);
                    self.turn_over = None;
                }
                None => {}
            }
        }
        if (prof.source == Source::Glass || self.force_glass) && prof.ask_more && prof.batch == 0 && self.arrived > 0 {
            self.more = true;
            self.note = t!("scan-main-note-another-page").into();
            return;
        }
        self.batch_next();
    }

    pub(super) fn batch_next(&mut self) {
        let Some(prof) = self.book.now().cloned() else { return };
        if prof.batch == 0 {
            self.finish_session(true);
            return;
        }
        self.batch.runs_done += 1;
        self.batch.pages += self.arrived;
        let cancelled = self.stop_scan.load(Ordering::Relaxed);
        let more = !cancelled && !self.batch.stop && if prof.batch == 1 { self.batch.runs_done < prof.runs } else { true };
        if more {
            if !prof.one_file {
                self.finish_session(false);
            }
            self.batch.pause_left = Some(prof.pause);
            self.note = t!("scan-main-note-run-done", n = self.batch.runs_done);
        } else {
            self.end_batch();
        }
    }

    pub(super) fn end_batch(&mut self) {
        self.finish_session(false);
        let b = std::mem::take(&mut self.batch);
        self.summary = t!("scan-main-summary-batch", runs = b.runs_done, pages = b.pages);
        self.note = self.summary.clone();
    }

    pub(super) fn tick(&mut self) -> bool {
        if let Some(n) = self.busy {
            if n <= 1 {
                self.busy = None;
                self.problem = Some(t!("scan-main-error-busy-still").into());
            } else {
                self.busy = Some(n - 1);
                self.problem = Some(t!("scan-main-error-busy-retry", n = n - 1));
                if (n - 1) % 5 == 0 {
                    self.begin_scan();
                }
            }
            return true;
        }
        match self.batch.pause_left {
            Some(n) if n > 1 => {
                self.batch.pause_left = Some(n - 1);
                true
            }
            Some(1) => {
                self.batch.pause_left = None;
                self.begin_scan();
                true
            }
            _ => false,
        }
    }

    pub(super) fn finish_session(&mut self, report: bool) {
        self.more = false;
        self.turn_over = None;
        let Some(ids) = self.session.take() else { return };
        let n = ids.len();
        if report {
            self.note = if self.blanks > 0 { t!("scan-main-note-scanned-skipped", n = n, blanks = self.blanks) } else { t!("scan-main-note-scanned", n = n) };
        }
        let Some(prof) = self.book.now().cloned() else { return };
        if prof.auto_save && !prof.folder.is_empty() && n > 0 {
            let idx: Vec<usize> = self.list.pages.iter().enumerate().filter(|(_, p)| ids.contains(&p.id)).map(|(i, _)| i).collect();
            let cuts = self.cuts(&idx);
            let (post, post2, ids2) = (self.post.clone(), self.post.clone(), ids.clone());
            let (opt, every) = (self.settings.pdf_options(), self.settings.split_every);
            self.stop_reading = Arc::new(AtomicBool::new(false));
            let stop = self.stop_reading.clone();
            worker(move || {
                let r = auto_save(&prof, &cuts, &opt, every, &post, &stop);
                match r {
                    Some(r) => post.send(Event::AutoSaved(r, ids)),
                    None => return,
                };
            }, move || {
                post2.send(Event::AutoSaved(Err(crashed()), ids2));
            });
        }
    }

    fn add_address(&mut self) {
        let (host, port) = split_address(self.address.trim());
        if host.is_empty() {
            return;
        }
        let s = Scanner { name: host.clone(), host, port, root: "eSCL".into() };
        let (post, post2) = (self.post.clone(), self.post.clone());
        self.problem = Some(t!("scan-main-note-checking-address").into());
        self.address.clear();
        worker(move || {
            let r = s.capabilities();
            post.send(Event::Checked(r.as_ref().err().map(|f| f.words())));
            if r.is_ok() {
                post.send(Event::Found(s.clone()));
            }
            post.send(Event::Caps(s, r));
        }, move || {
            post2.send(Event::Checked(Some(crashed())));
        });
    }

    pub(super) fn update_scan(&mut self, msg: Msg) {
        match msg {
            Msg::Scan => {
                self.screen = Screen::Main;
                self.begin_scan()
            }
            Msg::CancelScan => self.stop_scan.store(true, Ordering::Relaxed),
            Msg::CancelReading => self.stop_reading.store(true, Ordering::Relaxed),
            Msg::Finish => self.finish_session(true),
            Msg::Scanners => {
                self.opening = false;
                self.screen = Screen::Scanners;
                self.start_search();
            }
            Msg::SearchAgain => self.start_search(),
            Msg::StopSearch => self.stop_search.store(true, Ordering::Relaxed),
            Msg::Pick(i) => {
                self.opening = false;
                self.choose(i, true);
                self.screen = Screen::Main;
            }
            Msg::Address(a) => self.address = a,
            Msg::AddAddress => self.add_address(),
            Msg::SkipPause => {
                self.batch.pause_left = None;
                self.begin_scan();
            }
            Msg::StopBatch => {
                self.batch.stop = true;
                if self.batch.pause_left.take().is_some() {
                    self.end_batch();
                }
            }
            Msg::Digit(n) if n >= 1 && n <= self.book.profiles.len() && !self.scanning => {
                self.book.current = n - 1;
                self.persist();
                self.follow_profile();
                self.begin_scan();
            }
            Msg::StopRetry => {
                self.busy = None;
                self.problem = Some(t!("scan-main-error-stopped-busy").into());
            }
            Msg::TestScan => self.begin_test(),
            Msg::UseGlass => {
                self.force_glass = true;
                self.problem = None;
                self.no_paper = false;
                self.begin_scan();
                self.force_glass = false;
            }
            Msg::TryAgain => self.begin_scan(),
            _ => {}
        }
    }

    fn test_done(&mut self, r: Result<usize, Fail>) {
        self.testing = false;
        if self.test_img.is_none() {
            self.test_problem = Some(match r {
                Err(f) => t!("scan-main-test-failed", why = &f.words()),
                Ok(_) => t!("scan-main-test-no-page").into(),
            });
        }
        self.announce = Some(if self.test_img.is_some() { t!("scan-main-announce-test-done").into() } else { t!("scan-main-announce-test-failed").into() });
    }

    fn page_arrived(&mut self, mut pg: Page, blank: bool) {
        if blank {
            self.blanks += 1;
            self.held_blank.push(pg);
            return;
        }
        if self.arrived == 0 {
            self.list.mark();
            for p in &mut self.list.pages {
                p.selected = false;
            }
        }
        self.arrived += 1;
        pg.selected = true;
        self.keep(&pg);
        if let Some(ids) = &mut self.session {
            ids.push(pg.id);
        }
        self.list.pages.push(pg);
    }

    pub(super) fn scan_done(&mut self, r: Result<usize, Fail>) {
        self.scanning = false;
        if self.session.as_ref().map(|s| s.is_empty()).unwrap_or(true) && !self.held_blank.is_empty() {

            let mut keep = self.held_blank.remove(0);
            keep.selected = true;
            self.keep(&keep);
            if let Some(ids) = &mut self.session {
                ids.push(keep.id);
            }
            self.list.mark();
            self.list.pages.push(keep);
            self.blanks -= 1;
            self.arrived += 1;
        }
        self.held_blank.clear();
        match r {
            Ok(_) => {
                self.busy = None;
                self.after_run();
                self.announce = Some(self.note.clone());
            }
            Err(f) => {
                self.no_paper = f == Fail::NoPaper;
                self.offline = matches!(f, Fail::Offline(_));
                if f != Fail::Busy {
                    self.busy = None;
                }
                if f == Fail::Busy && self.arrived == 0 && self.busy.is_none() {
                    self.busy = Some(60);
                }
                self.problem = Some(if self.arrived > 0 { t!("scan-main-error-with-kept", words = &f.words(), n = self.arrived) } else { f.words() });
                self.announce = self.problem.clone();
                if self.busy.is_none() {
                    self.batch = BatchState::default();
                }
                if self.arrived > 0 {
                    self.finish_session(false);
                }
            }
        }
    }

    pub(super) fn event_scan(&mut self, e: Event) {
        match e {
            Event::TestPage(img) => self.test_img = Some(Rc::new(img)),
            Event::TestDone(r) => self.test_done(r),
            Event::Found(s) => {
                let first = self.scanners.is_empty();
                let i = self.add_scanner(s);
                if first && self.chosen.is_none() {
                    self.choose(i, true);
                } else if self.chosen == Some(i) && self.caps.is_none() {
                    self.choose(i, false);
                }
            }
            Event::SearchDone => {
                self.searching = false;
                if self.opening && self.screen == Screen::Scanners && self.scanners.len() == 1 {
                    self.screen = Screen::Main;
                }
                self.opening = false;
            }
            Event::Caps(s, r) if self.scanner() == Some(&s) => match r {
                Ok(c) => {
                    self.caps = Some(c);
                    self.problem = None;
                }
                Err(f) => self.problem = Some(f.words()),
            },
            Event::Failed(why) => self.note = why,
            Event::Damaged => self.note = t!("scan-main-note-damaged").into(),
            Event::Page { jpeg, thumb, width, height, grey, bw, blank, dpi } => {
                let id = self.list.new_id();
                let mut pg = Page::new(id, jpeg, thumb, width, height, grey);
                pg.dpi = dpi;
                pg.bw = bw;
                self.page_arrived(pg, blank);
            }
            Event::ScanDone(r) => self.scan_done(r),
            Event::Checked(w) if w.is_some() || self.problem.as_deref() == Some(t!("scan-main-note-checking-address")) => {
                self.problem = w;
            }
            _ => {}
        }
    }
}

fn take_scanned(post: &Post<Event>, prof: &Profile, bw: bool, dpi: u32, jpeg: Vec<u8>) {
    let Ok(img) = image::load_from_memory(&jpeg) else {
        post.send(Event::Damaged);
        return;
    };
    if !prof.changes_pixels() {
        return send_page(post, img, Some(jpeg), bw, false, dpi);
    }
    let orig = prof.skip_blank.then(|| img.clone());
    match (tidy::tidy(prof, img), orig) {
        (Some(img), _) => send_page(post, img, None, bw, false, dpi),
        (None, Some(orig)) => send_page(post, orig, Some(jpeg), bw, true, dpi),
        (None, None) => {}
    }
}

pub(super) fn send_page(post: &Post<Event>, img: DynamicImage, jpeg: Option<Vec<u8>>, bw: bool, blank: bool, dpi: u32) {
    let jpeg = match jpeg.map_or_else(|| tidy::jpeg(&img), Ok) {
        Ok(j) => j,
        Err(e) => {
            post.send(Event::Failed(e.to_string()));
            return;
        }
    };
    let (jpeg, thumb, width, height, grey) = pages::loaded(jpeg, &img);
    post.send(Event::Page { jpeg, thumb, width, height, grey, bw, blank, dpi });
}
