//! The program's own picker for folders and files, because a plugin in its box has no file chooser.

use std::path::{Path, PathBuf};
use trindle_toolkit::t;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    SavePdf,
    SaveFolder,
    Pictures,
    ProfileFolder,
    SettingsFolder,
    TempFolder,
}

impl Purpose {
    pub fn title(self) -> &'static str {
        match self {
            Purpose::SavePdf => t!("scan-browse-title-save-pdf"),
            Purpose::SaveFolder => t!("scan-browse-title-save-folder"),
            Purpose::Pictures => t!("scan-browse-title-pictures"),
            Purpose::ProfileFolder => t!("scan-browse-title-profile-folder"),
            Purpose::SettingsFolder => t!("scan-browse-title-settings-folder"),
            Purpose::TempFolder => t!("scan-browse-title-temp-folder"),
        }
    }

    pub fn wants_folder(self) -> bool {
        !matches!(self, Purpose::SavePdf | Purpose::Pictures)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub path: PathBuf,
    pub folder: bool,
}

#[derive(Clone, Debug)]
pub struct Browse {
    pub purpose: Purpose,

    pub dir: Option<PathBuf>,
    pub name: String,
    pub new_folder: String,
    pub picked: Vec<PathBuf>,
    pub error: Option<String>,

    pub listing: Vec<Row>,
}

pub fn roots() -> Vec<PathBuf> {
    trindle_common::files::home().map(|h| roots_in(&h)).unwrap_or_default()
}

pub fn roots_in(home: &Path) -> Vec<PathBuf> {
    ["Documents", "Downloads", "Pictures", "Music", "Videos", "Desktop"].iter().map(|n| home.join(n)).filter(|p| p.is_dir()).collect()
}

pub fn is_picture(p: &Path) -> bool {
    p.extension().map(|e| matches!(e.to_string_lossy().to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "tif" | "tiff")).unwrap_or(false)
}

impl Browse {
    pub fn new(purpose: Purpose, start: Option<PathBuf>, roots: &[PathBuf]) -> Browse {
        let dir = start.filter(|d| d.is_dir() && roots.iter().any(|r| d.starts_with(r)));
        let mut b = Browse { purpose, dir, name: format!("{}.pdf", t!("scan-browse-default-name")), new_folder: String::new(), picked: Vec::new(), error: None, listing: Vec::new() };
        b.listing = b.rows(roots);
        b
    }

    pub fn rows(&self, roots: &[PathBuf]) -> Vec<Row> {
        let Some(dir) = &self.dir else {
            return roots.iter().map(|p| Row { path: p.clone(), folder: true }).collect();
        };
        let mut folders = Vec::new();
        let mut files = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if e.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                if p.is_dir() {
                    folders.push(p);
                } else if self.purpose == Purpose::Pictures && is_picture(&p) {
                    files.push(p);
                }
            }
        }
        folders.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
        files.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
        folders.into_iter().map(|path| Row { path, folder: true }).chain(files.into_iter().map(|path| Row { path, folder: false })).collect()
    }

    pub fn up(&mut self, roots: &[PathBuf]) {
        self.error = None;
        let Some(dir) = self.dir.clone() else { return };
        self.dir = if roots.contains(&dir) { None } else { dir.parent().map(|p| p.to_path_buf()).filter(|p| roots.iter().any(|r| p.starts_with(r))) };
        self.listing = self.rows(roots);
    }

    pub fn pick(&mut self, i: usize, roots: &[PathBuf]) {
        self.error = None;
        let Some(row) = self.listing.get(i).cloned() else { return };
        if row.folder {
            self.dir = Some(row.path);
            self.listing = self.rows(roots);
        } else if let Some(at) = self.picked.iter().position(|p| *p == row.path) {
            self.picked.remove(at);
        } else {
            self.picked.push(row.path);
        }
    }

    pub fn make_folder(&mut self) {
        let name = self.new_folder.trim().to_string();
        let Some(dir) = self.dir.clone() else {
            self.error = Some(t!("scan-browse-error-open-first-new").into());
            return;
        };
        if name.is_empty() || name.contains('/') || name.starts_with('.') {
            self.error = Some(t!("scan-browse-error-folder-name").into());
            return;
        }
        match std::fs::create_dir(dir.join(&name)) {
            Ok(()) => {
                self.dir = Some(dir.join(name));
                self.listing = self.rows(&[]);
                self.new_folder.clear();
                self.error = None;
            }
            Err(e) => self.error = Some(t!("scan-browse-error-make-folder", why = &e.to_string())),
        }
    }

    pub fn confirm(&mut self) -> Option<Vec<PathBuf>> {
        self.error = None;
        match self.purpose {
            Purpose::Pictures => {
                if self.picked.is_empty() {
                    self.error = Some(t!("scan-browse-error-no-pictures").into());
                    None
                } else {
                    Some(self.picked.clone())
                }
            }
            _ => {
                let Some(dir) = self.dir.clone() else {
                    self.error = Some(t!("scan-browse-error-open-first").into());
                    return None;
                };
                if self.purpose.wants_folder() {
                    return Some(vec![dir]);
                }
                let mut name = self.name.trim().to_string();
                if name.is_empty() || name.contains('/') || name.starts_with('.') {
                    self.error = Some(t!("scan-browse-error-file-name").into());
                    return None;
                }
                if !name.to_lowercase().ends_with(".pdf") {
                    name.push_str(".pdf");
                }
                let path = dir.join(name);
                if path.exists() {
                    self.error = Some(t!("scan-browse-error-name-taken").into());
                    return None;
                }
                Some(vec![path])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> crate::testdir::Tmp {
        let d = crate::testdir::tmp(name);
        std::fs::create_dir_all(d.join("Documents/Bills")).unwrap();
        std::fs::create_dir_all(d.join("Pictures")).unwrap();
        d
    }

    #[test]
    fn only_the_usual_folders_can_be_reached() {
        crate::words::ready();
        let home = tmp("roots");
        let roots = roots_in(&home);
        assert_eq!(roots, vec![home.join("Documents"), home.join("Pictures")]);
        let mut b = Browse::new(Purpose::SavePdf, Some(home.join("Documents/Bills")), &roots);
        assert_eq!(b.dir, Some(home.join("Documents/Bills")));
        b.up(&roots);
        assert_eq!(b.dir, Some(home.join("Documents")));
        b.up(&roots);
        assert_eq!(b.dir, None);
        assert!(Browse::new(Purpose::SavePdf, Some(home.to_path_buf()), &roots).dir.is_none());
        assert_eq!(b.rows(&roots).len(), 2);
    }

    #[test]
    fn saving_needs_a_folder_and_a_fresh_plain_name() {
        crate::words::ready();
        let home = tmp("save");
        let roots = roots_in(&home);
        let mut b = Browse::new(Purpose::SavePdf, None, &roots);
        assert!(b.confirm().is_none());
        b.pick(0, &roots);
        b.name = "Bills 2026".into();
        let got = b.confirm().unwrap();
        assert_eq!(got, vec![home.join("Documents/Bills 2026.pdf")]);
        std::fs::write(&got[0], b"x").unwrap();
        assert!(b.confirm().is_none());
        assert!(b.error.is_some());
        b.name = "../x".into();
        assert!(b.confirm().is_none());
    }

    #[test]
    fn the_folder_is_read_when_it_changes_and_not_when_it_is_drawn() {
        crate::words::ready();
        let home = tmp("listing");
        let roots = roots_in(&home);
        let mut b = Browse::new(Purpose::Pictures, Some(home.join("Pictures")), &roots);
        std::fs::write(home.join("Pictures/late.png"), b"x").unwrap();
        assert!(b.listing.is_empty(), "a file added later is not seen until the folder is entered again");
        b.up(&roots);
        b.pick(b.listing.iter().position(|r| r.path.ends_with("Pictures")).unwrap(), &roots);
        assert_eq!(b.listing.len(), 1);
    }

    #[test]
    fn pictures_are_chosen_and_unchosen_and_folders_can_be_made() {
        crate::words::ready();
        let home = tmp("pics");
        let roots = roots_in(&home);
        std::fs::write(home.join("Pictures/a.JPG"), b"x").unwrap();
        std::fs::write(home.join("Pictures/b.txt"), b"x").unwrap();
        std::fs::write(home.join("Pictures/.hidden.png"), b"x").unwrap();
        let mut b = Browse::new(Purpose::Pictures, Some(home.join("Pictures")), &roots);
        let rows = b.rows(&roots);
        assert_eq!(rows.len(), 1);
        b.pick(0, &roots);
        assert_eq!(b.confirm().unwrap(), vec![home.join("Pictures/a.JPG")]);
        b.pick(0, &roots);
        assert!(b.confirm().is_none());
        b.new_folder = "Receipts".into();
        b.make_folder();
        assert_eq!(b.dir, Some(home.join("Pictures/Receipts")));
        b.new_folder = "a/b".into();
        b.make_folder();
        assert!(b.error.is_some());
    }
}
