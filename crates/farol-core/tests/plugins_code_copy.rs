use std::path::PathBuf;

use farol_core::PluginHost;
use farol_core::config::Config;
use farol_core::frontmatter::Frontmatter;
use farol_core::page::Page;
use farol_core::plugins::core::code_copy::CodeCopyPlugin;

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

fn inject_buttons(html: &str) -> String {
    CodeCopyPlugin.on_page_html(html.to_string(), &page(), &Config::default()).unwrap()
}

#[test]
fn wraps_pre_and_adds_button() {
    let html = "<pre><code>x</code></pre>";
    let out = inject_buttons(html);
    assert!(out.starts_with(r#"<div class="code-copy-wrap">"#));
    assert!(out.contains(r#"class="code-copy""#));
    assert!(out.contains("<pre><code>x</code></pre>"));
    assert!(out.ends_with("</div>"));
}

#[test]
fn multiple_pre_blocks() {
    let html = "<pre>a</pre>text<pre>b</pre>";
    let out = inject_buttons(html);
    assert_eq!(out.matches("code-copy-wrap").count(), 2);
    assert!(out.contains("text"));
}

#[test]
fn unterminated_pre_left_as_is() {
    let html = "<pre>no end";
    let out = inject_buttons(html);
    assert!(out.contains("<pre>no end"));
}

#[test]
fn no_pre_no_change() {
    assert_eq!(inject_buttons("<p>hi</p>"), "<p>hi</p>");
}
