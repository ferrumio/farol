use std::path::PathBuf;

use tempfile::TempDir;

use farol_core::PluginHost;
use farol_core::config::Config;
use farol_core::frontmatter::Frontmatter;
use farol_core::page::Page;
use farol_core::plugins::core::search::SearchPlugin;

fn page(url: &str, title: &str) -> Page {
    Page {
        relative: PathBuf::from(format!("{title}.md")),
        source_abs: PathBuf::from(format!("/tmp/{title}.md")),
        url: url.into(),
        output: PathBuf::from(format!("{url}index.html")),
        title: title.into(),
        frontmatter: Frontmatter::new(),
        body_html: String::new(),
        toc: Vec::new(),
        layout: "default".to_string(),
    }
}

#[test]
fn writes_search_assets_on_post_build() {
    let tmp = TempDir::new().unwrap();
    let site = tmp.path();

    let plugin = SearchPlugin::default();
    let cfg = Config::default();
    for (url, title, html) in [
        ("/a/", "Alpha", "<p>Alpha is the first</p>"),
        ("/b/", "Beta", "<p>Beta is the second letter</p>"),
    ] {
        plugin.on_page_html(html.into(), &page(url, title), &cfg).unwrap();
    }
    plugin.on_post_build(site, &cfg).unwrap();

    let docs_path = site.join("assets/search/docs.json");
    let index_path = site.join("assets/search/index.json");
    assert!(docs_path.exists());
    assert!(index_path.exists());

    let docs: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&docs_path).unwrap()).unwrap();
    assert_eq!(docs.as_array().unwrap().len(), 2);

    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&index_path).unwrap()).unwrap();
    assert_eq!(index["version"], 1);
    let map = index["index"].as_object().unwrap();
    assert!(map.contains_key("alpha"), "missing alpha in {map:?}");
}
