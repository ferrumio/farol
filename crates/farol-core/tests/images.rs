use std::collections::HashMap;
use std::path::{Path, PathBuf};

use farol_core::images::{ImageIndex, ImageMeta, rewrite_images};

fn meta() -> ImageMeta {
    ImageMeta {
        original_url: "/img/logo.abcd.png".into(),
        webp_url: Some("/img/logo.abcd.webp".into()),
        lqip: "data:image/webp;base64,ZZ".into(),
        width: 200,
        height: 100,
        mime: "image/png",
    }
}

#[test]
fn rewrites_img_with_picture() {
    let mut idx = HashMap::new();
    idx.insert(PathBuf::from("img/logo.png"), meta());
    let html = r#"<p><img src="./img/logo.png" alt="Logo"></p>"#;
    let out = rewrite_images(html, Path::new("index.md"), &idx);
    assert!(out.contains("<picture>"));
    assert!(out.contains("/img/logo.abcd.webp"));
    assert!(out.contains(r#"width="200""#));
    assert!(out.contains(r#"alt="Logo""#));
}

#[test]
fn leaves_absolute_urls_alone() {
    let idx = HashMap::new();
    let html = r#"<img src="https://example.com/x.png">"#;
    assert_eq!(rewrite_images(html, Path::new("index.md"), &idx), html);
}

#[test]
fn leaves_unknown_local_paths_alone() {
    let idx: ImageIndex = HashMap::new();
    let html = r#"<img src="./missing.png">"#;
    assert_eq!(rewrite_images(html, Path::new("index.md"), &idx), html);
}
