//! What each screen looks like.

use super::*;

impl Scan {
    pub(super) fn viewer_view(&self, i: usize) -> Node<Msg> {
        let n = self.list.pages.len();
        let Some(img) = self.view_img.clone() else { return self.pages_view() };
        let pic = self.viewer_picture(i, n, img);
        column(vec![
            row(vec![
                title(t!("scan-view-page-of", i = i + 1, n = n)),
                space(),
                button("go-previous", t!("scan-view-previous")).on_press(Msg::Step(-1)),
                button("go-next", t!("scan-view-next")).on_press(Msg::Step(1)),
                word_button(t!("scan-common-back")).on_press(Msg::Back),
            ])
            .gap(10.0),
            row(vec![pic, self.viewer_side().width(Length::Fixed(320.0))]).gap(16.0),
        ])
        .gap(12.0)
        .padding(24.0)
    }

    fn viewer_picture(&self, i: usize, n: usize, img: Rc<DynamicImage>) -> Node<Msg> {
        let (view, crop_box) = (self.zoom, self.crop_box);
        let pic = drawing(VIEW_AREA, t!("scan-view-page-of", i = i + 1, n = n), move |canvas, pen| {
            let k = pen.scale;
            let at = view.content_rect();
            let (x, y, w, h) = ((at.x * k).round() as i32, (at.y * k).round() as i32, ((at.w * k) as u32).max(1), ((at.h * k) as u32).max(1));
            picture::draw(canvas, &img, x, y, w, h);
            let area = Rect { x: 0.0, y: 0.0, w: VIEW_AREA.0, h: VIEW_AREA.1 };
            crop::paint_crop(canvas, pen, at.intersect(&area), view.rect_to_screen(crop_box.rect()).intersect(&area));
        })
        .on_pointer(Msg::ViewPointer)
        .on_scroll(Msg::ViewScroll);
        let (held, panning, whole) = (self.crop_pull.holding(), self.panning && self.zoom_drag.holding(), self.crop_is_whole());
        pic.cursor_with(move |x, y| {
            if panning {
                return Cursor::Move;
            }
            if let Some(h) = held {
                return h.cursor();
            }
            let (cx, cy) = view.to_content(x, y);
            match crop_box.handle_at(cx, cy, crop::GRAB / view.scale()) {
                Some(crop::Handle::Body) if whole => pan_or_arrow(view),
                Some(h) => h.cursor(),
                None => pan_or_arrow(view),
            }
        })
    }

    fn viewer_side(&self) -> Node<Msg> {
        let (l, t, r, b) = self.draft;
        let slide = |name: &str, spoken: &str, v: f32, side: usize| -> Node<Msg> {
            row(vec![form_label(t!("scan-view-crop-value", name = name, value = (v * 100.0).round() as i32)), slider(v * 100.0, 0.0, 45.0, 1.0, spoken, move |x| Msg::Crop(side, x))]).gap(12.0)
        };
        column(vec![
            side_heading(t!("scan-view-zoom-heading")),
            row(vec![word_button(t!("scan-view-zoom-in")).on_press(Msg::Zoom(ZoomBy::In)), word_button(t!("scan-view-zoom-out")).on_press(Msg::Zoom(ZoomBy::Out))]).gap(10.0),
            row(vec![word_button(t!("scan-view-zoom-fit")).on_press(Msg::Zoom(ZoomBy::Fit)), word_button(t!("scan-view-zoom-actual")).on_press(Msg::Zoom(ZoomBy::Actual))]).gap(10.0),
            note(t!("scan-view-zoom-note")),
            side_heading(t!("scan-view-crop-heading")),
            note(t!("scan-view-crop-note")),
            form_label(t!("scan-view-crop-shape")),
            choice(
                vec![
                    t!("scan-view-crop-shape-free").into(),
                    t!("scan-view-crop-shape-square").into(),
                    t!("scan-view-crop-shape-4-3").into(),
                    t!("scan-view-crop-shape-3-2").into(),
                    t!("scan-view-crop-shape-16-9").into(),
                ],
                self.ratio,
                t!("scan-view-crop-shape"),
                Msg::CropRatio,
            ),
            slide(t!("scan-view-crop-left"), t!("scan-view-crop-left-spoken"), l, 0),
            slide(t!("scan-view-crop-top"), t!("scan-view-crop-top-spoken"), t, 1),
            slide(t!("scan-view-crop-right"), t!("scan-view-crop-right-spoken"), r, 2),
            slide(t!("scan-view-crop-bottom"), t!("scan-view-crop-bottom-spoken"), b, 3),
            word_button(t!("scan-view-crop-remove")).on_press(Msg::NoCrop),
            main_word(t!("scan-view-crop-apply")).on_press(Msg::ApplyCrop),
        ])
        .gap(10.0)
    }

    pub(super) fn bar(&self) -> Node<Msg> {
        let scan: Node<Msg> = if let Some((i, n)) = self.reading {
            row(vec![spinner(), label(t!("scan-ocr-reading", i = i, n = n)), word_button(t!("scan-common-cancel")).on_press(Msg::CancelReading)]).gap(12.0)
        } else if let Some(n) = self.batch.pause_left {
            let words = if n == 0 { t!("scan-bar-pause-empty").to_string() } else { t!("scan-bar-pause-wait", n = n) };
            row(vec![
                label(words),
                main_word(if n == 0 { t!("scan-bar-continue") } else { t!("scan-bar-skip-pause") }).on_press(Msg::SkipPause),
                word_button(t!("scan-common-stop")).on_press(Msg::StopBatch),
            ])
            .gap(12.0)
        } else if self.scanning {
            row(vec![spinner(), label(t!("scan-status-scanning", n = self.arrived + 1)), word_button(t!("scan-common-cancel")).on_press(Msg::CancelScan)]).gap(12.0)
        } else if self.more {
            row(vec![label(t!("scan-main-note-another-page")), main_word(t!("scan-bar-scan-next")).on_press(Msg::Scan), word_button(t!("scan-bar-finish")).on_press(Msg::Finish)]).gap(12.0)
        } else {
            main_word(t!("scan-bar-scan")).on_press(Msg::Scan)
        };
        let prof = self.book.now().map(|p| p.name.clone()).unwrap_or_else(|| t!("scan-bar-no-profile").into());
        let top = row(vec![
            scan,
            word_button(prof).on_press(Msg::Profiles),
            word_button(self.scanner().map(|s| s.name.clone()).unwrap_or_else(|| t!("scan-bar-choose-scanner").into())).on_press(Msg::Scanners),
        ])
        .gap(10.0);
        let tools = row(vec![
            main_word(t!("scan-bar-save")).on_press(Msg::Save),
            button("object-rotate-left", t!("scan-bar-rotate-left")).on_press(Msg::RotateLeft),
            button("object-rotate-right", t!("scan-bar-rotate-right")).on_press(Msg::RotateRight),
            button("object-flip-horizontal", t!("scan-bar-flip")).on_press(Msg::Flip),
            button("zoom-in", t!("scan-bar-look")).on_press(Msg::Look),
            button("edit-undo", t!("scan-bar-undo")).on_press(Msg::Undo).on(self.list.can_undo()),
            button("edit-redo", t!("scan-bar-redo")).on_press(Msg::Redo).on(self.list.can_redo()),
            button("edit-delete", t!("scan-bar-delete")).on_press(Msg::Delete),
        ])
        .gap(10.0);
        let mut col = vec![top, tools];
        if let Some(p) = &self.problem {
            let mut r = vec![label(p.clone()).tone(Tone::Problem)];
            if self.no_paper {
                r.push(word_button(t!("scan-bar-try-again")).on_press(Msg::TryAgain));
                r.push(word_button(t!("scan-bar-use-glass")).on_press(Msg::UseGlass));
            }
            if self.offline {
                r.push(word_button(t!("scan-bar-search-again")).on_press(Msg::Scanners));
                r.push(word_button(t!("scan-bar-choose-another")).on_press(Msg::Scanners));
            }
            if self.busy.is_some() {
                r.push(word_button(t!("scan-common-stop")).on_press(Msg::StopRetry));
            }
            col.push(row(r).gap(12.0));
        }
        column(col).gap(8.0).padding(12.0)
    }

    pub(super) fn pages_view(&self) -> Node<Msg> {
        if self.list.pages.is_empty() {
            return column(vec![space(), lead(t!("scan-pages-empty")), space()]).gap(8.0).centred().padding(24.0);
        }
        let n = self.list.pages.len();
        let tiles: Vec<Node<Msg>> = self
            .list
            .pages
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let img = p.thumb.clone();
                let tw = self.settings.thumb as f32;
                let pic =
                    drawing((tw, tw * 4.0 / 3.0), if p.selected { t!("scan-pages-tile-name-selected", i = i + 1, n = n) } else { t!("scan-pages-tile-name", i = i + 1, n = n) }, move |canvas, _| {
                        let (x, y, w, h) = fitted((canvas.width(), canvas.height()), &img);
                        picture::draw(canvas, &img, x, y, w, h);
                    })
                    .on_press(Msg::Click(i));
                let caption = if p.saved { t!("scan-pages-caption-saved", i = i + 1) } else { t!("scan-pages-caption", i = i + 1) };
                card(column(vec![pic, note(caption)]).gap(6.0).padding(8.0)).current(p.selected)
            })
            .collect();
        scroll("pages", even_grid(self.settings.thumb as f32 + 20.0, tiles).padding(16.0).gap(12.0))
    }

    pub(super) fn browse_view(&self) -> Node<Msg> {
        let Some(b) = &self.browse else { return self.pages_view() };
        let mut items: Vec<Node<Msg>> = vec![row(vec![title(b.purpose.title()), space(), word_button(t!("scan-common-cancel")).on_press(Msg::Browse(BrowseMsg::Cancel))]).gap(10.0)];
        let place = match &b.dir {
            Some(d) => d.display().to_string(),
            None => t!("scan-browse-top").to_string(),
        };
        items.push(row(vec![label(place), space(), word_button(t!("scan-browse-up")).on_press(Msg::Browse(BrowseMsg::Up)).on(b.dir.is_some())]).gap(10.0));
        let shown: Vec<Item> = b
            .listing
            .iter()
            .map(|r| {
                let name = r.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if r.folder {
                    item(name).icon("folder").detail(t!("scan-browse-kind-folder"))
                } else if b.picked.contains(&r.path) {
                    item(name).icon("image-x-generic").detail(t!("scan-browse-kind-chosen"))
                } else {
                    item(name).icon("image-x-generic").detail(t!("scan-browse-kind-picture"))
                }
            })
            .collect();
        if shown.is_empty() {
            items.push(note(if b.purpose == Purpose::Pictures { t!("scan-browse-empty-pictures") } else { t!("scan-browse-empty-folders") }));
        } else {
            items.push(list(shown, None, |i| Msg::Browse(BrowseMsg::Pick(i))).height(Length::Shrink));
        }
        if b.dir.is_some() {
            items.push(form_label(t!("scan-browse-new-folder")));
            items.push(
                row(vec![
                    entry(b.new_folder.clone(), t!("scan-browse-new-folder-spoken"), |t| Msg::Browse(BrowseMsg::NewName(t))).on_submit(Msg::Browse(BrowseMsg::MakeFolder)),
                    word_button(t!("scan-browse-make-folder")).on_press(Msg::Browse(BrowseMsg::MakeFolder)),
                ])
                .gap(8.0),
            );
        }
        if b.purpose == Purpose::SavePdf {
            items.push(form_label(t!("scan-browse-file-name")));
            items.push(entry(b.name.clone(), t!("scan-browse-file-name"), |t| Msg::Browse(BrowseMsg::Name(t))).on_submit(Msg::Browse(BrowseMsg::Confirm)));
        }
        if let Some(e) = &b.error {
            items.push(label(e.clone()).tone(Tone::Problem));
        }
        let go = match b.purpose {
            Purpose::SavePdf => t!("scan-browse-go-save").to_string(),
            Purpose::Pictures => t!("scan-browse-go-pictures", n = b.picked.len()),
            _ => t!("scan-browse-go-folder").to_string(),
        };
        items.push(main_word(go).on_press(Msg::Browse(BrowseMsg::Confirm)));
        scroll("browse", column(items).gap(10.0).padding(24.0))
    }

    pub(super) fn closing_view(&self) -> Node<Msg> {
        let n = self.list.pages.iter().filter(|p| !p.saved).count();
        let mut items: Vec<Node<Msg>> = vec![title(t!("scan-close-title"))];
        if n > 0 {
            items.push(label(t!("scan-close-pages", n = n)));
        }
        if self.scanning {
            items.push(label(t!("scan-close-scanning")));
        }
        items.push(
            row(vec![
                main_word(t!("scan-bar-save")).on_press(Msg::CloseSave),
                word_button(t!("scan-close-discard")).on_press(Msg::CloseDiscard),
                word_button(t!("scan-close-keep")).on_press(Msg::Back),
            ])
            .gap(16.0),
        );
        column(items).gap(14.0).padding(24.0)
    }

    pub(super) fn shortcuts_view(&self) -> Node<Msg> {
        let keys: [(&str, &str); 9] = [
            (t!("scan-shortcuts-profile-keys"), t!("scan-shortcuts-profile-what")),
            (t!("scan-shortcuts-select-keys"), t!("scan-shortcuts-select-what")),
            (t!("scan-shortcuts-delete-keys"), t!("scan-shortcuts-delete-what")),
            (t!("scan-shortcuts-undo-keys"), t!("scan-shortcuts-undo-what")),
            (t!("scan-shortcuts-redo-keys"), t!("scan-shortcuts-redo-what")),
            (t!("scan-shortcuts-earlier-keys"), t!("scan-shortcuts-earlier-what")),
            (t!("scan-shortcuts-later-keys"), t!("scan-shortcuts-later-what")),
            (t!("scan-shortcuts-tab-keys"), t!("scan-shortcuts-tab-what")),
            (t!("scan-shortcuts-press-keys"), t!("scan-shortcuts-press-what")),
        ];
        let mut items: Vec<Node<Msg>> = vec![row(vec![title(t!("scan-shortcuts-title")), space(), main_word(t!("scan-common-done")).on_press(Msg::Back)])];
        for (k, what) in keys {
            items.push(card(row(vec![label(k), space(), note(what)]).gap(16.0).padding(10.0)));
        }
        items.push(note(t!("scan-shortcuts-note")));
        scroll("shortcuts", column(items).gap(10.0).padding(24.0))
    }

    pub(super) fn settings_view(&self) -> Node<Msg> {
        let st = &self.settings;
        let place = |text: &str, empty: &str| if text.is_empty() { empty.to_string() } else { text.to_string() };
        let items: Vec<Node<Msg>> = vec![
            row(vec![title(t!("scan-settings-title")), space(), main_word(t!("scan-common-done")).on_press(Msg::Back)]),
            form_label(t!("scan-settings-folder")),
            row(vec![label(place(&st.folder, t!("scan-settings-no-folder"))), space(), word_button(t!("scan-common-choose")).on_press(Msg::AskSettingsFolder(false))]).gap(10.0),
            form_label(t!("scan-settings-pattern")),
            entry(st.pattern.clone(), t!("scan-settings-pattern-spoken"), |t| Msg::Set(Set::Pattern(t))),
            form_label(t!("scan-settings-thumb", n = st.thumb)),
            slider(st.thumb as f32, settings::THUMB_MIN as f32, settings::THUMB_MAX as f32, 10.0, t!("scan-settings-thumb-spoken"), |v| Msg::Set(Set::Thumb(v))),
            form_label(t!("scan-settings-units")),
            choice(vec![t!("scan-settings-units-mm").into(), t!("scan-settings-units-in").into()], if st.units == Units::Inches { 1 } else { 0 }, t!("scan-settings-units-spoken"), |i| {
                Msg::Set(Set::Units(i))
            }),
            note(t!("scan-settings-example", size = &st.size_words(Size::A4))),
            row(vec![label(t!("scan-settings-remove-saved")), space(), switch(st.remove_saved, t!("scan-settings-remove-saved-spoken"), |b| Msg::Set(Set::RemoveSaved(b)))]),
            section(t!("scan-settings-pdf-heading")),
            form_label(t!("scan-settings-pdf-title")),
            entry(st.title.clone(), t!("scan-settings-pdf-title-spoken"), |t| Msg::Set(Set::Title(t))),
            form_label(t!("scan-settings-pdf-author")),
            entry(st.author.clone(), t!("scan-settings-pdf-author-spoken"), |t| Msg::Set(Set::Author(t))),
            form_label(t!("scan-settings-pdf-subject")),
            entry(st.subject.clone(), t!("scan-settings-pdf-subject-spoken"), |t| Msg::Set(Set::Subject(t))),
            form_label(t!("scan-settings-pdf-keywords")),
            entry(st.keywords.clone(), t!("scan-settings-pdf-keywords-spoken"), |t| Msg::Set(Set::Keywords(t))),
            form_label(t!("scan-settings-pdf-split")),
            choice(
                settings::SPLITS.iter().map(|n| if *n == 0 { t!("scan-settings-pdf-split-all").to_string() } else { n.to_string() }).collect(),
                settings::SPLITS.iter().position(|n| *n == st.split_every).unwrap_or(0),
                t!("scan-settings-pdf-split-spoken"),
                |i| Msg::Set(Set::Split(i)),
            ),
            form_label(t!("scan-settings-pdf-password")),
            entry(st.password.clone(), t!("scan-settings-pdf-password-spoken"), |t| Msg::Set(Set::Password(t))).secret().key("pass"),
            note(t!("scan-settings-pdf-password-note")),
            section(t!("scan-settings-progress-heading")),
            form_label(t!("scan-settings-temp")),
            row(vec![label(place(&st.temp_folder, t!("scan-settings-temp-default"))), space(), word_button(t!("scan-common-choose")).on_press(Msg::AskSettingsFolder(true))]).gap(10.0),
            note(t!("scan-settings-temp-note")),
            note(t!("scan-common-changes-save")),
        ];
        scroll("settings", column(items).gap(10.0).padding(24.0))
    }

    pub(super) fn scanners_view(&self) -> Node<Msg> {
        let mut items: Vec<Node<Msg>> = vec![row(vec![title(t!("scan-bar-choose-scanner")), space(), word_button(t!("scan-common-back")).on_press(Msg::Back)])];
        if self.searching {
            items.push(row(vec![spinner(), label(t!("scan-status-searching")), word_button(t!("scan-common-stop")).on_press(Msg::StopSearch)]).gap(10.0));
        }
        let rows: Vec<Item> =
            self.scanners.iter().map(|s| item(s.name.clone()).detail(if s.usb_device().is_some() { t!("scan-scanners-usb").to_string() } else { format!("{}:{}", s.host, s.port) })).collect();
        if !rows.is_empty() {
            items.push(list(rows, self.chosen, Msg::Pick).height(Length::Shrink));
        } else if !self.searching {
            items.push(note(t!("scan-scanners-none")));
        }
        if !self.searching {
            items.push(word_button(t!("scan-bar-search-again")).on_press(Msg::SearchAgain));
        }
        items.push(form_label(t!("scan-scanners-address")));
        items.push(
            row(vec![entry(self.address.clone(), t!("scan-scanners-address-spoken"), Msg::Address).on_submit(Msg::AddAddress), word_button(t!("scan-scanners-add")).on_press(Msg::AddAddress)])
                .gap(8.0),
        );
        items.push(note(t!("scan-scanners-address-note")));
        items.push(space());
        column(items).gap(12.0).padding(24.0)
    }

    pub(super) fn presets_view(&self) -> Node<Msg> {
        let mut items = vec![row(vec![title(t!("scan-presets-title")), space(), word_button(t!("scan-common-back")).on_press(Msg::Back)]), note(t!("scan-presets-note"))];
        for (i, p) in Profile::presets().iter().enumerate() {
            items.push(row(vec![label(p.name.clone()), space(), main_word(t!("scan-common-use")).on_press(Msg::Preset(i))]).gap(12.0));
        }
        items.push(space());
        column(items).gap(14.0).padding(24.0)
    }

    pub(super) fn profiles_view(&self) -> Node<Msg> {
        let mut items = vec![
            row(vec![title(t!("scan-profiles-title")), space(), word_button(t!("scan-profiles-new")).on_press(Msg::NewProfile), word_button(t!("scan-common-back")).on_press(Msg::Back)]).gap(10.0),
        ];
        for (i, p) in self.book.profiles.iter().enumerate() {
            let mut name = p.name.clone();
            if i == self.book.default {
                name = t!("scan-profiles-default-mark", name = &name);
            }
            let line = row(vec![
                column(vec![label(name), note(p.scanner.clone())]).gap(2.0),
                space(),
                word_button(t!("scan-common-use")).on_press(Msg::UseProfile(i)),
                word_button(t!("scan-profiles-edit")).on_press(Msg::EditProfile(i)),
                button("go-up", t!("scan-profiles-up")).on_press(Msg::MoveProfile(i, -1)),
                button("go-down", t!("scan-profiles-down")).on_press(Msg::MoveProfile(i, 1)),
                button("edit-copy", t!("scan-profiles-duplicate")).on_press(Msg::DuplicateProfile(i)),
                word_button(t!("scan-profiles-make-default")).on_press(Msg::DefaultProfile(i)),
                remove("edit-delete", t!("scan-profiles-delete")).on_press(Msg::DeleteProfile(i)),
            ])
            .gap(8.0);
            items.push(card(line.padding(10.0)).current(i == self.book.current));
        }
        items.push(space());
        scroll("profiles", column(items).gap(10.0).padding(24.0))
    }

    pub(super) fn editor_view(&self) -> Node<Msg> {
        let Some(p) = self.book.now() else { return self.profiles_view() };
        let mut items: Vec<Node<Msg>> = vec![
            row(vec![title(t!("scan-editor-title")), space(), main_word(t!("scan-common-done")).on_press(Msg::Back)]),
            form_label(t!("scan-editor-name")),
            entry(p.name.clone(), t!("scan-editor-name-spoken"), |t| Msg::Edit(Edit::Name(t))),
        ];
        if let Some(err) = &self.name_error {
            items.push(label(err.clone()).tone(Tone::Problem));
        }
        self.editor_test(&mut items);
        editor_paper(self, p, &mut items);
        editor_tone(p, &mut items);
        editor_batch(p, &mut items);
        items.push(section(t!("scan-editor-after-heading")));
        items.push(row(vec![label(t!("scan-editor-auto-save")), space(), switch(p.auto_save, t!("scan-editor-auto-save"), |b| Msg::Edit(Edit::AutoSave(b)))]));
        items.push(row(vec![label(t!("scan-editor-searchable")), space(), switch(p.searchable, t!("scan-editor-searchable"), |b| Msg::Edit(Edit::Searchable(b)))]));
        self.editor_search(p, &mut items);
        editor_files(p, &mut items);
        items.push(note(t!("scan-common-changes-save")));
        scroll("editor", column(items).gap(10.0).padding(24.0))
    }

    fn editor_test(&self, items: &mut Vec<Node<Msg>>) {
        items.push(section(t!("scan-editor-test-heading")));
        if self.testing {
            items.push(row(vec![spinner(), label(t!("scan-editor-test-busy"))]).gap(10.0));
        } else {
            items.push(row(vec![word_button(t!("scan-editor-test-heading")).on_press(Msg::TestScan), note(t!("scan-editor-test-note"))]).gap(12.0));
        }
        if let Some(e) = &self.test_problem {
            items.push(label(e.clone()).tone(Tone::Problem));
        }
        if let Some(img) = self.test_img.clone() {
            let (w, h) = img.dimensions();
            items.push(note(t!("scan-editor-test-size", w = w, h = h)));
            items.push(drawing((420.0, 520.0), t!("scan-editor-test-picture"), move |canvas, _| {
                let (x, y, w, h) = fitted((canvas.width(), canvas.height()), &img);
                picture::draw(canvas, &img, x, y, w, h);
            }));
        }
    }

    fn editor_search(&self, p: &Profile, items: &mut Vec<Node<Msg>>) {
        if p.searchable {
            if self.ocr_langs.is_empty() {
                items.push(note(t!("scan-ocr-missing")));
            } else {
                let mut parts = p.ocr_lang.split('+');
                let (first, second) = (parts.next().unwrap_or("eng"), parts.next());
                let names: Vec<String> = self.ocr_langs.iter().map(|l| ocr::language_name(l)).collect();
                items.push(form_label(t!("scan-editor-ocr-language")));
                items.push(choice(names, self.ocr_langs.iter().position(|l| l == first).unwrap_or(0), t!("scan-editor-ocr-language"), |i| Msg::Edit(Edit::OcrLang(i))));
                let others: Vec<&String> = self.ocr_langs.iter().filter(|l| *l != first).collect();
                if !others.is_empty() {
                    let mut choices = vec![t!("scan-editor-ocr-none").to_string()];
                    choices.extend(others.iter().map(|l| ocr::language_name(l)));
                    items.push(form_label(t!("scan-editor-ocr-second")));
                    items.push(choice(choices, others.iter().position(|l| Some(l.as_str()) == second).map(|k| k + 1).unwrap_or(0), t!("scan-editor-ocr-second"), |i| Msg::Edit(Edit::OcrLang2(i))));
                }
            }
            items.push(note(t!("scan-editor-ocr-note")));
        }
    }
}

fn editor_paper(app: &Scan, p: &Profile, items: &mut Vec<Node<Msg>>) {
    let source = match p.source {
        Source::Glass => 0,
        Source::Feeder => 1,
        Source::FeederBoth => 2,
        Source::FeederManualBoth => 3,
    };
    let can_feeder = app.caps.as_ref().map(|c| c.feeder).unwrap_or(true);
    let can_two = app.caps.as_ref().map(|c| c.two_sided).unwrap_or(true);
    let mut sources: Vec<String> = vec![t!("scan-editor-source-glass").into()];
    let mut source_ix: Vec<usize> = vec![0];
    if can_feeder {
        sources.push(t!("scan-editor-source-feeder").into());
        source_ix.push(1);
        if can_two {
            sources.push(t!("scan-editor-source-both").into());
            source_ix.push(2);
        }
        sources.push(t!("scan-editor-source-manual").into());
        source_ix.push(3);
    }
    let chosen_source = source_ix.iter().position(|s| *s == source).unwrap_or(0);
    let dpis: Vec<u32> = match &app.caps {
        Some(c) if !c.resolutions.is_empty() => DPIS.iter().copied().filter(|d| c.resolutions.contains(d)).collect(),
        _ => DPIS.to_vec(),
    };
    let dpi_ix = dpis.iter().position(|d| *d == p.dpi).unwrap_or(0);
    let si = source_ix.clone();
    let di = dpis.clone();
    items.push(form_label(t!("scan-editor-source")));
    items.push(one_of(sources, Some(chosen_source), t!("scan-editor-source"), move |i| Msg::Edit(Edit::Source(si[i]))));
    items.push(form_label(t!("scan-editor-size")));
    items.push(one_of(Size::ALL.iter().map(|s| app.settings.size_words(*s)).collect(), Size::ALL.iter().position(|s| *s == p.size), t!("scan-editor-size"), |i| Msg::Edit(Edit::Size(i))));
    items.push(form_label(t!("scan-editor-dpi")));
    items.push(one_of(dpis.iter().map(|d| d.to_string()).collect(), Some(dpi_ix), t!("scan-editor-dpi-spoken"), move |i| Msg::Edit(Edit::Dpi(DPIS.iter().position(|d| *d == di[i]).unwrap_or(4)))));
    if p.dpi >= 600 {
        items.push(note(t!("scan-editor-dpi-note")));
    }
}

fn editor_tone(p: &Profile, items: &mut Vec<Node<Msg>>) {
    items.push(form_label(t!("scan-editor-colour")));
    items.push(choice(
        vec![t!("scan-editor-colour-colour").into(), t!("scan-editor-colour-grey").into(), t!("scan-editor-colour-bw").into()],
        match p.colour {
            Colour::Full => 0,
            Colour::Grey => 1,
            Colour::BlackWhite => 2,
        },
        t!("scan-editor-colour-mode"),
        |i| Msg::Edit(Edit::Colour(i)),
    ));
    if p.colour == Colour::BlackWhite {
        items.push(form_label(t!("scan-editor-threshold", n = p.threshold)));
        items.push(slider(p.threshold as f32, 0.0, 100.0, 1.0, t!("scan-editor-threshold-spoken"), |v| Msg::Edit(Edit::Threshold(v))));
    }
    items.push(form_label(t!("scan-editor-brightness", n = p.brightness)));
    items.push(slider(p.brightness as f32, -100.0, 100.0, 1.0, t!("scan-editor-brightness-spoken"), |v| Msg::Edit(Edit::Brightness(v))));
    items.push(form_label(t!("scan-editor-contrast", n = p.contrast)));
    items.push(slider(p.contrast as f32, -100.0, 100.0, 1.0, t!("scan-editor-contrast-spoken"), |v| Msg::Edit(Edit::Contrast(v))));
    items.push(word_button(t!("scan-editor-reset-tone")).on_press(Msg::Edit(Edit::ResetTone)));
    items.push(form_label(t!("scan-editor-rotate")));
    items.push(choice(
        vec![t!("scan-editor-rotate-none").into(), "90".into(), "180".into(), "270".into()],
        [0u16, 90, 180, 270].iter().position(|r| *r == p.rotate).unwrap_or(0),
        t!("scan-editor-rotate-spoken"),
        |i| Msg::Edit(Edit::Rotate(i)),
    ));
    items.push(row(vec![label(t!("scan-editor-straighten")), space(), switch(p.straighten, t!("scan-editor-straighten"), |b| Msg::Edit(Edit::Straighten(b)))]));
    items.push(row(vec![label(t!("scan-editor-trim")), space(), switch(p.trim, t!("scan-editor-trim-spoken"), |b| Msg::Edit(Edit::Trim(b)))]));
    items.push(row(vec![label(t!("scan-editor-skip-blank")), space(), switch(p.skip_blank, t!("scan-editor-skip-blank"), |b| Msg::Edit(Edit::SkipBlank(b)))]));
    if p.source == Source::Glass {
        items.push(row(vec![label(t!("scan-editor-ask-more")), space(), switch(p.ask_more, t!("scan-editor-ask-more-spoken"), |b| Msg::Edit(Edit::AskMore(b)))]));
    }
}

fn editor_batch(p: &Profile, items: &mut Vec<Node<Msg>>) {
    items.push(section(t!("scan-editor-batch-heading")));
    items.push(choice(vec![t!("scan-editor-batch-one").into(), t!("scan-editor-batch-several").into(), t!("scan-editor-batch-until").into()], p.batch as usize, t!("scan-editor-batch-spoken"), |i| {
        Msg::Edit(Edit::Batch(i))
    }));
    if p.batch == 1 {
        items.push(form_label(t!("scan-editor-runs")));
        items.push(choice(RUNS.iter().map(|r| r.to_string()).collect(), RUNS.iter().position(|r| *r == p.runs).unwrap_or(0), t!("scan-editor-runs"), |i| Msg::Edit(Edit::Runs(i))));
    }
    if p.batch > 0 {
        items.push(form_label(t!("scan-editor-pause")));
        items.push(choice(
            PAUSES.iter().map(|r| if *r == 0 { t!("scan-editor-pause-wait").to_string() } else { r.to_string() }).collect(),
            PAUSES.iter().position(|r| *r == p.pause).unwrap_or(0),
            t!("scan-editor-pause-spoken"),
            |i| Msg::Edit(Edit::Pause(i)),
        ));
        items.push(row(vec![label(t!("scan-editor-one-file")), space(), switch(p.one_file, t!("scan-editor-one-file"), |b| Msg::Edit(Edit::OneFile(b)))]));
    }
}

fn editor_files(p: &Profile, items: &mut Vec<Node<Msg>>) {
    if p.auto_save {
        items.push(form_label(t!("scan-editor-folder")));
        items.push(
            row(vec![
                label(if p.folder.is_empty() { t!("scan-settings-no-folder").to_string() } else { p.folder.clone() }),
                space(),
                word_button(t!("scan-common-choose")).on_press(Msg::ChooseFolder),
            ])
            .gap(10.0),
        );
        items.push(form_label(t!("scan-editor-file-type")));
        items.push(choice(Format::ALL.iter().map(|f| f.words().to_string()).collect(), Format::ALL.iter().position(|f| *f == p.format).unwrap_or(0), t!("scan-editor-file-type"), |i| {
            Msg::Edit(Edit::Format(i))
        }));
        items.push(form_label(t!("scan-browse-file-name")));
        items.push(entry(p.pattern.clone(), t!("scan-editor-pattern-spoken"), |t| Msg::Edit(Edit::Pattern(t))));
        let example = naming::name_from(p.pattern.split('/').next_back().unwrap_or(""), &p.name, &naming::now(), 1);
        items.push(note(t!("scan-editor-next-file", name = &format!("{example}.{}", p.format.ext()))));
        items.push(note(t!("scan-editor-parts", parts = "{year} {month} {day} {hour} {minute} {second} {counter:3} {profile}")));
        items.push(row(vec![label(t!("scan-editor-clear-after")), space(), switch(p.clear_after, t!("scan-editor-clear-after-spoken"), |b| Msg::Edit(Edit::ClearAfter(b)))]));
    }
}

fn pan_or_arrow(view: View) -> Cursor {
    if view.can_pan() { Cursor::Move } else { Cursor::Arrow }
}

fn fitted(canvas: (u32, u32), img: &DynamicImage) -> (i32, i32, u32, u32) {
    let (iw, ih) = img.dimensions();
    let s = (canvas.0 as f32 / iw as f32).min(canvas.1 as f32 / ih as f32);
    let (w, h) = (((iw as f32 * s) as u32).max(1), ((ih as f32 * s) as u32).max(1));
    (((canvas.0 - w.min(canvas.0)) / 2) as i32, ((canvas.1 - h.min(canvas.1)) / 2) as i32, w, h)
}

#[cfg(test)]
mod look_tests {
    const SOURCES: [(&str, &str); 3] = [("views.rs", include_str!("views.rs")), ("viewer.rs", include_str!("viewer.rs")), ("main.rs", include_str!("main.rs"))];

    #[test]
    fn no_part_carries_its_own_corner_or_button_size() {
        let banned = ["panel_radius", "control_radius", "row_radius", "float_radius", "ring_corner", "border::radius", "Radius::", "corner_radius", "button_height"];
        for (file, text) in SOURCES {
            let code = text.split("#[cfg(test)]").next().unwrap_or(text);
            for word in banned {
                assert!(!code.contains(word), "{file} has {word}");
            }
        }
    }

    #[test]
    fn viewer_side_panel_headings_are_side_headings() {
        let code = SOURCES[0].1.split("#[cfg(test)]").next().unwrap_or("");
        let panel = code.split("fn viewer_side(").nth(1).and_then(|s| s.split("pub(super) fn bar").next()).unwrap_or("");
        let side_call = ["side_", "heading", "("].concat();
        assert_eq!(panel.matches(&side_call).count(), 2);
        let section_call = ["sec", "tion", "("].concat();
        assert_eq!(panel.matches(&section_call).count(), 0);
    }

    #[test]
    fn a_picture_is_fitted_and_centred() {
        let img = super::DynamicImage::new_rgb8(100, 200);
        assert_eq!(super::fitted((200, 200), &img), (50, 0, 100, 200));
        assert_eq!(super::fitted((100, 400), &img), (0, 100, 100, 200));
    }
}
