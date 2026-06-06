use std::{fs, path::Path};

use tempfile::TempDir;

use farol_core::FarolError;
use farol_core::files::walk;

fn write(dir: &Path, relative: &str, content: &str) {
    let p = dir.join(relative);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, content).unwrap();
}

#[test]
fn walks_markdown_and_assets() {
    let tmp = TempDir::new().unwrap();
    let docs = tmp.path().join("docs");
    write(&docs, "index.md", "# hi");
    write(&docs, "guide/install.md", "# install");
    write(&docs, "img/logo.png", "fake");

    let tree = walk(&docs).unwrap();
    assert_eq!(tree.len(), 3);
    assert_eq!(tree.markdown().count(), 2);
    assert_eq!(tree.assets().count(), 1);
}

#[test]
fn respects_farolignore() {
    let tmp = TempDir::new().unwrap();
    let docs = tmp.path().join("docs");
    write(&docs, "index.md", "# hi");
    write(&docs, "draft.md", "# wip");
    write(&docs, ".farolignore", "draft.md\n");

    let tree = walk(&docs).unwrap();
    let paths: Vec<_> =
        tree.files.iter().map(|f| f.relative.to_string_lossy().into_owned()).collect();
    assert!(paths.contains(&"index.md".to_string()));
    assert!(!paths.contains(&"draft.md".to_string()));
}

#[test]
fn missing_docs_dir_errors() {
    let tmp = TempDir::new().unwrap();
    let err = walk(tmp.path().join("nope")).unwrap_err();
    assert!(matches!(err, FarolError::Io { .. }));
}
