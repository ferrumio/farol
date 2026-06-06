use std::path::PathBuf;

use farol_core::frontmatter::Frontmatter;
use farol_core::nav;
use farol_core::page::Page;
use farol_core::toc::TocEntry;

fn p(rel: &str, title: &str, url: &str, weight: Option<i64>) -> Page {
    let mut fm = Frontmatter::new();
    if let Some(w) = weight {
        fm.insert("weight".into(), toml::Value::Integer(w));
    }
    Page {
        relative: PathBuf::from(rel),
        source_abs: PathBuf::from(format!("/tmp/{rel}")),
        url: url.into(),
        output: PathBuf::from("ignored"),
        title: title.into(),
        frontmatter: fm,
        body_html: String::new(),
        toc: Vec::<TocEntry>::new(),
        layout: "default".into(),
    }
}

#[test]
fn flat_pages_are_listed() {
    let nav =
        nav::build(&[p("index.md", "Home", "/", None), p("about.md", "About", "/about/", None)]);
    let titles: Vec<_> = nav.iter().map(|n| n.title.as_str()).collect();
    assert!(titles.contains(&"Home"));
    assert!(titles.contains(&"About"));
}

#[test]
fn subdirs_become_sections() {
    let nav = nav::build(&[
        p("guide/install.md", "Install", "/guide/install/", Some(1)),
        p("guide/config.md", "Config", "/guide/config/", Some(2)),
    ]);
    let guide = nav.iter().find(|n| n.title == "Guide").expect("guide section");
    assert!(guide.is_section());
    assert_eq!(guide.children.len(), 2);
    assert_eq!(guide.children[0].title, "Install");
    assert_eq!(guide.children[1].title, "Config");
}

#[test]
fn weight_controls_order() {
    let nav = nav::build(&[
        p("a.md", "A", "/a/", Some(10)),
        p("b.md", "B", "/b/", Some(1)),
        p("c.md", "C", "/c/", Some(5)),
    ]);
    let titles: Vec<_> = nav.iter().map(|n| n.title.as_str()).collect();
    assert_eq!(titles, vec!["B", "C", "A"]);
}

#[test]
fn section_index_contributes_url() {
    let nav = nav::build(&[
        p("guide/index.md", "Guide overview", "/guide/", Some(1)),
        p("guide/install.md", "Install", "/guide/install/", Some(2)),
    ]);
    let guide = nav.iter().find(|n| n.title == "Guide overview").unwrap();
    assert_eq!(guide.url.as_deref(), Some("/guide/"));
    assert_eq!(guide.children.len(), 1);
}
