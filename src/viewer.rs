//! The page list, the one-page viewer and its crop box.

use super::*;

impl Scan {
    pub(super) fn open_view(&mut self, i: usize) {
        let Some(p) = self.list.pages.get(i) else { return };
        let Ok(img) = image::load_from_memory(&p.jpeg) else {
            self.problem = Some(t!("scan-view-error-unshown").into());
            return;
        };
        let mut flat = p.clone();
        flat.crop = None;
        self.draft = p.crop.unwrap_or((0.0, 0.0, 0.0, 0.0));
        let shown = flat.apply(img.thumbnail(1800, 1800));
        let (w, h) = shown.dimensions();
        self.zoom = View::new((w as f32, h as f32), VIEW_AREA);
        self.crop_box = Crop::new(Rect { x: 0.0, y: 0.0, w: w as f32, h: h as f32 });
        self.ratio = 0;
        self.view_img = Some(Rc::new(shown));
        self.crop_from_draft();
        self.screen = Screen::Viewer(i);
    }

    pub(super) fn crop_from_draft(&mut self) {
        let Some(img) = &self.view_img else {
            return;
        };
        let (w, h) = img.dimensions();
        let (w, h) = (w as f32, h as f32);
        let (l, t, r, b) = self.draft;
        self.crop_box.set_rect(Rect { x: l * w, y: t * h, w: (1.0 - l - r) * w, h: (1.0 - t - b) * h });
    }

    pub(super) fn draft_from_crop(&mut self) {
        let Some(img) = &self.view_img else {
            return;
        };
        let (w, h) = img.dimensions();
        let (w, h) = (w as f32, h as f32);
        let r = self.crop_box.rect();
        let share = |v: f32| if v < 0.002 { 0.0 } else { v };
        self.draft = (share(r.x / w), share(r.y / h), share(1.0 - (r.x + r.w) / w), share(1.0 - (r.y + r.h) / h));
    }

    pub(super) fn crop_is_whole(&self) -> bool {
        self.draft == (0.0, 0.0, 0.0, 0.0)
    }

    pub(super) fn view_pointer(&mut self, ev: Pointer) {
        match ev {
            Pointer::Hover(x, y) => self.hover = (x, y),
            Pointer::Down(x, y) => {
                self.hover = (x, y);
                let (cx, cy) = self.zoom.to_content(x, y);
                let part = self.crop_box.handle_at(cx, cy, crop::GRAB / self.zoom.scale());
                self.panning = match part {
                    Some(crop::Handle::Body) => self.crop_is_whole(),
                    Some(_) => false,
                    None => true,
                };
                if self.panning {
                    self.zoom_drag.pointer(&mut self.zoom, ev);
                } else {
                    self.crop_pull.pointer(&mut self.crop_box, &self.zoom, ev);
                }
            }
            Pointer::Move(x, y) => {
                self.hover = (x, y);
                if self.panning {
                    self.zoom_drag.pointer(&mut self.zoom, ev);
                } else if self.crop_pull.pointer(&mut self.crop_box, &self.zoom, ev) {
                    self.draft_from_crop();
                }
            }
            Pointer::Up(..) => {
                if self.panning {
                    self.zoom_drag.pointer(&mut self.zoom, ev);
                } else {
                    self.crop_pull.pointer(&mut self.crop_box, &self.zoom, ev);
                }
                self.panning = false;
            }
            _ => {}
        }
    }

    pub(super) fn update_pages(&mut self, msg: Msg) {
        match msg {
            Msg::Click(i) => {
                let m = self.mods.get();
                if m.ctrl {
                    self.list.toggle(i)
                } else if m.shift {
                    self.list.range_to(i)
                } else {
                    self.list.select_only(&[i])
                }
            }
            Msg::Delete => {
                self.list.delete_selected();
                if let Screen::Viewer(i) = self.screen {
                    match self.list.pages.len() {
                        0 => self.screen = Screen::Main,
                        n => self.open_view(i.min(n - 1)),
                    }
                }
            }
            Msg::Undo => {
                self.list.undo();
            }
            Msg::Redo => {
                self.list.redo();
            }
            Msg::ClearSaved => {
                self.list.mark();
                self.list.pages.retain(|p| !p.saved);
            }
            Msg::RotateLeft => self.list.edit_selected(|p| p.rotate(270)),
            Msg::RotateRight => self.list.edit_selected(|p| p.rotate(90)),
            Msg::Flip => self.list.edit_selected(|p| p.flip_h = !p.flip_h),
            _ => {}
        }
    }

    fn crop_side(&mut self, side: usize, v: f32) {
        let d = &mut self.draft;
        let v = v / 100.0;
        match side {
            0 => d.0 = v,
            1 => d.1 = v,
            2 => d.2 = v,
            _ => d.3 = v,
        }

        d.0 = d.0.min(0.9 - d.2);
        d.1 = d.1.min(0.9 - d.3);
        self.crop_from_draft();
    }

    fn apply_crop(&mut self) {
        if let Screen::Viewer(i) = self.screen {
            let c = self.draft;
            let c = if c == (0.0, 0.0, 0.0, 0.0) { None } else { Some(c) };
            let mut targets = self.list.selected();
            if !targets.contains(&i) {
                targets = vec![i];
            }
            self.list.select_only(&targets);
            self.list.edit_selected(|p| p.crop = c);
            self.screen = Screen::Main;
        }
    }

    fn keys(&mut self, m: Mods, k: Keysym) {
        if matches!(self.screen, Screen::Viewer(_)) {
            let middle = (VIEW_AREA.0 / 2.0, VIEW_AREA.1 / 2.0);
            match k.raw() {
                0x2b | 0x3d => self.zoom.zoom_in(middle),
                0x2d => self.zoom.zoom_out(middle),
                0x30 => self.zoom.fit(),
                _ => {}
            }
            return;
        }
        if m.ctrl && k == Keysym::a {
            let all: Vec<usize> = (0..self.list.pages.len()).collect();
            self.list.select_only(&all);
        } else if m.ctrl && m.shift && (k == Keysym::z || k == Keysym::Z) {
            self.list.redo();
        } else if m.ctrl && (k == Keysym::z || k == Keysym::Z) {
            self.list.undo();
        } else if k == Keysym::Delete {
            self.list.delete_selected();

        } else if m.alt && k == Keysym::Left {
            self.list.move_selected(-1);
        } else if m.alt && k == Keysym::Right {
            self.list.move_selected(1);
        }
    }

    pub(super) fn update_viewer(&mut self, msg: Msg) {
        match msg {
            Msg::Look => {
                if let Some(&i) = self.list.selected().first() {
                    self.open_view(i);
                }
            }
            Msg::Step(by) => {
                let n = self.list.pages.len();
                match self.screen {
                    Screen::Viewer(_) if n == 0 => self.screen = Screen::Main,
                    Screen::Viewer(i) => self.open_view((i as isize + by).clamp(0, n as isize - 1) as usize),
                    _ => {}
                }
            }
            Msg::ViewPointer(ev) => self.view_pointer(ev),
            Msg::ViewScroll(lines) => self.zoom.wheel(lines, self.hover),
            Msg::Zoom(by) => {
                let middle = (VIEW_AREA.0 / 2.0, VIEW_AREA.1 / 2.0);
                match by {
                    ZoomBy::In => self.zoom.zoom_in(middle),
                    ZoomBy::Out => self.zoom.zoom_out(middle),
                    ZoomBy::Fit => self.zoom.fit(),
                    ZoomBy::Actual => self.zoom.actual(middle),
                }
            }
            Msg::CropRatio(i) => {
                self.ratio = i.min(RATIOS.len() - 1);
                self.crop_box.set_aspect(RATIOS[self.ratio]);
                self.draft_from_crop();
            }
            Msg::Crop(side, v) => self.crop_side(side, v),
            Msg::ApplyCrop => self.apply_crop(),
            Msg::NoCrop => {
                self.draft = (0.0, 0.0, 0.0, 0.0);
                self.crop_from_draft();
            }

            Msg::DropIn(files) if self.screen == Screen::Main => {
                self.event(Event::Opened(files));
                self.announce = Some(self.note.clone());
            }
            Msg::DropFailed(why) => self.problem = Some(why),
            Msg::Keys(m, k) => self.keys(m, k),
            _ => {}
        }
    }
}
