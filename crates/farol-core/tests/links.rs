use std::collections::HashMap;
use std::path::{Path, PathBuf};

use farol_core::links::{apply_rewrites, resolve_in_html};

#[test]
fn rewrites_internal_markdown_link() {
    let mut known = HashMap::new();
    known.insert(PathBuf::from("other.md"), "/other/".to_string());

    let html = r#"<p>See <a href="./other.md">here</a>.</p>"#;
    let (rw, broken) = resolve_in_html(Path::new("index.md"), html, &known);
    assert_eq!(rw.len(), 1);
    assert!(broken.is_empty());

    let out = apply_rewrites(html, &rw);
    assert!(out.contains(r#"href="/other/""#));
}

#[test]
fn broken_link_reported() {
    let known = HashMap::new();
    let html = r#"<a href="missing.md">x</a>"#;
    let (_, broken) = resolve_in_html(Path::new("index.md"), html, &known);
    assert_eq!(broken.len(), 1);
}

#[test]
fn external_and_anchor_ignored() {
    let known = HashMap::new();
    let html = r##"<a href="https://x.com">e</a> <a href="#top">t</a>"##;
    let (rw, broken) = resolve_in_html(Path::new("p.md"), html, &known);
    assert!(rw.is_empty());
    assert!(broken.is_empty());
}

#[test]
fn preserves_anchor_in_rewrite() {
    let mut known = HashMap::new();
    known.insert(PathBuf::from("guide.md"), "/guide/".to_string());
    let html = r#"<a href="./guide.md#section">s</a>"#;
    let (rw, _) = resolve_in_html(Path::new("index.md"), html, &known);
    assert_eq!(rw[0].to, "/guide/#section");
}
