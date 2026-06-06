use std::path::PathBuf;

use farol_core::PluginHost;
use farol_core::config::Config;
use farol_core::frontmatter::Frontmatter;
use farol_core::page::Page;
use farol_core::plugins::core::anchor_links::AnchorLinksPlugin;

fn page() -> Page {
    Page {
        relative: PathBuf::from("p.md"),
        source_abs: PathBuf::from("/tmp/p.md"),
        url: "/p/".into(),
        output: PathBuf::from("p/index.html"),
        title: "p".into(),
        frontmatter: Frontmatter::new(),
        body_html: String::new(),
        toc: Vec::new(),
        layout: "default".into(),
    }
}

fn inject_anchors(html: &str) -> String {
    AnchorLinksPlugin.on_page_html(html.to_string(), &page(), &Config::default()).unwrap()
}

#[test]
fn injects_id_and_anchor() {
    let html = "<h2>Hello World</h2>";
    let out = inject_anchors(html);
    assert!(out.contains(r#"id="hello-world""#));
    assert!(out.contains(r##"href="#hello-world""##));
    assert!(out.contains("heading-anchor"));
}

#[test]
fn h3_supported() {
    let out = inject_anchors("<h3>Sub</h3>");
    assert!(out.contains(r#"<h3 id="sub">"#));
}

#[test]
fn duplicates_get_suffixes() {
    let out = inject_anchors("<h2>Same</h2><h2>Same</h2>");
    assert!(out.contains(r#"id="same""#));
    assert!(out.contains(r#"id="same-1""#));
}

#[test]
fn preserves_inline_markup() {
    let out = inject_anchors("<h2>Hello <em>world</em></h2>");
    assert!(out.contains(r#"id="hello-world""#));
    assert!(out.contains("<em>world</em>"));
}

#[test]
fn leaves_other_content_alone() {
    let out = inject_anchors("<p>nothing</p>");
    assert_eq!(out, "<p>nothing</p>");
}
