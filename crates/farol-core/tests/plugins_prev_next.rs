use std::path::PathBuf;

use tempfile::TempDir;

use farol_core::PluginHost;
use farol_core::config::Config;
use farol_core::frontmatter::Frontmatter;
use farol_core::page::Page;
use farol_core::plugins::core::prev_next::PrevNextPlugin;

fn p(url: &str, title: &str, out: &str) -> Page {
    Page {
        relative: PathBuf::from(format!("{title}.md")),
        source_abs: PathBuf::from(format!("/tmp/{title}.md")),
        url: url.into(),
        output: PathBuf::from(out),
        title: title.into(),
        frontmatter: Frontmatter::new(),
        body_html: String::new(),
        toc: Vec::new(),
        layout: "default".to_string(),
    }
}

#[test]
fn inserts_nav_on_middle_page() {
    let tmp = TempDir::new().unwrap();
    let site = tmp.path();
    let plugin = PrevNextPlugin::default();
    let cfg = Config::default();

    for name in ["a", "b", "c"] {
        std::fs::create_dir_all(site.join(name)).unwrap();
        std::fs::write(site.join(name).join("index.html"), format!("<main>{name}</main>")).unwrap();
        let pg = p(&format!("/{name}/"), name, &format!("{name}/index.html"));
        plugin.on_page_html(pg.body_html.clone(), &pg, &cfg).unwrap();
    }

    plugin.on_post_build(site, &cfg).unwrap();

    let b = std::fs::read_to_string(site.join("b").join("index.html")).unwrap();
    assert!(b.contains(r#"class="prev-next""#));
    assert!(b.contains(r#"href="/a/""#));
    assert!(b.contains(r#"href="/c/""#));
}

#[test]
fn first_page_only_has_next() {
    let tmp = TempDir::new().unwrap();
    let site = tmp.path();
    let plugin = PrevNextPlugin::default();
    let cfg = Config::default();

    for name in ["a", "b"] {
        std::fs::create_dir_all(site.join(name)).unwrap();
        std::fs::write(site.join(name).join("index.html"), format!("<main>{name}</main>")).unwrap();
        let pg = p(&format!("/{name}/"), name, &format!("{name}/index.html"));
        plugin.on_page_html(pg.body_html.clone(), &pg, &cfg).unwrap();
    }
    plugin.on_post_build(site, &cfg).unwrap();

    let a = std::fs::read_to_string(site.join("a").join("index.html")).unwrap();
    assert!(a.contains(r#"class="prev disabled""#));
    assert!(a.contains(r#"href="/b/""#));
}

#[test]
fn single_page_skips_nav() {
    let tmp = TempDir::new().unwrap();
    let site = tmp.path();
    let plugin = PrevNextPlugin::default();
    let cfg = Config::default();
    std::fs::write(site.join("index.html"), "<main>only</main>").unwrap();
    let pg = p("/", "only", "index.html");
    plugin.on_page_html(pg.body_html.clone(), &pg, &cfg).unwrap();
    plugin.on_post_build(site, &cfg).unwrap();

    let out = std::fs::read_to_string(site.join("index.html")).unwrap();
    assert!(!out.contains("prev-next"));
}
