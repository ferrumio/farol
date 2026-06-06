use std::path::PathBuf;

use farol_core::markdown;

#[test]
fn extracts_title_from_h1() {
    let p = PathBuf::from("p.md");
    let out = markdown::parse("# Hello World\n\nbody", &p).unwrap();
    assert_eq!(out.title.as_deref(), Some("Hello World"));
    assert!(out.html.contains("<h1>Hello World</h1>"));
}

#[test]
fn collects_headings_with_slugs() {
    let src = "# One\n\n## Two\n\n## Two\n\n### Three";
    let out = markdown::parse(src, &PathBuf::from("p.md")).unwrap();
    let slugs: Vec<&str> = out.headings.iter().map(|(_, _, s)| s.as_str()).collect();
    assert_eq!(slugs, vec!["one", "two", "two-1", "three"]);
}

#[test]
fn no_title_when_no_h1() {
    let out = markdown::parse("## Only H2\n", &PathBuf::from("p.md")).unwrap();
    assert!(out.title.is_none());
}

#[test]
fn renders_gfm_tables() {
    let src = "| a | b |\n| - | - |\n| 1 | 2 |\n";
    let out = markdown::parse(src, &PathBuf::from("p.md")).unwrap();
    assert!(out.html.contains("<table>"));
    assert!(out.html.contains("<td>1</td>"));
}

#[test]
fn renders_strikethrough() {
    let out = markdown::parse("~~gone~~", &PathBuf::from("p.md")).unwrap();
    assert!(out.html.contains("<del>gone</del>"));
}

#[test]
fn renders_task_list() {
    let src = "- [x] done\n- [ ] todo\n";
    let out = markdown::parse(src, &PathBuf::from("p.md")).unwrap();
    assert!(out.html.contains("type=\"checkbox\""));
}
