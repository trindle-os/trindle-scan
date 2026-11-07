//! A temporary folder for a test, named so nobody can guess it and removed when the test ends.

use std::ops::Deref;
use std::path::Path;

pub struct Tmp(tempfile::TempDir);

pub fn tmp(name: &str) -> Tmp {
    Tmp(tempfile::Builder::new().prefix(&format!("trindle-scan-{name}-")).tempdir().unwrap())
}

impl Deref for Tmp {
    type Target = Path;
    fn deref(&self) -> &Path {
        self.0.path()
    }
}

impl AsRef<Path> for Tmp {
    fn as_ref(&self) -> &Path {
        self.0.path()
    }
}
