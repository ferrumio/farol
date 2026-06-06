use std::path::PathBuf;

use farol_core::PluginHost;
use farol_core::config::Config;
use farol_core::frontmatter::Frontmatter;
use farol_core::page::Page;
use farol_core::plugins::core::containers::ContainersPlugin;

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

fn transform(md: &str) -> String {
    ContainersPlugin.on_page_markdown(md.to_string(), &page(), &Config::default()).unwrap()
}

#[test]
fn tabs_wraps_sections() {
    let md = "::: tabs\n### Python\n```python\nprint(1)\n```\n\n### Rust\n```rust\nfn main() {}\n```\n:::\n";
    let out = transform(md);
    assert!(out.contains("farol-tabs"));
    assert!(out.contains(r#"data-tab="0""#));
    assert!(out.contains(r#"data-tab="1""#));
    assert!(out.contains("Python"));
    assert!(out.contains("Rust"));
    // Code blocks remain as markdown fences for the highlight plugin.
    assert!(out.contains("```python"));
    assert!(out.contains("```rust"));
}

#[test]
fn files_wraps_sections() {
    let md = "::: files\n#### config.toml\n```toml\nkey = 1\n```\n\n#### main.py\n```python\nprint(1)\n```\n:::\n";
    let out = transform(md);
    assert!(out.contains("farol-files"));
    assert!(out.contains("file-label"));
    assert!(out.contains("config.toml"));
    assert!(out.contains("main.py"));
}

#[test]
fn unterminated_container_left_alone() {
    let md = "::: tabs\n### Python\n```py\nx\n```\nno-end\n";
    let out = transform(md);
    assert!(out.contains("::: tabs"));
}

#[test]
fn unknown_container_passes_through() {
    let md = "::: whatever\ncontent\n:::\n";
    let out = transform(md);
    assert!(out.contains("::: whatever"));
}

#[test]
fn nested_content_preserved() {
    let md = "before\n::: tabs\n### A\nhello\n### B\nworld\n:::\nafter\n";
    let out = transform(md);
    assert!(out.starts_with("before"));
    assert!(out.contains("hello"));
    assert!(out.contains("world"));
    assert!(out.trim_end().ends_with("after"));
}
