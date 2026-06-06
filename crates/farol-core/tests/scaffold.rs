use std::fs;

use tempfile::TempDir;

use farol_core::FarolError;
use farol_core::scaffold::scaffold;

#[test]
fn scaffolds_minimal_project() {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path().join("demo");
    scaffold(&target).unwrap();

    assert!(target.join("farol.toml").exists());
    assert!(target.join("docs").join("index.md").exists());
    assert!(target.join("docs").join("getting-started.md").exists());
}

#[test]
fn refuses_non_empty_target() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("existing.txt"), "").unwrap();
    let err = scaffold(tmp.path()).unwrap_err();
    assert!(matches!(err, FarolError::ScaffoldExists { .. }));
}
