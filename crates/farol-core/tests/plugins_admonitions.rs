use std::path::PathBuf;

use farol_core::PluginHost;
use farol_core::config::Config;
use farol_core::frontmatter::Frontmatter;
use farol_core::page::Page;
use farol_core::plugins::core::admonitions::AdmonitionsPlugin;

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

fn transform(html: &str) -> String {
    AdmonitionsPlugin.on_page_html(html.to_string(), &page(), &Config::default()).unwrap()
}

#[test]
fn note_alert_is_transformed() {
    let input = "<blockquote>\n<p>[!NOTE]<br />\nHello world</p>\n</blockquote>";
    let out = transform(input);
    assert!(out.contains(r#"<div class="admonition note">"#));
    assert!(out.contains(r#"<div class="admonition-title">Note</div>"#));
    assert!(out.contains("Hello world"));
}

#[test]
fn unknown_tag_is_left_alone() {
    let input = "<blockquote>\n<p>[!MYSTERY]<br />\nHello</p>\n</blockquote>";
    let out = transform(input);
    assert!(out.contains("<blockquote>"));
    assert!(!out.contains("admonition"));
}

#[test]
fn plain_blockquote_unchanged() {
    let input = "<blockquote>\n<p>normal quote</p>\n</blockquote>";
    let out = transform(input);
    assert_eq!(out, input);
}

#[test]
fn multiple_alerts_transformed() {
    let input = "<blockquote>\n<p>[!NOTE]<br />\nfirst</p>\n</blockquote><p>gap</p>\
        <blockquote>\n<p>[!WARNING]<br />\nsecond</p>\n</blockquote>";
    let out = transform(input);
    assert!(out.contains("admonition note"));
    assert!(out.contains("admonition warning"));
    assert!(out.contains("<p>gap</p>"));
}

#[test]
fn case_insensitive() {
    let input = "<blockquote>\n<p>[!note]<br />\nlower</p>\n</blockquote>";
    let out = transform(input);
    assert!(out.contains("admonition note"));
}
