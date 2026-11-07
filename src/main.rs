//! The window, the scanner list and the page list: everything the person sees and presses.

mod browse;
mod escl;
mod mdns;
mod pages;
mod naming;
mod ocr;
mod pdf;
mod profile;
mod profiles;
mod sane;
mod saving;
mod scanning;
mod settings;
mod tidy;
mod viewer;
mod views;
mod words;
#[cfg(test)]
mod shots;
#[cfg(test)]
mod testdir;
#[cfg(test)]
mod tests;

use browse::{Browse, Purpose};
use escl::{Caps, Fail, Scanner};
use image::{DynamicImage, GenericImageView};
use pages::{Cut, List, Loaded, Page};
use profile::{Book, Colour, Format, Profile, Size, Source};
use settings::{Settings, Units};
use std::cell::Cell;
use trindle_toolkit::t;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use trindle_toolkit::bus::{Bus, Listen};
use trindle_toolkit::chooser::{self, Ask};
use trindle_toolkit::dnd::Given;
use trindle_toolkit::event::{Loop, Post};
use trindle_toolkit::input::{Input, Keysym, Mods};
use trindle_toolkit::crop::{self, Crop};
use trindle_toolkit::input::Cursor;
use trindle_toolkit::picture;
use trindle_toolkit::zoom::{self, View};
use trindle_toolkit::ui::{App, Shown};
use trindle_toolkit::widget::*;
use trindle_toolkit::window::{Display, Kind};
use saving::*;

enum Event {
    Found(Scanner),
    SearchDone,
    Caps(Scanner, Result<Caps, Fail>),
    Page { jpeg: Vec<u8>, thumb: DynamicImage, width: u32, height: u32, grey: bool, bw: bool, blank: bool, dpi: u32 },
    Damaged,
    ScanDone(Result<usize, Fail>),
    SaveTo(PathBuf),
    SaveIn(PathBuf),
    Failed(String),
    Saved(Result<String, String>),
    AutoSaved(Result<String, String>, Vec<u32>),

    Reading(usize, usize),

    TextNote(String),

    SaveCancelled,
    Folder(PathBuf),
    SettingsFolder(PathBuf, bool),
    TestPage(DynamicImage),
    TestDone(Result<usize, Fail>),
    Opened(Vec<PathBuf>),

    Loaded(Vec<Loaded>, usize),

    NoOpen,

    Checked(Option<String>),
    Recovered(Vec<Loaded>, Vec<PathBuf>),
}

#[derive(Clone)]
enum Edit {
    Name(String),
    Source(usize),
    Size(usize),
    Dpi(usize),
    Colour(usize),
    Threshold(f32),
    Brightness(f32),
    Contrast(f32),
    Rotate(usize),
    SkipBlank(bool),
    AskMore(bool),
    AutoSave(bool),
    Pattern(String),
    Format(usize),
    ClearAfter(bool),
    ResetTone,
    Trim(bool),
    Straighten(bool),
    Batch(usize),
    Runs(usize),
    Pause(usize),
    OneFile(bool),
    Searchable(bool),
    OcrLang(usize),
    OcrLang2(usize),
}

#[derive(Clone, Copy, PartialEq)]
enum ZoomBy {
    In,
    Out,
    Fit,
    Actual,
}

const RATIOS: [Option<f32>; 5] = [None, Some(1.0), Some(4.0 / 3.0), Some(3.0 / 2.0), Some(16.0 / 9.0)];

const VIEW_AREA: (f32, f32) = (520.0, 520.0);

#[derive(Clone)]
enum Msg {
    Scan,
    CancelScan,
    CancelReading,
    Finish,
    Scanners,
    SearchAgain,
    StopSearch,
    Pick(usize),
    Address(String),
    AddAddress,
    Back,
    Click(usize),
    Delete,
    Undo,
    Redo,
    Save,
    CloseSave,
    CloseDiscard,
    Keys(Mods, Keysym),
    ClearSaved,
    RotateLeft,
    RotateRight,
    Flip,
    Profiles,
    UseProfile(usize),
    NewProfile,
    Preset(usize),
    EditProfile(usize),
    DuplicateProfile(usize),
    DeleteProfile(usize),
    MoveProfile(usize, isize),
    DefaultProfile(usize),
    Edit(Edit),
    ChooseFolder,
    UseGlass,
    TryAgain,
    OpenFolder,
    SkipPause,
    StopBatch,
    Look,
    Step(isize),
    Crop(usize, f32),

    ViewPointer(Pointer),
    ViewScroll(f32),
    Zoom(ZoomBy),
    CropRatio(usize),
    ApplyCrop,
    NoCrop,
    Digit(usize),
    StopRetry,
    OpenPictures,
    Recover,
    ForgetRecovery,
    Settings,
    Shortcuts,
    Browse(BrowseMsg),
    TestScan,
    DropIn(Vec<PathBuf>),
    DropFailed(String),
    Set(Set),
    AskSettingsFolder(bool),
}

#[derive(Clone)]
enum BrowseMsg {
    Pick(usize),
    Up,
    Name(String),
    NewName(String),
    MakeFolder,
    Confirm,
    Cancel,
}

#[derive(Clone)]
enum Set {
    Pattern(String),
    Thumb(f32),
    Units(usize),
    RemoveSaved(bool),
    Title(String),
    Author(String),
    Subject(String),
    Keywords(String),
    Password(String),
    Split(usize),
}

#[derive(PartialEq, Clone, Copy)]
enum Screen {
    Main,
    Scanners,
    Profiles,
    Presets,
    Editor(usize),
    Viewer(usize),
    Settings,
    Shortcuts,
    Browse,
    Closing,
}

const DPIS: [u32; 8] = [75, 100, 150, 200, 300, 400, 600, 1200];

struct Scan {
    events: Loop,
    bus: Option<Bus>,
    post: Post<Event>,
    asking: Option<Listen>,
    screen: Screen,
    scanners: Vec<Scanner>,
    chosen: Option<usize>,
    caps: Option<Caps>,
    searching: bool,
    opening: bool,
    stop_search: Arc<AtomicBool>,
    address: String,
    problem: Option<String>,
    name_error: Option<String>,
    list: List,
    scanning: bool,
    stop_scan: Arc<AtomicBool>,
    arrived: usize,
    mods: Cell<Mods>,
    saving: bool,

    reading: Option<(usize, usize)>,
    stop_reading: Arc<AtomicBool>,
    text_note: String,

    ocr_langs: Vec<String>,
    note: String,
    book: Book,
    book_path: Option<PathBuf>,

    session: Option<Vec<u32>>,
    more: bool,
    turn_over: Option<usize>,
    no_paper: bool,
    offline: bool,
    blanks: usize,
    held_blank: Vec<Page>,
    last_saved_dir: Option<PathBuf>,
    force_glass: bool,
    batch: BatchState,
    summary: String,
    busy: Option<u32>,
    announce: Option<String>,
    keep_dir: Option<PathBuf>,
    recoverable: usize,
    keep_lock: Option<std::fs::File>,

    remembered: Option<usize>,
    view_img: Option<Rc<DynamicImage>>,

    zoom: View,
    zoom_drag: zoom::Drag,
    crop_box: Crop,
    crop_pull: crop::Gesture,
    panning: bool,
    ratio: usize,
    hover: (f32, f32),
    draft: (f32, f32, f32, f32),
    settings: Settings,
    settings_path: Option<PathBuf>,
    browse: Option<Browse>,
    browse_back: Screen,
    roots: Vec<PathBuf>,
    testing: bool,
    test_img: Option<Rc<DynamicImage>>,
    test_problem: Option<String>,
}

#[derive(Default)]
struct BatchState {
    runs_done: u32,
    pages: usize,

    pause_left: Option<u32>,
    stop: bool,
}

const PAUSES: [u32; 6] = [0, 5, 10, 30, 60, 120];
const RUNS: [u32; 6] = [2, 3, 4, 5, 10, 20];

impl Scan {
    fn new(events: Loop, bus: Option<Bus>, post: Post<Event>) -> Scan {
        Scan {
            events, bus, post,
            asking: None, screen: Screen::Main,
            scanners: Vec::new(), chosen: None, caps: None,
            searching: false, opening: false,
            stop_search: Arc::new(AtomicBool::new(false)),
            address: String::new(),
            problem: None, name_error: None,
            list: List::default(),
            scanning: false, stop_scan: Arc::new(AtomicBool::new(false)), arrived: 0,
            mods: Cell::new(Mods::default()),
            saving: false, reading: None, stop_reading: Arc::new(AtomicBool::new(false)),
            text_note: String::new(), ocr_langs: Vec::new(), note: String::new(),
            book: Book::first_run(), book_path: None,
            session: None, more: false, turn_over: None,
            no_paper: false, offline: false, blanks: 0, held_blank: Vec::new(),
            last_saved_dir: None, force_glass: false,
            batch: BatchState::default(),
            summary: String::new(),
            busy: None,
            announce: None,
            keep_dir: None, recoverable: 0, keep_lock: None, remembered: None,
            view_img: None,
            zoom: View::new((1.0, 1.0), VIEW_AREA), zoom_drag: zoom::Drag::new(),
            crop_box: Crop::new(Rect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 }),
            crop_pull: crop::Gesture::new(), panning: false, ratio: 0,
            hover: (0.0, 0.0),
            draft: (0.0, 0.0, 0.0, 0.0),
            settings: Settings::default(), settings_path: None,
            browse: None, browse_back: Screen::Main,
            roots: browse::roots(),
            testing: false, test_img: None, test_problem: None,
        }
    }

    fn with_settings(mut self, path: Option<PathBuf>) -> Scan {
        match path {
            Some(path) => {
                self.settings = Settings::load(&path);
                self.settings_path = Some(path);
            }
            None => self.problem = Some(t!("scan-main-error-no-home").into()),
        }
        self
    }

    fn persist_settings(&mut self) {
        if let Some(p) = &self.settings_path
            && let Err(e) = self.settings.save(p) {
                self.problem = Some(t!("scan-main-error-keep-settings", why = &e.to_string()));
            }
    }

    fn with_book(mut self, path: Option<PathBuf>) -> Scan {
        if let Some(path) = path {
            self.book = Book::load(&path);
            self.book_path = Some(path);
        }
        self.follow_profile();
        self
    }

    fn with_keep(mut self, dir: PathBuf) -> Scan {
        if !settings::private_folder(&dir) {
            return self;
        }

        let lock = std::fs::File::create(dir.with_extension("lock")).ok().filter(|f| f.try_lock().is_ok());
        if lock.is_some() {
            self.recoverable = std::fs::read_dir(&dir).map(|r| r.flatten().filter(|e| e.path().extension().map(|x| x == "jpg").unwrap_or(false)).count()).unwrap_or(0);
            self.keep_dir = Some(dir);
            self.keep_lock = lock;
        }
        self
    }

    fn close_asked(&mut self) {
        if self.list.pages.iter().any(|p| !p.saved) || self.scanning || self.saving {
            self.screen = Screen::Closing;
        } else {
            self.events.quit();
        }
    }

    fn keep(&self, p: &Page) {

        if let Some(d) = &self.keep_dir
            && self.recoverable == 0
        {
            let _ = trindle_common::files::write_private(&d.join(format!("{:06}.jpg", p.id)), p.jpeg.as_slice());
        }
    }

    fn clear_keep(&self) {
        if let Some(d) = &self.keep_dir {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    fn persist(&mut self) {
        if let Some(p) = &self.book_path
            && let Err(e) = self.book.save(p) {
                self.problem = Some(t!("scan-main-error-keep-profiles", why = &e.to_string()));
            }
    }

    fn scanner(&self) -> Option<&Scanner> {
        self.chosen.and_then(|i| self.scanners.get(i))
    }

    fn status(&self) -> String {
        let n = self.list.pages.len();
        let sel = self.list.selected().len();
        let pages = if sel > 0 { t!("scan-status-pages-selected", n = n, sel = sel) } else { t!("scan-status-pages", n = n) };
        let state = if self.scanning {
            t!("scan-status-scanning", n = self.arrived + 1)
        } else if self.searching {
            t!("scan-status-searching").into()
        } else if let Some(s) = self.scanner() {
            if self.caps.is_some() { t!("scan-status-ready", name = &s.name) } else { t!("scan-status-asking", name = &s.name) }
        } else {
            t!("scan-status-none").into()
        };
        t!("scan-status-line", pages = &pages, state = &state)
    }

    fn event(&mut self, e: Event) {
        match e {
            m @ (Event::TestPage(..) | Event::TestDone(..) | Event::Found(..) | Event::SearchDone | Event::Caps(..) | Event::Failed(..) | Event::Damaged | Event::Page { .. } | Event::ScanDone(..) | Event::Checked(..)) => self.event_scan(m),
            m @ (Event::SaveTo(..) | Event::SaveIn(..) | Event::Reading(..) | Event::TextNote(..) | Event::SaveCancelled | Event::Saved(..) | Event::AutoSaved(..)) => self.event_saving(m),
            m @ (Event::Opened(..) | Event::Loaded(..) | Event::NoOpen | Event::Recovered(..) | Event::SettingsFolder(..) | Event::Folder(..)) => self.event_files(m),
        }
    }
}

impl App for Scan {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            m @ (Msg::Scan | Msg::CancelScan | Msg::CancelReading | Msg::Finish | Msg::Scanners | Msg::SearchAgain | Msg::StopSearch | Msg::Pick(..) | Msg::Address(..) | Msg::AddAddress | Msg::SkipPause | Msg::StopBatch | Msg::Digit(..) | Msg::StopRetry | Msg::TestScan | Msg::UseGlass | Msg::TryAgain) => self.update_scan(m),
            m @ (Msg::Click(..) | Msg::Delete | Msg::Undo | Msg::Redo | Msg::ClearSaved | Msg::RotateLeft | Msg::RotateRight | Msg::Flip) => self.update_pages(m),
            m @ (Msg::Look | Msg::Step(..) | Msg::ViewPointer(..) | Msg::ViewScroll(..) | Msg::Zoom(..) | Msg::CropRatio(..) | Msg::Crop(..) | Msg::ApplyCrop | Msg::NoCrop | Msg::Keys(..) | Msg::DropIn(..) | Msg::DropFailed(..)) => self.update_viewer(m),
            m @ (Msg::Profiles | Msg::UseProfile(..) | Msg::NewProfile | Msg::Preset(..) | Msg::EditProfile(..) | Msg::DuplicateProfile(..) | Msg::DeleteProfile(..) | Msg::MoveProfile(..) | Msg::DefaultProfile(..) | Msg::Edit(..)) => self.update_profiles(m),
            m @ (Msg::Save | Msg::CloseSave | Msg::OpenFolder | Msg::OpenPictures | Msg::Recover | Msg::ForgetRecovery | Msg::ChooseFolder | Msg::AskSettingsFolder(..)) => self.update_saving(m),
            Msg::Back => {
                self.opening = false;
                self.screen = Screen::Main;
            }
            Msg::CloseDiscard => self.events.quit(),
            Msg::Settings => self.screen = Screen::Settings,
            Msg::Shortcuts => self.screen = Screen::Shortcuts,
            Msg::Browse(m) => self.browse_update(m),
            Msg::Set(x) => {
                match x {
                    Set::Pattern(t) => self.settings.pattern = t,
                    Set::Thumb(v) => self.settings.thumb = (v.round() as u32).clamp(settings::THUMB_MIN, settings::THUMB_MAX),
                    Set::Units(i) => self.settings.units = if i == 1 { Units::Inches } else { Units::Mm },
                    Set::RemoveSaved(b) => self.settings.remove_saved = b,
                    Set::Title(t) => self.settings.title = t,
                    Set::Author(t) => self.settings.author = t,
                    Set::Subject(t) => self.settings.subject = t,
                    Set::Keywords(t) => self.settings.keywords = t,
                    Set::Password(t) => self.settings.password = t,
                    Set::Split(i) => self.settings.split_every = settings::SPLITS.get(i).copied().unwrap_or(0),
                }
                self.persist_settings();
            }
        }
    }

    fn intercept(&self, input: &Input, _keys: bool) -> Option<Msg> {
        match input {
            Input::Mods(m) => {
                self.mods.set(*m);
                None
            }
            Input::Key { sym, mods, .. } if matches!(self.screen, Screen::Main | Screen::Viewer(_)) => {
                self.mods.set(*mods);
                if self.screen == Screen::Main && !mods.ctrl && !mods.alt
                    && let Some(n) = (Keysym::_1.raw()..=Keysym::_9.raw()).contains(&sym.raw()).then(|| (sym.raw() - Keysym::_1.raw()) as usize + 1) {
                        return Some(Msg::Digit(n));
                    }
                (matches!(*sym, Keysym::a | Keysym::z | Keysym::Z | Keysym::Delete | Keysym::Left | Keysym::Right) || (!mods.ctrl && !mods.alt && matches!(sym.raw(), 0x2b | 0x2d | 0x30 | 0x3d))).then_some(Msg::Keys(*mods, *sym))
            }
            _ => None,
        }
    }

    fn view(&self) -> Node<Msg> {
        let mut words = vec![note(self.status())];
        if !self.note.is_empty() {
            words.push(label(self.note.clone()));
        }
        if self.recoverable > 0 {
            words.push(label(t!("scan-pages-recover-left", n = self.recoverable)));
        }
        let mut lines: Vec<Node<Msg>> = vec![row(words).gap(16.0)];
        if self.recoverable > 0 {
            lines.push(row(vec![word_button(t!("scan-pages-recover")).on_press(Msg::Recover), word_button(t!("scan-pages-discard")).on_press(Msg::ForgetRecovery)]).gap(16.0));
        }
        if self.screen == Screen::Main && !self.list.pages.is_empty() {
            lines.push(row(vec![label(t!("scan-pages-thumbnails")), slider(self.settings.thumb as f32, settings::THUMB_MIN as f32, settings::THUMB_MAX as f32, 10.0, t!("scan-settings-thumb-spoken"), |v| Msg::Set(Set::Thumb(v))).width(Length::Fixed(180.0))]).gap(16.0));
        }
        lines.push(row(vec![word_button(t!("scan-pages-settings")).on_press(Msg::Settings), word_button(t!("scan-pages-shortcuts")).on_press(Msg::Shortcuts), word_button(t!("scan-pages-add-pictures")).on_press(Msg::OpenPictures)]).gap(16.0));
        let mut after: Vec<Node<Msg>> = Vec::new();
        if self.last_saved_dir.is_some() {
            after.push(word_button(t!("scan-pages-open-folder")).on_press(Msg::OpenFolder));
        }
        if self.list.pages.iter().any(|p| p.saved) {
            after.push(word_button(t!("scan-pages-clear-saved")).on_press(Msg::ClearSaved));
        }
        if !after.is_empty() {
            lines.push(row(after).gap(16.0));
        }
        let body = match self.screen {
            Screen::Main => self.pages_view(),
            Screen::Scanners => self.scanners_view(),
            Screen::Profiles => self.profiles_view(),
            Screen::Presets => self.presets_view(),
            Screen::Editor(_) => self.editor_view(),
            Screen::Viewer(i) => self.viewer_view(i),
            Screen::Settings => self.settings_view(),
            Screen::Shortcuts => self.shortcuts_view(),
            Screen::Closing => self.closing_view(),
            Screen::Browse => self.browse_view(),
        };
        column(vec![card(self.bar()), body, column(lines).gap(8.0).padding(10.0)])
            .gap(8.0)
            .padding(12.0)
            .on_drop_from_outside(
                |held| if held.offered.files { held.usual() } else { None },
                |_, given, _| match given {
                    Given::Files(f) => Msg::DropIn(f),
                    Given::Nothing(why) => Msg::DropFailed(t!("scan-main-drop-nothing", why = why.words())),
                    Given::Text(_) | Given::Typed(..) => Msg::DropFailed(t!("scan-main-drop-text").into()),
                },
            )
    }

    fn window_title(&self) -> Option<String> {
        Some(t!("scan-app-name").into())
    }
}

fn worker(work: impl FnOnce() + Send + 'static, on_panic: impl FnOnce() + Send + 'static) {
    std::thread::spawn(move || {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).is_err() {
            on_panic();
        }
    });
}

fn crashed() -> String {
    t!("scan-main-error-crashed").into()
}

fn split_address(text: &str) -> (String, u16) {
    if let Some(rest) = text.strip_prefix('[')
        && let Some((host, tail)) = rest.split_once(']') {
            let port = tail.strip_prefix(':').and_then(|p| p.parse().ok()).unwrap_or(80);
            return (format!("[{host}]"), port);
        }
    if let Some((h, p)) = text.rsplit_once(':')
        && let (false, Ok(port)) = (h.contains(':'), p.parse::<u16>()) {
            return (h.to_string(), port);
        }
    if text.contains(':') { (format!("[{text}]"), 80) } else { (text.to_string(), 80) }
}

fn main() {
    words::ready();
    let events = Loop::new();
    let gone = events.clone();
    let Some(display) = Display::connect(&events, move || gone.quit()) else {
        eprintln!("{}", t!("scan-main-start-no-desktop"));
        std::process::exit(1);
    };
    let slot: Rc<std::cell::RefCell<Option<Rc<Shown<Scan>>>>> = Rc::new(std::cell::RefCell::new(None));
    let there = slot.clone();
    let post = events.post(move |e: Event| {
        if let Some(s) = there.borrow().as_ref() {
            s.change(|a| a.event(e));
            if let Some(words) = s.with(|a| a.announce.clone()) {
                s.change(|a| a.announce = None);
                s.announce(&words);
            }
        }
    });
    let app = Scan::new(events.clone(), Bus::session(&events).ok(), post).with_settings(Settings::path()).with_book(Book::path());
    let keep = app.settings.keep_folder();
    let app = app.with_keep(keep);
    let kind = Kind::Window { title: t!("scan-app-name").into(), app_id: "org.trindle.Scan".into() };
    let Some(shown) = Shown::open(&events, &display, kind, (960, 680), app) else {
        eprintln!("{}", t!("scan-main-start-no-window"));
        std::process::exit(1);
    };
    shown.change(|a| a.on_open());
    let shown = Rc::new(shown);
    *slot.borrow_mut() = Some(shown.clone());
    let asked = shown.clone();
    shown.window.on_close(move || asked.change(|a| a.close_asked()));
    let tick = shown.clone();
    events.every(std::time::Duration::from_secs(1), move || {
        tick.change_if(|a| a.tick());

        if let Some(words) = tick.with(|a| a.announce.clone()) {
            tick.change(|a| a.announce = None);
            tick.announce(&words);
        }
        trindle_toolkit::event::Flow::Continue
    });

    shown.change(|a| a.start_search());
    events.run();

    shown.with(|a| if a.recoverable == 0 { a.clear_keep() });
}
