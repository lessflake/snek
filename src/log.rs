use std::path::{Path, PathBuf};

use crate::target::Target;

#[derive(Debug, Clone)]
pub struct Log {
    target: Target,
    path: PathBuf,
}

impl Log {
    pub fn from_file(path: impl AsRef<Path>) -> Option<Self> {
        use crate::parse;

        // let target_name = path.parent()?.file_stem()?.to_str()?;
        let target_id = parse::target_id(&path)?;
        Target::from_id(target_id).map(|target| Self {
            target,
            path: path.as_ref().to_owned(),
        })
    }

    pub fn validate(path: impl AsRef<Path>) -> bool {
        if let Some(file_ext) = path.as_ref().extension() {
            file_ext.to_str() == Some("zevtc") || file_ext.to_str() == Some("evtc")
        } else {
            false
        }
    }

    pub fn from_file_checked(path: impl AsRef<Path>) -> Option<Self> {
        if Self::validate(&path) {
            Self::from_file(path)
        } else {
            None
        }
    }

    pub const fn target(&self) -> Target {
        self.target
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn id(&self) -> String {
        self.path
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap()
    }

    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap()
    }

    // link format: https://dps.report/P8NN-20200407-185643_arkk
    // id format:                           20200407-185643
    //                           compare id ^^^^^^^^^^^^^^^
    pub fn same_as(&self, link: &str) -> bool {
        self.id()[0..15] == link[24..39]
    }
}

impl std::fmt::Display for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} @ {}", self.target(), self.file_name())
    }
}
