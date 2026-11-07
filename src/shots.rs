//! Pictures of the real window, drawn headless by the toolkit from made-up state; pictures/ is not kept in git.

use super::*;
use trindle_toolkit::theme::Theme;
use trindle_toolkit::ui::Ui;

const SIZE: (u32, u32) = (1000, 700);

fn paper(n: u32) -> DynamicImage {
    let mut img = image::RgbImage::from_pixel(300, 400, image::Rgb([244, 241, 234]));
    for i in 0..14u32 {
        let w = 240 - (i * 37 + n * 11) % 70;
        let y = 30 + i * 18 + if i > 9 { 20 } else { 0 };
        for x in 30..30 + w {
            for dy in 0..5 {
                img.put_pixel(x, y + dy, image::Rgb([150, 150, 150]));
            }
        }
    }
    DynamicImage::ImageRgb8(img)
}

fn app() -> Scan {
    let events = Loop::new();
    let post = events.post(|_: Event| {});
    let mut a = Scan::new(events, None, post);
    a.scanners.push(Scanner { name: "Office printer".into(), host: "192.168.1.40".into(), port: 80, root: "eSCL".into() });
    a.chosen = Some(0);
    a.caps = Some(Caps::default());
    a
}

fn fill(a: &mut Scan, n: u32) {
    for i in 0..n {
        let id = a.list.new_id();
        a.list.pages.push(Page::new(id, vec![], paper(i), 2480, 3508, false));
    }
}

fn shoot(a: &Scan, name: &str) {
    shoot_at(a, name, 1.0);
}

fn shoot_at(a: &Scan, name: &str, scale: f32) {
    let mut ui = Ui::new(a.view(), Theme::new(false, "#fbc711"));
    ui.lay_at(SIZE.0, SIZE.1);
    let mut pix = tiny_skia_pixmap();
    ui.paint(&mut pix.as_mut(), scale);
    let out = std::path::PathBuf::from(std::env::var_os("SHOTS_DIR").unwrap_or_else(|| "pictures".into()));
    std::fs::create_dir_all(&out).unwrap();
    let file = out.join(format!("real-{name}.png"));
    pix.save_png(&file).unwrap();
    assert!(std::fs::metadata(&file).unwrap().len() > 2000, "{name} came out blank");
}

fn tiny_skia_pixmap() -> tiny_skia::Pixmap {
    tiny_skia::Pixmap::new(SIZE.0, SIZE.1).unwrap()
}

#[test]
#[ignore = "draws the window pictures into pictures/ (or SHOTS_DIR); run in release"]
fn shoot_the_window() {
    crate::words::ready();
    let mut a = app();
    shoot(&a, "1-first-run");
    fill(&mut a, 8);
    a.list.select_only(&[2, 3]);
    a.note = "Scanned 8 pages".into();
    shoot(&a, "2-page-list");
    a.scanning = true;
    a.arrived = 8;
    shoot(&a, "3-scanning");
    a.scanning = false;
    let mut p = Profile::new("Document, colour");
    p.auto_save = true;
    p.folder = "/home/john/Scans".into();
    a.book.add(p);
    a.screen = Screen::Editor(0);
    shoot(&a, "5-profile-editor");
    a.test_img = Some(Rc::new(paper(3)));
    shoot(&a, "11-test-scan");
    a.test_img = None;
    a.book.profiles[0].searchable = true;
    a.book.profiles[0].ocr_lang = "eng+fra".into();
    a.ocr_langs = vec!["eng".into(), "fra".into(), "deu".into()];
    shoot(&a, "14-searchable-text");
    a.book.profiles[0].searchable = false;
    a.screen = Screen::Main;
    a.reading = Some((3, 8));
    shoot(&a, "15-reading-text");
    a.reading = None;
    a.screen = Screen::Profiles;
    a.book.add(Profile::new("Photo"));
    shoot(&a, "6-profiles");
    a.screen = Screen::Main;
    a.batch.pause_left = Some(12);
    shoot(&a, "7-batch-pause");
    a.batch.pause_left = None;
    a.list.select_only(&[1]);
    a.list.pages[1].jpeg = Arc::new(tidy::jpeg(&paper(1)).unwrap());
    a.open_view(1);
    shoot(&a, "8-viewer-crop");
    shoot_the_other_screens(&mut a);
}

fn shoot_the_other_screens(a: &mut Scan) {
    a.screen = Screen::Shortcuts;
    shoot(a, "12-shortcuts");
    let home = std::env::temp_dir().join("trindle-scan-shot-home");
    for d in ["Documents/Bills", "Documents/Letters", "Documents/Scans 2026", "Pictures", "Downloads"] {
        std::fs::create_dir_all(home.join(d)).unwrap();
    }
    a.roots = crate::browse::roots_in(&home);
    a.screen = Screen::Main;
    a.last_saved_dir = Some(home.join("Documents"));
    a.update(Msg::Save);
    shoot(a, "13-save-picker");
    a.last_saved_dir = None;
    a.screen = Screen::Main;
    a.browse = None;
    a.screen = Screen::Settings;
    a.settings.folder = "/home/john/Scans".into();
    shoot(a, "9-settings");
    a.screen = Screen::Main;
    a.settings.thumb = 100;
    shoot(a, "10-small-thumbnails");
    a.settings.thumb = 150;
    a.screen = Screen::Scanners;
    a.searching = true;
    shoot(a, "4-scanners");
}

#[test]
#[ignore = "draws the window pictures into pictures/ (or SHOTS_DIR); run in release"]
fn shoot_at_200_percent() {
    crate::words::ready();
    let mut a = app();
    fill(&mut a, 6);
    a.list.select_only(&[1]);
    shoot_at(&a, "zoom-1-page-list", 2.0);
    let mut p = Profile::new("Document, colour");
    p.auto_save = true;
    p.folder = "/home/john/Scans".into();
    a.book.add(p);
    a.screen = Screen::Editor(0);
    shoot_at(&a, "zoom-2-editor", 2.0);
    a.screen = Screen::Settings;
    shoot_at(&a, "zoom-3-settings", 2.0);
    a.screen = Screen::Scanners;
    shoot_at(&a, "zoom-4-scanners", 2.0);
}
