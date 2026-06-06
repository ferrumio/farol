use std::{fs, path::Path};

use tempfile::TempDir;

use farol_core::assets::copy_asset;

#[test]
fn copies_asset_with_hash() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src.png");
    fs::write(&src, b"data").unwrap();
    let site = tmp.path().join("site");
    let url = copy_asset(&src, Path::new("img/logo.png"), &site, true).unwrap();

    assert!(url.starts_with("/img/logo."));
    assert!(url.ends_with(".png"));
    let final_path = site.join(url.trim_start_matches('/'));
    assert!(final_path.exists());
}

#[test]
fn non_hashed_extension_keeps_name() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("a.pdf");
    fs::write(&src, b"x").unwrap();
    let site = tmp.path().join("site");
    let url = copy_asset(&src, Path::new("docs/a.pdf"), &site, true).unwrap();
    assert_eq!(url, "/docs/a.pdf");
}

#[test]
fn no_hash_when_disabled() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("a.png");
    fs::write(&src, b"x").unwrap();
    let site = tmp.path().join("site");
    let url = copy_asset(&src, Path::new("a.png"), &site, false).unwrap();
    assert_eq!(url, "/a.png");
}
