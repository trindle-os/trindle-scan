//! Tests that run the whole program with no screen and no D-Bus.

use super::*;
use std::cell::RefCell;
use std::time::Duration;

fn jpeg() -> Vec<u8> {
    let mut out = Vec::new();
    let mut img = image::RgbImage::from_pixel(200, 280, image::Rgb([240, 238, 232]));
    for y in 100..110 {
        for x in 20..180 {
            img.put_pixel(x, y, image::Rgb([20, 20, 20]));
        }
    }
    DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Jpeg).unwrap();
    out
}

use crate::testdir::tmp;

fn run(pages: usize, setup: impl FnOnce(&mut Scan), mut step: impl FnMut(&mut Scan, u32) -> bool + 'static) -> Rc<RefCell<Scan>> {
    let port = escl::tests::fake_with(pages, jpeg());
    let events = Loop::new();
    let seen: Rc<RefCell<Vec<Event>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = seen.clone();
    let post = events.post(move |e: Event| sink.borrow_mut().push(e));
    let app = Rc::new(RefCell::new(Scan::new(events.clone(), None, post)));
    setup(&mut app.borrow_mut());
    app.borrow_mut().update(Msg::Address(format!("127.0.0.1:{port}")));
    app.borrow_mut().update(Msg::AddAddress);
    let (a, q) = (app.clone(), events.clone());
    let mut n = 0;
    events.every(Duration::from_millis(30), move || {
        let batch: Vec<Event> = seen.borrow_mut().drain(..).collect();
        for e in batch {
            a.borrow_mut().event(e);
        }
        n += 1;
        if a.borrow().caps.is_some() && step(&mut a.borrow_mut(), n) {
            q.quit();
        }
        trindle_toolkit::event::Flow::Continue
    });
    let q = events.clone();
    events.after(Duration::from_secs(20), move || q.quit());
    events.run();
    app
}

fn profile_with(f: impl Fn(&mut Profile) + 'static) -> impl FnOnce(&mut Scan) {
    move |a: &mut Scan| {
        let mut p = Profile::new("Test");
        f(&mut p);
        a.book.add(p);
    }
}

#[test]
fn dot_dot_in_a_pattern_stays_inside_the_folder() {
    crate::words::ready();
    let d = tmp("dotdot");
    let mut p = Profile::new("P");
    p.folder = d.join("in").display().to_string();
    p.pattern = "../../out/Scan {counter:2}".into();
    p.format = Format::Jpeg;
    let pics = vec![(jpeg(), false)];
    let made = save_to_folder(&p, None, &pics, &pdf::Options::default(), 0).unwrap();
    assert!(made.starts_with(&p.folder), "{made}");
    assert!(std::fs::read_dir(&made).unwrap().count() == 1);
    assert!(!d.join("out").exists());
}

#[test]
fn a_worker_that_panics_still_posts_its_ending() {
    let (tx, rx) = std::sync::mpsc::channel();
    worker(|| panic!("boom"), move || tx.send(1).unwrap());
    assert_eq!(rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap(), 1);
}

#[test]
fn opening_shows_the_scanner_picker_until_one_is_found_then_the_page() {
    crate::words::ready();
    let mut a = Scan::new(Loop::new(), None, Loop::new().post(|_: Event| {}));
    a.searching = true;
    a.screen = Screen::Scanners;
    a.opening = true;
    a.event(Event::Found(Scanner { name: "Test".into(), host: "h".into(), port: 80, root: "eSCL".into() }));
    assert!(matches!(a.screen, Screen::Scanners));
    a.event(Event::SearchDone);
    assert!(matches!(a.screen, Screen::Main) && a.chosen == Some(0) && !a.opening);
    a.screen = Screen::Scanners;
    a.opening = true;
    a.update(Msg::Back);
    a.event(Event::SearchDone);
    assert!(matches!(a.screen, Screen::Main));
}

#[test]
fn deleting_the_last_page_in_the_viewer_goes_back_and_stepping_is_safe() {
    crate::words::ready();
    let mut a = Scan::new(Loop::new(), None, Loop::new().post(|_: Event| {}));
    let id = a.list.new_id();
    a.list.pages.push(Page::new(id, jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false));
    a.list.select_only(&[0]);
    a.open_view(0);
    assert!(matches!(a.screen, Screen::Viewer(0)));
    a.update(Msg::Delete);
    assert!(matches!(a.screen, Screen::Main));
    a.screen = Screen::Viewer(0);
    a.update(Msg::Step(1));
    assert!(matches!(a.screen, Screen::Main));
}

#[test]
fn find_scan_list_save() {
    crate::words::ready();
    let dir = tmp("find");
    let path = dir.join("test.pdf");
    let p = path.clone();
    let mut stage = 0;
    let app = run(
        2,
        profile_with(|p| {
            p.source = Source::Feeder;
        }),
        move |a, _| {
            match stage {
                0 => {
                    a.update(Msg::Scan);
                    stage = 1;
                }
                1 if !a.scanning && a.arrived > 0 => {
                    assert_eq!(a.list.pages.len(), 2);
                    assert!(a.problem.is_none(), "{:?}", a.problem);
                    a.event(Event::SaveTo(p.clone()));
                    stage = 2;
                }
                2 if !a.saving => return true,
                _ => {}
            }
            false
        },
    );
    assert!(app.borrow().problem.is_none());
    let pdf = std::fs::read(&path).expect("the PDF was not written");
    let _ = std::fs::remove_file(&path);
    assert!(pdf.starts_with(b"%PDF-1.7"));
    assert!(String::from_utf8_lossy(&pdf).contains("/Count 2"));
}

fn save_searchable(name: &str) -> (Vec<u8>, String) {
    let dir = tmp("searchable");
    let path = dir.join(format!("{name}.pdf"));
    let p = path.clone();
    let mut stage = 0;
    let app = run(
        2,
        profile_with(|p| {
            p.source = Source::Feeder;
            p.searchable = true;
        }),
        move |a, _| {
            match stage {
                0 => {
                    a.update(Msg::Scan);
                    stage = 1;
                }
                1 if !a.scanning && a.arrived > 0 => {
                    a.event(Event::SaveTo(p.clone()));
                    stage = 2;
                }
                2 if !a.saving => return true,
                _ => {}
            }
            false
        },
    );
    let note = app.borrow().note.clone();
    assert!(app.borrow().problem.is_none(), "{:?}", app.borrow().problem);
    let pdf = std::fs::read(&path).expect("the PDF was not written");
    (pdf, note)
}

#[test]
fn a_searchable_save_carries_the_text_and_never_blocks_without_the_reader() {
    crate::words::ready();
    let _path = crate::words::PATH_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tmp("reader");
    let empty = tmp("noreader");
    let _ = crate::ocr::tests::stand_in(&dir);
    ocr::set_test_path(Some(dir.as_os_str()));
    let (with, note) = save_searchable("with");
    assert!(!note.contains("Tesseract"), "{note}");
    assert!(String::from_utf8_lossy(&with).contains("/ToUnicode"), "the text font is missing");
    ocr::set_test_path(Some(empty.as_os_str()));
    let (without, note) = save_searchable("without");
    assert!(String::from_utf8_lossy(&without).contains("/Count 2"));
    assert!(!String::from_utf8_lossy(&without).contains("/ToUnicode"));
    assert!(note.contains("Tesseract"), "{note}");
    ocr::set_test_path(None);
}

#[test]
fn auto_save_writes_a_file_and_clears_the_list() {
    crate::words::ready();
    let dir = tmp("auto");
    let d = dir.display().to_string();
    let mut stage = 0;
    let app = run(
        3,
        profile_with(move |p| {
            p.source = Source::Feeder;
            p.auto_save = true;
            p.folder = d.clone();
            p.pattern = "{year}/Doc {counter:3}".into();
        }),
        move |a, _| {
            if stage == 0 {
                a.update(Msg::Scan);
                stage = 1;
            } else if stage == 1 && !a.scanning && a.session.is_none() && a.arrived > 0 {
                stage = 2;
            } else if stage == 2 && a.list.pages.is_empty() {
                return true;
            }
            false
        },
    );
    assert!(app.borrow().problem.is_none(), "{:?}", app.borrow().problem);
    let year = dir.read_dir().unwrap().flatten().next().expect("a folder from the pattern");
    let files: Vec<_> = year.path().read_dir().unwrap().flatten().collect();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name().to_string_lossy(), "Doc 001.pdf");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn turning_the_stack_over_puts_the_backs_in_order() {
    crate::words::ready();
    let mut stage = 0;
    let app = run(
        2,
        profile_with(|p| {
            p.source = Source::FeederManualBoth;
        }),
        move |a, _| {
            if stage == 0 {
                a.update(Msg::Scan);
                stage = 1;
            } else if stage == 1 && !a.scanning && a.turn_over.is_some() {
                assert_eq!(a.list.pages.len(), 2);
                a.update(Msg::Scan);
                stage = 2;
            } else if stage == 2 && !a.scanning && a.session.is_none() {
                return true;
            }
            false
        },
    );
    let ids: Vec<u32> = app.borrow().list.pages.iter().map(|p| p.id).collect();
    assert_eq!(ids, vec![1, 4, 2, 3], "front 1, back 1, front 2, back 2 with the backs turned over");
}

#[test]
fn glass_asks_for_another_page_then_finishes() {
    crate::words::ready();
    let mut stage = 0;
    let app = run(1, profile_with(|_| {}), move |a, _| {
        if stage == 0 {
            a.update(Msg::Scan);
            stage = 1;
        } else if stage == 1 && a.more {
            a.update(Msg::Scan);
            stage = 2;
        } else if stage == 2 && a.more {
            a.update(Msg::Finish);
            return true;
        }
        false
    });
    assert_eq!(app.borrow().list.pages.len(), 2);
    assert!(app.borrow().session.is_none());
}

#[test]
fn a_batch_of_runs_with_a_pause_makes_a_file_each() {
    crate::words::ready();
    let dir = tmp("batch");
    let d = dir.display().to_string();
    let mut stage = 0;
    let app = run(
        1,
        profile_with(move |p| {
            p.source = Source::Feeder;
            p.batch = 1;
            p.runs = 2;
            p.pause = 5;
            p.auto_save = true;
            p.folder = d.clone();
            p.pattern = "Run {counter:2}".into();
        }),
        move |a, _| {
            if stage == 0 {
                a.update(Msg::Scan);
                stage = 1;
            } else if stage == 1 && a.batch.pause_left.is_some() {
                a.update(Msg::SkipPause);
                stage = 2;
            } else if stage == 2 && a.batch.runs_done == 0 && a.list.pages.is_empty() && !a.scanning {
                return true;
            }
            false
        },
    );
    assert!(app.borrow().note.contains("2 runs"));
    let mut names: Vec<String> = dir.read_dir().unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    assert_eq!(names, vec!["Run 01.pdf", "Run 02.pdf"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pulling_a_handle_sets_the_crop_and_a_press_elsewhere_moves_a_zoomed_page() {
    crate::words::ready();
    let mut a = Scan::new(Loop::new(), None, Loop::new().post(|_: Event| {}));
    let id = a.list.new_id();
    a.list.pages.push(Page::new(id, jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false));
    a.open_view(0);
    let r = a.crop_box.rect();
    let (sx, sy) = a.zoom.to_screen(r.x, r.y);
    a.update(Msg::ViewPointer(Pointer::Down(sx, sy)));
    a.update(Msg::ViewPointer(Pointer::Move(sx + 60.0, sy + 40.0)));
    a.update(Msg::ViewPointer(Pointer::Up(sx + 60.0, sy + 40.0)));
    assert!(a.draft.0 > 0.0 && a.draft.1 > 0.0, "{:?}", a.draft);
    a.update(Msg::NoCrop);
    a.update(Msg::Zoom(ZoomBy::In));
    a.update(Msg::Zoom(ZoomBy::In));
    a.update(Msg::Zoom(ZoomBy::In));
    let before = a.zoom.content_rect();
    a.update(Msg::ViewPointer(Pointer::Down(260.0, 260.0)));
    a.update(Msg::ViewPointer(Pointer::Move(230.0, 230.0)));
    a.update(Msg::ViewPointer(Pointer::Up(230.0, 230.0)));
    assert!(a.zoom.content_rect() != before);
    assert!(a.crop_is_whole());
}

#[test]
fn crop_is_an_instruction_applied_to_every_selected_page() {
    crate::words::ready();
    let mut a = Scan::new(Loop::new(), None, Loop::new().post(|_: Event| {}));
    for _ in 0..3 {
        let id = a.list.new_id();
        a.list.pages.push(Page::new(id, jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false));
    }
    a.list.select_only(&[0, 1]);
    a.open_view(0);
    a.update(Msg::Crop(0, 10.0));
    a.update(Msg::Crop(3, 20.0));
    a.update(Msg::ApplyCrop);
    assert!(a.list.pages[0].crop.is_some() && a.list.pages[1].crop.is_some() && a.list.pages[2].crop.is_none());
    let (_, w, h) = a.list.pages[0].cut().final_jpeg().unwrap();
    assert_eq!((w, h), (180, 224));
    a.update(Msg::Undo);
    assert!(a.list.pages[0].crop.is_none());
}

#[test]
fn first_press_with_no_profile_offers_presets() {
    crate::words::ready();
    let mut a = Scan::new(Loop::new(), None, Loop::new().post(|_: Event| {}));
    a.update(Msg::Scan);
    assert!(a.screen == Screen::Presets);
    a.update(Msg::Preset(0));
    assert_eq!(a.book.profiles.len(), 1);
    assert_eq!(a.book.profiles[0].colour, Colour::BlackWhite);
}

#[test]
fn program_settings_reach_new_profiles_and_thumbnails() {
    crate::words::ready();
    let events = Loop::new();
    let post = events.post(|_: Event| {});
    let mut a = Scan::new(events, None, post);
    a.update(Msg::Set(Set::Pattern("Letter {counter:3}".into())));
    a.update(Msg::Set(Set::Thumb(5000.0)));
    a.update(Msg::Set(Set::Units(1)));
    a.update(Msg::Set(Set::RemoveSaved(true)));
    a.event(Event::SettingsFolder(PathBuf::from("/home/john/Scans"), false));
    assert_eq!(a.settings.thumb, settings::THUMB_MAX);
    a.update(Msg::Preset(0));
    let p = a.book.now().unwrap();
    assert_eq!(p.folder, "/home/john/Scans");
    assert_eq!(p.pattern, "Letter {counter:3}");
    a.update(Msg::Settings);
    assert!(a.screen == Screen::Settings);
    a.update(Msg::Back);
    assert!(a.screen == Screen::Main);
}

#[test]
fn test_scan_shows_one_page_and_keeps_the_list_empty() {
    crate::words::ready();
    let mut started = false;
    let a = run(2, profile_with(|_| {}), move |a, _| {
        if !started {
            started = true;
            a.update(Msg::TestScan);
        }
        a.test_img.is_some() && !a.testing
    });
    let a = a.borrow();
    assert!(a.test_img.is_some());
    assert!(a.list.pages.is_empty());
    assert!(a.test_problem.is_none());
}

#[test]
fn pdfs_split_by_page_count_and_lock_with_a_password() {
    crate::words::ready();
    let d = tmp("split");
    let page = || pdf::PdfPage { words: Vec::new(), jpeg: Arc::new(vec![0xFF, 0xD8, 0xFF, 0xD9]), mono: None, width: 10, height: 20, grey: false, page: (595.0, 842.0) };
    let pages: Vec<pdf::PdfPage> = (0..5).map(|_| page()).collect();
    let files = write_pdfs(&pages, &pdf::Options::default(), 2, &d.join("Bills.pdf"), false).unwrap();
    assert_eq!(files.len(), 3);
    assert!(d.join("Bills part 3.pdf").exists());
    let one = write_pdfs(&pages, &pdf::Options { password: "open sesame".into(), ..Default::default() }, 0, &d.join("Locked.pdf"), false).unwrap();
    let bytes = std::fs::read(&one[0]).unwrap();
    assert!(bytes.windows(8).any(|w| w == b"/Encrypt"));
}

#[test]
fn a_black_and_white_page_goes_into_the_pdf_as_one_bit() {
    crate::words::ready();
    let mut pg = Page::new(1, jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false);
    pg.bw = true;
    let parts = pdf_parts(&[pg.cut()]).unwrap();
    assert!(parts[0].mono.is_some());
    pg.bw = false;
    assert!(pdf_parts(&[pg.cut()]).unwrap()[0].mono.is_none());
}

#[test]
fn dropped_picture_files_become_pages() {
    crate::words::ready();
    let d = tmp("drop");
    let one = d.join("a.png");
    image::RgbImage::from_pixel(30, 40, image::Rgb([9, 9, 9])).save(&one).unwrap();
    let events = Loop::new();
    let post = events.post(|_: Event| {});
    let mut a = Scan::new(events, None, post);
    let (items, failed) = load_pictures(vec![one, d.join("notes.txt")]);
    a.event(Event::Loaded(items, failed));
    assert_eq!(a.list.pages.len(), 1);
    assert_eq!(a.note, "Added 1 picture. One file could not be read.");
}

#[test]
fn shortcuts_page_opens_and_closes() {
    crate::words::ready();
    let events = Loop::new();
    let post = events.post(|_: Event| {});
    let mut a = Scan::new(events, None, post);
    a.update(Msg::Shortcuts);
    assert!(a.screen == Screen::Shortcuts);
    let _ = a.view();
    a.update(Msg::Back);
    assert!(a.screen == Screen::Main);
}

#[test]
fn without_a_file_chooser_the_save_goes_through_the_programs_own_picker() {
    crate::words::ready();
    let home = tmp("picker");
    std::fs::create_dir_all(home.join("Documents")).unwrap();
    let events = Loop::new();
    let post = events.post(|_: Event| {});
    let mut a = Scan::new(events, None, post);
    a.roots = browse::roots_in(&home);
    let id = a.list.new_id();
    a.list.pages.push(Page::new(id, jpeg(), image::DynamicImage::new_rgb8(10, 10), 10, 10, false));
    a.update(Msg::Save);
    assert!(a.screen == Screen::Browse);
    a.update(Msg::Browse(BrowseMsg::Confirm));
    assert!(a.browse.as_ref().unwrap().error.is_some());
    a.update(Msg::Browse(BrowseMsg::Pick(0)));
    a.update(Msg::Browse(BrowseMsg::Name("Bills".into())));
    let _ = a.view();
    a.update(Msg::Browse(BrowseMsg::Confirm));
    assert!(a.screen == Screen::Main);
    assert!(a.saving);
    a.update(Msg::OpenPictures);
    assert!(a.screen == Screen::Browse);
    a.update(Msg::Browse(BrowseMsg::Cancel));
    assert!(a.screen == Screen::Main);
    let saved = home.join("Documents/Bills.pdf");
    for _ in 0..100 {
        if saved.is_file() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(100));
}

#[test]
fn the_settings_screen_with_its_hidden_password_box_lays_out() {
    crate::words::ready();
    let events = Loop::new();
    let mut a = Scan::new(events.clone(), None, events.post(|_: Event| {}));
    a.screen = Screen::Settings;
    let mut ui = trindle_toolkit::ui::Ui::new(a.view(), trindle_toolkit::theme::Theme::new(false, "#fbc711"));
    ui.lay_at(1000, 700);
}

#[test]
fn closing_with_unsaved_pages_asks_first() {
    crate::words::ready();
    let events = Loop::new();
    let mut a = Scan::new(events.clone(), None, events.post(|_: Event| {}));
    let id = a.list.new_id();
    a.list.pages.push(Page::new(id, jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false));
    a.close_asked();
    assert!(a.screen == Screen::Closing);
    assert!(!events.quitting());
    let _ = a.view();
    a.update(Msg::Back);
    assert!(a.screen == Screen::Main);
    a.list.pages[0].saved = true;
    a.close_asked();
    assert!(events.quitting());
}

#[test]
fn the_keep_folder_must_be_a_real_folder_of_ours() {
    crate::words::ready();
    let d = tmp("keep-safe");
    let real = d.join("real");
    std::fs::create_dir_all(&real).unwrap();
    let link = d.join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let events = Loop::new();
    let a = Scan::new(events.clone(), None, events.post(|_: Event| {})).with_keep(link);
    assert!(a.keep_dir.is_none());
    let a = Scan::new(events.clone(), None, events.post(|_: Event| {})).with_keep(d.join("fresh"));
    assert!(a.keep_dir.is_some());
    let mode = std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(d.join("fresh")).unwrap().permissions());
    assert_eq!(mode & 0o777, 0o700);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn forgetting_a_recovery_leaves_the_keep_folder_private() {
    crate::words::ready();
    let d = tmp("forget");
    let keep = d.join("pages");
    assert!(settings::private_folder(&keep));
    std::fs::write(keep.join("000001.jpg"), jpeg()).unwrap();
    let events = Loop::new();
    let mut a = Scan::new(events.clone(), None, events.post(|_: Event| {})).with_keep(keep.clone());
    assert_eq!(a.recoverable, 1);
    a.update(Msg::ForgetRecovery);
    assert_eq!(a.recoverable, 0);
    assert_eq!(std::fs::read_dir(&keep).unwrap().count(), 0);
    let mode = std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(&keep).unwrap().permissions());
    assert_eq!(mode & 0o777, 0o700);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn recovered_pages_are_kept_again_before_the_old_files_go() {
    crate::words::ready();
    let d = tmp("recover");
    let keep = d.join("pages");
    assert!(settings::private_folder(&keep));
    let old = vec![keep.join("000007.jpg"), keep.join("000008.jpg")];
    for f in &old {
        std::fs::write(f, jpeg()).unwrap();
    }
    let events = Loop::new();
    let mut a = Scan::new(events.clone(), None, events.post(|_: Event| {})).with_keep(keep.clone());
    assert_eq!(a.recoverable, 2);
    let items = old.iter().map(|_| (jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false)).collect();
    a.event(Event::Recovered(items, old.clone()));
    let left = std::fs::read_dir(&keep).unwrap().count();
    assert_eq!(left, 2, "each recovered page is on disk again");
    assert!(old.iter().all(|f| !f.exists()), "old names are gone");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn typed_addresses_are_split_at_the_right_colon() {
    assert_eq!(split_address("192.168.1.40"), ("192.168.1.40".into(), 80));
    assert_eq!(split_address("printer.local:8080"), ("printer.local".into(), 8080));
    assert_eq!(split_address("[fe80::1]:8080"), ("[fe80::1]".into(), 8080));
    assert_eq!(split_address("[fe80::1]"), ("[fe80::1]".into(), 80));
    assert_eq!(split_address("fe80::1"), ("[fe80::1]".into(), 80));
}

#[test]
fn same_named_scanners_stay_apart_and_a_failed_check_is_said() {
    crate::words::ready();
    let events = Loop::new();
    let mut a = Scan::new(events.clone(), None, events.post(|_: Event| {}));
    let one = Scanner { name: "Office".into(), host: "192.168.1.40".into(), port: 80, root: "eSCL".into() };
    let two = Scanner { name: "Office".into(), host: "192.168.1.41".into(), port: 80, root: "eSCL".into() };
    a.add_scanner(one.clone());
    a.add_scanner(two);
    assert_eq!(a.scanners.len(), 2);
    let again = Scanner { root: "scan".into(), ..one };
    let i = a.add_scanner(again);
    assert_eq!((a.scanners.len(), a.scanners[i].root.as_str()), (2, "scan"));
    a.checking = true;
    a.event(Event::Checked(Some("No answer".into())));
    assert_eq!(a.problem.as_deref(), Some("No answer"));
    a.problem = Some(t!("scan-main-note-checking-address").into());
    a.checking = true;
    a.event(Event::Checked(None));
    assert!(a.problem.is_none());
    a.problem = Some("Another problem".into());
    a.event(Event::Checked(None));
    assert_eq!(a.problem.as_deref(), Some("Another problem"), "a check nobody is waiting for leaves other words alone");
}

#[test]
fn pictures_that_cannot_be_read_are_counted_and_leave_undo_alone() {
    crate::words::ready();
    let events = Loop::new();
    let mut a = Scan::new(events.clone(), None, events.post(|_: Event| {}));
    a.event(Event::Loaded(Vec::new(), 2));
    assert!(a.note.contains('2'), "{}", a.note);
    assert!(a.list.pages.is_empty());
    assert!(!a.list.can_undo());
}

#[test]
fn no_home_folder_is_said_in_words_and_nothing_is_written() {
    crate::words::ready();
    let events = Loop::new();
    let a = Scan::new(events.clone(), None, events.post(|_: Event| {})).with_settings(None).with_book(None);
    assert!(a.problem.is_some());
    assert!(a.settings_path.is_none() && a.book_path.is_none());
}

fn one_page_app() -> Scan {
    crate::words::ready();
    let mut a = Scan::new(Loop::new(), None, Loop::new().post(|_: Event| {}));
    let id = a.list.new_id();
    a.list.pages.push(Page::new(id, jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false));
    a
}

#[test]
fn keys_and_drops_reach_their_handlers() {
    let mut a = one_page_app();
    let id = a.list.new_id();
    a.list.pages.push(Page::new(id, jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false));
    a.update(Msg::Keys(Mods { ctrl: true, ..Mods::default() }, Keysym::a));
    assert_eq!(a.list.selected().len(), 2);
    a.update(Msg::Keys(Mods::default(), Keysym::Delete));
    assert!(a.list.pages.is_empty());
    a.update(Msg::Keys(Mods { ctrl: true, ..Mods::default() }, Keysym::z));
    assert_eq!(a.list.pages.len(), 2);
}

#[test]
fn editing_a_saved_page_makes_it_unsaved_again() {
    let mut a = one_page_app();
    a.list.pages[0].saved = true;
    a.list.select_only(&[0]);
    a.update(Msg::RotateRight);
    assert!(!a.list.pages[0].saved);
}

#[test]
fn a_cropped_page_opens_with_its_crop_and_the_right_size() {
    let mut a = one_page_app();
    a.list.pages[0].crop = Some((0.1, 0.0, 0.1, 0.0));
    a.open_view(0);
    let r = a.crop_box.rect();
    let w = a.view_img.as_ref().unwrap().dimensions().0 as f32;
    assert!((r.x - 0.1 * w).abs() < 0.5 && (r.w - 0.8 * w).abs() < 0.5);
}

#[test]
fn a_good_retry_ends_the_busy_countdown() {
    let mut a = one_page_app();
    a.busy = Some(30);
    a.scan_done(Ok(1));
    assert!(a.busy.is_none());
    a.busy = Some(30);
    a.scan_done(Err(Fail::Offline("x".into())));
    assert!(a.busy.is_none());
}

#[test]
fn a_lone_blank_page_is_kept_on_disk() {
    let d = tmp("blankkeep");
    let keep = d.join("pages");
    let mut a = one_page_app().with_keep(keep.clone());
    a.list.pages.clear();
    let id = a.list.new_id();
    a.held_blank.push(Page::new(id, jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false));
    a.blanks = 1;
    a.scan_done(Ok(0));
    assert_eq!(a.list.pages.len(), 1);
    assert_eq!(std::fs::read_dir(&keep).unwrap().count(), 1);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_second_copy_leaves_the_first_copys_kept_pages_alone() {
    crate::words::ready();
    let d = tmp("twocopies");
    let keep = d.join("pages");
    let first = one_page_app().with_keep(keep.clone());
    std::fs::write(keep.join("000001.jpg"), jpeg()).unwrap();
    let second = one_page_app().with_keep(keep.clone());
    assert!(second.keep_dir.is_none() && second.recoverable == 0);
    assert!(first.keep_dir.is_some());
    drop(first);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn zoom_keys_are_taken_in_the_viewer_but_digits_are_not() {
    let mut a = one_page_app();
    a.list.select_only(&[0]);
    a.open_view(0);
    let key = |sym: Keysym| Input::Key { sym, text: None, mods: Mods::default(), repeat: false };
    assert!(matches!(a.intercept(&key(Keysym::plus), true), Some(Msg::Keys(..))));
    assert!(a.intercept(&key(Keysym::_1), true).is_none());
}

#[test]
fn using_the_glass_holds_until_the_scan_is_done() {
    let mut a = one_page_app();
    a.book.add(Profile::new("Hand"));
    a.book.profiles[0].source = Source::FeederManualBoth;
    a.book.profiles[0].ask_more = true;
    a.update(Msg::UseGlass);
    assert!(a.force_glass, "the choice is still there when the scan finishes");
    a.arrived = 1;
    a.session = Some(vec![a.list.pages[0].id]);
    a.after_run();
    assert!(a.turn_over.is_none(), "a glass page is not turned over");
    assert!(a.more, "and the question for another page comes");
    a.finish_session(false);
    assert!(!a.force_glass);
}

#[test]
fn the_viewer_bar_acts_on_the_page_shown() {
    let mut a = one_page_app();
    let id = a.list.new_id();
    a.list.pages.push(Page::new(id, jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false));
    a.list.select_only(&[0]);
    a.open_view(1);
    a.update(Msg::RotateRight);
    assert_eq!(a.list.pages[0].rot, 0);
    assert_eq!(a.list.pages[1].rot, 90);
    let w = a.view_img.as_ref().unwrap().dimensions();
    assert!(w.0 > w.1, "the picture shown is turned too");
    a.update(Msg::Delete);
    assert_eq!(a.list.pages.len(), 1);
    assert_eq!(a.list.pages[0].rot, 0, "the page that was shown went, the other stayed");
}

#[test]
fn a_lock_made_through_a_link_is_not_used_and_the_target_is_not_cut() {
    crate::words::ready();
    let d = tmp("locklink");
    let victim = d.join("victim.txt");
    std::fs::write(&victim, "keep me").unwrap();
    let keep = d.join("pages");
    std::os::unix::fs::symlink(&victim, d.join("pages.lock")).unwrap();
    let a = one_page_app().with_keep(keep);
    assert!(a.keep_dir.is_none());
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep me");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_page_that_cannot_be_kept_is_said_and_old_files_stay_until_new_ones_exist() {
    crate::words::ready();
    let d = tmp("unkept");
    let keep = d.join("pages");
    assert!(settings::private_folder(&keep));
    let old = keep.join("000007.jpg");
    std::fs::write(&old, jpeg()).unwrap();
    let mut a = one_page_app().with_keep(keep.clone());
    assert_eq!(a.recoverable, 1);
    let pg = a.list.pages[0].clone();
    a.keep(&pg);
    assert_eq!(a.unkept, 1, "nothing is written while old pages wait");
    a.say_unkept();
    assert!(a.problem.as_ref().is_some_and(|p| p.contains("recovery folder")));
    a.problem = None;
    let mode = |m| std::fs::set_permissions(&keep, std::os::unix::fs::PermissionsExt::from_mode(m)).unwrap();
    mode(0o500);
    let items = vec![(jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false)];
    a.event(Event::Recovered(items, vec![old.clone()]));
    mode(0o700);
    assert!(old.exists(), "the old file stays when the new copy could not be written");
    assert!(a.problem.is_some());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_hand_save_that_fails_leaves_the_old_file_whole() {
    let d = tmp("replace");
    let path = d.join("Bills.pdf");
    std::fs::write(&path, "the old file").unwrap();
    let r = saving::replace_file(&path, |mut f| {
        std::io::Write::write_all(&mut f, b"half")?;
        Err(std::io::Error::from(std::io::ErrorKind::StorageFull))
    });
    assert!(r.is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "the old file");
    assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1, "no half file is left beside it");
    saving::replace_file(&path, |mut f| std::io::Write::write_all(&mut f, b"new")).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_save_remembers_its_folder_whatever_words_it_gave() {
    crate::words::ready();
    let mut a = one_page_app();
    let d = tmp("savedto").to_path_buf();
    a.event(Event::Saved(Ok(SavedTo { dir: d.clone(), words: "three files, as the words say".into() })));
    assert_eq!(a.last_saved_dir.as_deref(), Some(d.as_path()));
}

#[test]
fn what_was_added_is_announced_when_the_files_have_loaded() {
    crate::words::ready();
    let mut a = one_page_app();
    a.note = "Old note".into();
    a.announce = None;
    a.event(Event::Loaded(vec![(jpeg(), DynamicImage::new_rgb8(20, 28), 200, 280, false)], 0));
    assert_eq!(a.announce.as_deref(), Some(a.note.as_str()));
    assert_ne!(a.note, "Old note");
}

#[test]
fn the_recognition_languages_arrive_as_an_event() {
    let mut a = one_page_app();
    assert!(a.ocr_langs.is_empty());
    a.event(Event::Languages(vec!["eng".into(), "fra".into()]));
    assert_eq!(a.ocr_langs, vec!["eng".to_string(), "fra".to_string()]);
}

#[test]
fn a_black_and_white_page_is_read_once_for_a_pdf() {
    let mut a = one_page_app();
    a.list.pages[0].bw = true;
    let cut = a.list.pages[0].cut();
    let (j, w, h, pic) = cut.final_with_picture(true).unwrap();
    assert!(pic.is_some() && w == 200 && h == 280 && !j.is_empty());
    let parts = saving::pdf_parts(&[cut]).unwrap();
    assert!(parts[0].mono.is_some());
}

#[test]
fn a_split_pdf_save_that_fails_part_way_leaves_no_files() {
    let d = tmp("splitfail");
    let page = || pdf::PdfPage { words: Vec::new(), jpeg: Arc::new(vec![0xFF, 0xD8, 0xFF, 0xD9]), mono: None, width: 10, height: 20, grey: false, page: (595.0, 842.0) };
    let calls = Cell::new(0);
    let r = saving::write_pdfs_using(
        &[page(), page(), page()],
        |_, mut w| {
            calls.set(calls.get() + 1);
            std::io::Write::write_all(&mut w, b"%PDF")?;
            if calls.get() == 2 { Err(std::io::Error::from(std::io::ErrorKind::StorageFull)) } else { std::io::Write::flush(&mut w) }
        },
        1,
        &d.join("Bills.pdf"),
        true,
    );
    assert!(r.is_err());
    assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0);
}
