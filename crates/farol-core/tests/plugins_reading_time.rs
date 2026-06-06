use std::path::PathBuf;

use farol_core::PluginHost;
use farol_core::config::Config;
use farol_core::frontmatter::Frontmatter;
use farol_core::page::Page;
use farol_core::plugins::core::reading_time::ReadingTimePlugin;

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
        layout: "default".to_string(),
    }
}

#[test]
fn short_page_shows_one_minute() {
    let html = "<p>Hello world</p>".to_string();
    let out = ReadingTimePlugin.on_page_html(html, &page(), &Config::default()).unwrap();
    assert!(out.contains("1 min read"));
}

#[test]
fn longer_page_scales() {
    let words = "word ".repeat(600);
    let html = format!("<p>{words}</p>");
    let out = ReadingTimePlugin.on_page_html(html, &page(), &Config::default()).unwrap();
    assert!(out.contains("3 min read"), "unexpected output: {out}");
}

#[test]
fn marker_is_prepended() {
    let html = "<p>original</p>".to_string();
    let out = ReadingTimePlugin.on_page_html(html, &page(), &Config::default()).unwrap();
    assert!(out.starts_with(r#"<span class="reading-time""#));
    assert!(out.contains("<p>original</p>"));
}
