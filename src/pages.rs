//! The page list and its undo history.

use crate::settings::BASE_DPI;
use image::{DynamicImage, GenericImageView};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
pub struct Page {
    pub id: u32,

    pub rot: u16,
    pub flip_h: bool,
    pub flip_v: bool,

    pub crop: Option<(f32, f32, f32, f32)>,
    pub thumb0: Rc<DynamicImage>,
    pub jpeg: Arc<Vec<u8>>,
    pub thumb: Rc<DynamicImage>,
    pub width: u32,
    pub height: u32,
    pub grey: bool,
    pub bw: bool,
    pub dpi: u32,
    pub selected: bool,
    pub saved: bool,
}

pub type Loaded = (Vec<u8>, DynamicImage, u32, u32, bool);

pub fn loaded(jpeg: Vec<u8>, img: &DynamicImage) -> Loaded {
    let (w, h) = img.dimensions();
    (jpeg, img.thumbnail(300, 400), w, h, matches!(img, DynamicImage::ImageLuma8(_)))
}

impl Page {
    pub fn new(id: u32, jpeg: Vec<u8>, thumb: DynamicImage, width: u32, height: u32, grey: bool) -> Page {
        let thumb = Rc::new(thumb);
        Page { id, rot: 0, flip_h: false, flip_v: false, crop: None, thumb0: thumb.clone(), thumb, jpeg: Arc::new(jpeg), width, height, grey, bw: false, dpi: BASE_DPI, selected: false, saved: false }
    }

    pub fn apply(&self, img: DynamicImage) -> DynamicImage {
        edit(self.rot, self.flip_h, self.flip_v, self.crop, img)
    }

    pub fn refresh(&mut self) {
        self.thumb = Rc::new(self.apply((*self.thumb0).clone()));
    }

    pub fn rotate(&mut self, by: u16) {
        self.rot = (self.rot + by) % 360;
        self.crop = None;
    }

    pub fn cut(&self) -> Cut {
        Cut { jpeg: self.jpeg.clone(), rot: self.rot, flip_h: self.flip_h, flip_v: self.flip_v, crop: self.crop, width: self.width, height: self.height, grey: self.grey, bw: self.bw, dpi: self.dpi }
    }
}

fn edit(rot: u16, flip_h: bool, flip_v: bool, crop: Option<(f32, f32, f32, f32)>, img: DynamicImage) -> DynamicImage {
    let mut img = match rot {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => img,
    };
    if flip_h {
        img = img.fliph();
    }
    if flip_v {
        img = img.flipv();
    }
    if let Some((l, t, r, b)) = crop {
        let (w, h) = image::GenericImageView::dimensions(&img);
        let x = (l * w as f32) as u32;
        let y = (t * h as f32) as u32;
        let cw = ((1.0 - l - r) * w as f32) as u32;
        let ch = ((1.0 - t - b) * h as f32) as u32;
        if cw >= 8 && ch >= 8 && x + cw <= w && y + ch <= h {
            img = img.crop_imm(x, y, cw, ch);
        }
    }
    img
}

#[derive(Clone)]
pub struct Cut {
    jpeg: Arc<Vec<u8>>,
    rot: u16,
    flip_h: bool,
    flip_v: bool,
    crop: Option<(f32, f32, f32, f32)>,
    pub width: u32,
    pub height: u32,
    pub grey: bool,
    pub bw: bool,
    pub dpi: u32,
}

pub type FinalPage = (Arc<Vec<u8>>, u32, u32, Option<DynamicImage>);

impl Cut {
    pub fn final_jpeg(&self) -> Result<(Arc<Vec<u8>>, u32, u32), String> {
        self.final_with_picture(false).map(|(j, w, h, _)| (j, w, h))
    }

    pub fn final_with_picture(&self, want_picture: bool) -> Result<FinalPage, String> {
        let unedited = !(self.rot != 0 || self.flip_h || self.flip_v || self.crop.is_some());
        if unedited && !want_picture {
            return Ok((self.jpeg.clone(), self.width, self.height, None));
        }
        let img = self.edited()?;
        let (w, h) = image::GenericImageView::dimensions(&img);
        let jpeg = if unedited { self.jpeg.clone() } else { Arc::new(crate::tidy::jpeg(&img)?) };
        Ok((jpeg, w, h, want_picture.then_some(img)))
    }

    pub fn edited(&self) -> Result<DynamicImage, String> {
        let img = image::load_from_memory(&self.jpeg).map_err(|_| trindle_toolkit::t!("scan-save-page-unreadable").to_string())?;
        Ok(edit(self.rot, self.flip_h, self.flip_v, self.crop, img))
    }
}

const UNDO_DEPTH: usize = 30;

#[derive(Default)]
pub struct List {
    pub pages: Vec<Page>,
    undo: Vec<Vec<Page>>,
    redo: Vec<Vec<Page>>,
    next: u32,
}

impl List {
    pub fn new_id(&mut self) -> u32 {
        self.next += 1;
        self.next
    }

    pub fn mark(&mut self) {
        self.undo.push(self.pages.clone());
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn forget_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(before) => {
                self.redo.push(std::mem::replace(&mut self.pages, before));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(after) => {
                self.undo.push(std::mem::replace(&mut self.pages, after));
                true
            }
            None => false,
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn selected(&self) -> Vec<usize> {
        self.pages.iter().enumerate().filter(|(_, p)| p.selected).map(|(i, _)| i).collect()
    }

    pub fn select_only(&mut self, which: &[usize]) {
        for (i, p) in self.pages.iter_mut().enumerate() {
            p.selected = which.contains(&i);
        }
    }

    pub fn toggle(&mut self, i: usize) {
        if let Some(p) = self.pages.get_mut(i) {
            p.selected = !p.selected;
        }
    }

    pub fn range_to(&mut self, i: usize) {
        let from = self.pages.iter().rposition(|p| p.selected).unwrap_or(0);
        let (a, b) = (from.min(i), from.max(i));
        let which: Vec<usize> = (a..=b).collect();
        self.select_only(&which);
    }

    pub fn edit_selected(&mut self, f: impl Fn(&mut Page)) {
        if !self.pages.iter().any(|p| p.selected) {
            return;
        }
        self.mark();
        for p in self.pages.iter_mut().filter(|p| p.selected) {
            f(p);
            p.saved = false;
            p.refresh();
        }
    }

    pub fn delete_selected(&mut self) {
        if self.pages.iter().any(|p| p.selected) {
            self.mark();
            self.pages.retain(|p| !p.selected);
        }
    }

    pub fn move_selected(&mut self, by: isize) {
        let sel = self.selected();
        if sel.is_empty() {
            return;
        }
        let blocked = if by < 0 { sel[0] == 0 } else { *sel.last().unwrap() + 1 == self.pages.len() };
        if blocked {
            return;
        }
        self.mark();
        let order: Vec<usize> = if by < 0 { sel.clone() } else { sel.iter().rev().cloned().collect() };
        for i in order {
            self.pages.swap(i, (i as isize + by) as usize);
        }
    }

    pub fn interleave(&mut self, start: usize, n: usize, reversed: bool) {
        if n == 0 || start + n >= self.pages.len() {
            return;
        }
        self.mark();
        let mut backs: Vec<Page> = self.pages.split_off(start + n);
        let fronts: Vec<Page> = self.pages.split_off(start);
        if reversed {
            backs.reverse();
        }
        let mut out = Vec::new();
        let mut b = backs.into_iter();
        for f in fronts {
            out.push(f);
            if let Some(x) = b.next() {
                out.push(x);
            }
        }
        out.extend(b);
        self.pages.extend(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(n: usize) -> List {
        let mut l = List::default();
        for _ in 0..n {
            let id = l.new_id();
            l.pages.push(Page {
                id,
                rot: 0,
                flip_h: false,
                flip_v: false,
                crop: None,
                thumb0: Rc::new(DynamicImage::new_rgb8(1, 1)),
                jpeg: Arc::new(vec![]),
                thumb: Rc::new(DynamicImage::new_rgb8(1, 1)),
                width: 1,
                height: 1,
                grey: false,
                bw: false,
                dpi: BASE_DPI,
                selected: false,
                saved: false,
            });
        }
        l
    }
    #[test]
    fn an_unreadable_edited_page_is_an_error_not_a_blank_save() {
        crate::words::ready();
        let mut l = list(1);
        l.pages[0].rot = 90;
        assert!(l.pages[0].cut().final_jpeg().is_err());
        l.pages[0].rot = 0;
        assert!(l.pages[0].cut().final_jpeg().is_ok());
    }
    #[test]
    fn undo_is_capped_and_history_can_be_dropped() {
        let mut l = list(1);
        for _ in 0..(UNDO_DEPTH + 10) {
            l.mark();
        }
        assert_eq!(l.undo.len(), UNDO_DEPTH);
        l.forget_history();
        assert!(!l.can_undo() && !l.can_redo());
    }
    fn ids(l: &List) -> Vec<u32> {
        l.pages.iter().map(|p| p.id).collect()
    }

    #[test]
    fn delete_undo_redo() {
        crate::words::ready();
        let mut l = list(4);
        l.select_only(&[1, 2]);
        l.delete_selected();
        assert_eq!(ids(&l), vec![1, 4]);
        assert!(l.undo());
        assert_eq!(ids(&l), vec![1, 2, 3, 4]);
        assert!(l.redo());
        assert_eq!(ids(&l), vec![1, 4]);
    }

    #[test]
    fn moving_keeps_the_selection_together() {
        crate::words::ready();
        let mut l = list(5);
        l.select_only(&[1, 2]);
        l.move_selected(1);
        assert_eq!(ids(&l), vec![1, 4, 2, 3, 5]);
        l.move_selected(-1);
        l.move_selected(-1);
        assert_eq!(ids(&l), vec![2, 3, 1, 4, 5]);
    }

    #[test]
    fn two_sided_stack_is_interleaved() {
        crate::words::ready();
        let mut l = list(6);
        l.interleave(0, 3, true);
        assert_eq!(ids(&l), vec![1, 6, 2, 5, 3, 4]);
        let mut l = list(4);
        l.interleave(0, 2, false);
        assert_eq!(ids(&l), vec![1, 3, 2, 4]);
        let mut l = list(5);
        l.interleave(1, 2, false);
        assert_eq!(ids(&l), vec![1, 2, 4, 3, 5]);
    }

    #[test]
    fn edits_are_instructions_and_undo_exactly() {
        crate::words::ready();
        let mut l = list(2);
        l.select_only(&[0]);
        l.edit_selected(|p| p.rotate(90));
        l.edit_selected(|p| p.rotate(270));
        assert_eq!(l.pages[0].rot, 0);
        assert!(l.undo());
        assert_eq!(l.pages[0].rot, 90);
        assert!(l.undo());
        assert_eq!(l.pages[0].rot, 0);
        assert!(l.pages[0].crop.is_none() && l.pages[0].rot == 0);
    }

    #[test]
    fn range_selects_between() {
        crate::words::ready();
        let mut l = list(6);
        l.select_only(&[1]);
        l.range_to(3);
        assert_eq!(l.selected(), vec![1, 2, 3]);
    }
}
