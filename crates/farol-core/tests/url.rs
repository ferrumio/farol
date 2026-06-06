use std::path::{Path, PathBuf};

use farol_core::url::{LinkKind, classify_link, output_path_for, resolve_internal, site_url_for};

#[test]
fn root_index() {
    assert_eq!(site_url_for(Path::new("index.md")), "/");
}

#[test]
fn pretty_url_for_page() {
    assert_eq!(site_url_for(Path::new("guide/install.md")), "/guide/install/");
}

#[test]
fn nested_index() {
    assert_eq!(site_url_for(Path::new("guide/index.md")), "/guide/");
}

#[test]
fn preserves_assets() {
    assert_eq!(site_url_for(Path::new("img/logo.png")), "/img/logo.png/");
}

#[test]
fn output_paths() {
    assert_eq!(output_path_for("/"), PathBuf::from("index.html"));
    assert_eq!(output_path_for("/guide/install/"), PathBuf::from("guide/install/index.html"));
}

#[test]
fn classifies_links() {
    assert_eq!(classify_link("https://example.com"), LinkKind::External);
    assert_eq!(classify_link("mailto:a@b.com"), LinkKind::External);
    assert_eq!(classify_link("#section"), LinkKind::Anchor);
    assert_eq!(classify_link("./other.md"), LinkKind::InternalMarkdown("./other.md"));
    assert_eq!(classify_link("../img/logo.png"), LinkKind::InternalOther("../img/logo.png"));
}

#[test]
fn resolves_relative_markdown() {
    let (rel, anchor) = resolve_internal(Path::new("guide/install.md"), "../index.md").unwrap();
    assert_eq!(rel, PathBuf::from("index.md"));
    assert_eq!(anchor, None);
}

#[test]
fn resolves_with_anchor() {
    let (rel, anchor) = resolve_internal(Path::new("guide/install.md"), "config.md#env").unwrap();
    assert_eq!(rel, PathBuf::from("guide/config.md"));
    assert_eq!(anchor.as_deref(), Some("env"));
}
