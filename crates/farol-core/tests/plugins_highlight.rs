use std::path::PathBuf;

use farol_core::PluginHost;
use farol_core::config::Config;
use farol_core::frontmatter::Frontmatter;
use farol_core::page::Page;
use farol_core::plugins::core::highlight::HighlightPlugin;

fn plugin() -> HighlightPlugin {
    HighlightPlugin::new()
}

fn dummy_page() -> Page {
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
fn rewrites_rust_block() {
    let md = "```rust\nfn main() {}\n```\n";
    let out = plugin().on_page_markdown(md.into(), &dummy_page(), &Config::default()).unwrap();
    assert!(out.contains(r#"data-lang="rust""#));
    assert!(out.contains("<pre><code>"));
}

#[test]
fn mark_is_applied_and_stripped() {
    let md = "```python\nx = 1  # !mark\ny = 2\n```\n";
    let out = plugin().on_page_markdown(md.into(), &dummy_page(), &Config::default()).unwrap();
    assert!(out.contains(r#"class="line mark""#));
    assert!(!out.contains("!mark"));
}

#[test]
fn focus_dims_other_lines() {
    let md = "```python\nkeep  # !focus\nhide1\nhide2\n```\n";
    let out = plugin().on_page_markdown(md.into(), &dummy_page(), &Config::default()).unwrap();
    assert!(out.contains(r#"class="line focus""#));
    assert_eq!(out.matches(r#"class="line dim""#).count(), 2);
}

#[test]
fn title_renders_header() {
    let md = "```python title=\"hello.py\"\nprint(1)\n```\n";
    let out = plugin().on_page_markdown(md.into(), &dummy_page(), &Config::default()).unwrap();
    assert!(out.contains(r#"<span class="filename">hello.py</span>"#));
}

#[test]
fn hl_lines_applies_mark() {
    let md = "```python hl_lines=\"2\"\nline1\nline2\n```\n";
    let out = plugin().on_page_markdown(md.into(), &dummy_page(), &Config::default()).unwrap();
    assert_eq!(out.matches(r#"class="line mark""#).count(), 1);
}

#[test]
fn linenums_render() {
    let md = "```python linenums\nline1\nline2\n```\n";
    let out = plugin().on_page_markdown(md.into(), &dummy_page(), &Config::default()).unwrap();
    assert!(out.contains(r#"class="linenum">1</span>"#));
    assert!(out.contains(r#"class="linenum">2</span>"#));
    assert!(out.contains("with-linenums"));
}

#[test]
fn linenums_start_offset() {
    let md = "```python linenums=\"start=10\"\nline1\n```\n";
    let out = plugin().on_page_markdown(md.into(), &dummy_page(), &Config::default()).unwrap();
    assert!(out.contains(r#"class="linenum">10</span>"#));
}

#[test]
fn no_copy_flag_emits_class() {
    let md = "```python no-copy\nx\n```\n";
    let out = plugin().on_page_markdown(md.into(), &dummy_page(), &Config::default()).unwrap();
    assert!(out.contains("no-copy"));
}

#[test]
fn file_include_reads_sibling() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("hello.py"), "print(42)\n").unwrap();
    let page_md = tmp.path().join("page.md");
    std::fs::write(&page_md, "").unwrap();

    let mut page = dummy_page();
    page.source_abs = page_md;

    let md = "```python file=\"./hello.py\"\n```\n";
    let out = plugin().on_page_markdown(md.into(), &page, &Config::default()).unwrap();
    assert!(out.contains("42"));
    assert!(out.contains(r#"<span class="filename">hello.py</span>"#));
}

#[test]
fn file_include_with_region() {
    let tmp = tempfile::TempDir::new().unwrap();
    let source = "header_only\n# region: body\nkeep me\n# endregion: body\ntrailing\n";
    std::fs::write(tmp.path().join("src.py"), source).unwrap();
    let page_md = tmp.path().join("page.md");
    std::fs::write(&page_md, "").unwrap();

    let mut page = dummy_page();
    page.source_abs = page_md;

    let md = "```python file=\"./src.py\" region=\"body\"\n```\n";
    let out = plugin().on_page_markdown(md.into(), &page, &Config::default()).unwrap();
    assert!(out.contains("keep me"));
    assert!(!out.contains("header_only"));
    assert!(!out.contains("trailing"));
    assert!(!out.contains("region:"));
}

#[test]
fn unknown_language_falls_back() {
    let md = "```nosuchlang\nhello\n```\n";
    let out = plugin().on_page_markdown(md.into(), &dummy_page(), &Config::default()).unwrap();
    assert!(out.contains("hello"));
}

#[test]
fn non_fenced_content_untouched() {
    let md = "plain text\n\nmore text\n";
    let out = plugin().on_page_markdown(md.into(), &dummy_page(), &Config::default()).unwrap();
    assert_eq!(out, md);
}
