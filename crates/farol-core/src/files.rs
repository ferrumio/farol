use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use ignore::{DirEntry, WalkBuilder};

use crate::error::{FarolError, Result};

/// Classification of a file discovered during a walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileKind {
    Markdown,
    Asset,
}

/// A single file in the source tree.
#[derive(Debug, Clone)]
pub struct File {
    pub path: PathBuf,
    pub relative: PathBuf,
    pub kind: FileKind,
    pub mtime: Option<SystemTime>,
}

/// A collection of files found under the configured `docs_dir`.
#[derive(Debug, Clone, Default)]
pub struct FileTree {
    pub files: Vec<File>,
}

impl FileTree {
    pub fn len(&self) -> usize {
        self.files.len()
    }
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn markdown(&self) -> impl Iterator<Item = &File> {
        self.files.iter().filter(|f| f.kind == FileKind::Markdown)
    }

    pub fn assets(&self) -> impl Iterator<Item = &File> {
        self.files.iter().filter(|f| f.kind == FileKind::Asset)
    }
}

/// Walk `docs_dir` and collect every file we care about. Respects
/// `.gitignore`, `.ignore`, and a local `.farolignore` if present.
pub fn walk(docs_dir: impl AsRef<Path>) -> Result<FileTree> {
    let docs_dir = docs_dir.as_ref();
    if !docs_dir.exists() {
        return Err(FarolError::io(
            docs_dir,
            std::io::Error::new(std::io::ErrorKind::NotFound, "docs_dir not found"),
        ));
    }

    let walker = WalkBuilder::new(docs_dir)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .add_custom_ignore_filename(".farolignore")
        .build();

    let mut files = Vec::new();
    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = %e, "walk error, skipping");
                continue;
            }
        };
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        if let Some(file) = classify(&entry, docs_dir) {
            files.push(file);
        }
    }

    files.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(FileTree { files })
}

fn classify(entry: &DirEntry, root: &Path) -> Option<File> {
    let path = entry.path().to_path_buf();
    let relative = path.strip_prefix(root).ok()?.to_path_buf();
    let kind = match path.extension().and_then(|s| s.to_str()).map(|s| s.to_ascii_lowercase()) {
        Some(ref ext) if ext == "md" || ext == "markdown" => FileKind::Markdown,
        Some(_) => FileKind::Asset,
        None => FileKind::Asset,
    };
    let mtime = entry.metadata().ok().and_then(|m| m.modified().ok());
    Some(File { path, relative, kind, mtime })
}
