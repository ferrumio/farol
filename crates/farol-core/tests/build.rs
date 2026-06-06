use std::{fs, path::Path};

use tempfile::TempDir;

use farol_core::plugins::{ChainedHost, core as builtins};
use farol_core::{
    BuildOptions, Config, FarolError, NoOpHost, Page, PluginHost, Result, build, build_with,
};

fn write(dir: &Path, rel: &str, content: &str) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, content).unwrap();
}

#[test]
fn builds_minimal_site() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let docs = root.join("docs");
    write(&docs, "index.md", "# Home\n\n[guide](./guide/install.md).\n");
    write(&docs, "guide/install.md", "---\ntitle: Install\n---\n# Install\n\nstuff.\n");

    let cfg = Config { site_url: Some("https://example.com".into()), ..Config::default() };
    let mut hosts: Vec<Box<dyn PluginHost>> = vec![Box::new(NoOpHost)];
    hosts.extend(builtins::all());
    let host = ChainedHost::from_boxes(hosts);
    let report =
        build_with(&cfg, root, &BuildOptions { no_cache: true, ..Default::default() }, &host)
            .unwrap();

    assert_eq!(report.pages, 2);
    assert!(report.broken_links.is_empty());
    assert!(root.join("site/index.html").exists());
    assert!(root.join("site/guide/install/index.html").exists());
    assert!(root.join("site/sitemap.xml").exists());
    assert!(root.join("site/robots.txt").exists());
    assert!(root.join("site/assets/base.css").exists());

    let home = fs::read_to_string(root.join("site/index.html")).unwrap();
    assert!(home.contains("Home"));
    assert!(home.contains(r#"href="/guide/install/""#));
}

#[test]
fn reports_broken_link() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let docs = root.join("docs");
    write(&docs, "index.md", "# Home\n\n[missing](./nope.md)\n");

    let cfg = Config::default();
    let report = build(&cfg, root).unwrap();
    assert_eq!(report.broken_links.len(), 1);
}

#[test]
fn warm_rebuild_hits_cache() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let docs = root.join("docs");
    write(&docs, "index.md", "# Home\n");
    write(&docs, "a.md", "# A\n");

    let cfg = Config::default();
    let opts = BuildOptions { timings: true, ..BuildOptions::default() };

    let r1 = build_with(&cfg, root, &opts, &NoOpHost).unwrap();
    assert_eq!(r1.graph.as_ref().unwrap().cache_misses, 2);
    assert_eq!(r1.graph.as_ref().unwrap().cache_hits, 0);

    let r2 = build_with(&cfg, root, &opts, &NoOpHost).unwrap();
    assert_eq!(r2.graph.as_ref().unwrap().cache_hits, 2);
    assert_eq!(r2.graph.as_ref().unwrap().cache_misses, 0);
}

#[test]
fn plugin_can_rewrite_markdown() {
    struct WaveHost;
    impl PluginHost for WaveHost {
        fn on_page_markdown(
            &self,
            markdown: String,
            _page: &Page,
            _config: &Config,
        ) -> Result<String> {
            Ok(markdown.replace(":wave:", "\u{1f44b}"))
        }
    }

    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let docs = root.join("docs");
    write(&docs, "index.md", "# Hi :wave:\n");

    let cfg = Config::default();
    let opts = BuildOptions { no_cache: true, ..BuildOptions::default() };
    build_with(&cfg, root, &opts, &WaveHost).unwrap();

    let html = fs::read_to_string(root.join("site/index.html")).unwrap();
    assert!(html.contains("\u{1f44b}"), "plugin replacement missing from output");
    assert!(!html.contains(":wave:"), "raw token leaked");
}

#[test]
fn plugin_can_rewrite_html() {
    struct AttrHost;
    impl PluginHost for AttrHost {
        fn on_page_html(&self, html: String, _page: &Page, _config: &Config) -> Result<String> {
            Ok(html.replace("<p>", "<p data-plugin=\"attr\">"))
        }
    }

    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let docs = root.join("docs");
    write(&docs, "index.md", "# Hi\n\nparagraph.\n");

    let cfg = Config::default();
    let opts = BuildOptions { no_cache: true, ..BuildOptions::default() };
    build_with(&cfg, root, &opts, &AttrHost).unwrap();

    let html = fs::read_to_string(root.join("site/index.html")).unwrap();
    assert!(html.contains("data-plugin=\"attr\""));
}

#[test]
fn plugin_error_propagates() {
    struct Fails;
    impl PluginHost for Fails {
        fn on_page_markdown(
            &self,
            _markdown: String,
            _page: &Page,
            _config: &Config,
        ) -> Result<String> {
            Err(FarolError::ConfigInvalid { message: "plugin said no".into() })
        }
    }

    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let docs = root.join("docs");
    write(&docs, "index.md", "# hi\n");

    let cfg = Config::default();
    let opts = BuildOptions { no_cache: true, ..BuildOptions::default() };
    let err = build_with(&cfg, root, &opts, &Fails).unwrap_err();
    match err {
        FarolError::ConfigInvalid { message } => assert!(message.contains("plugin said no")),
        other => panic!("wrong error: {other:?}"),
    }
}

#[test]
fn edited_page_invalidates_cache() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let docs = root.join("docs");
    write(&docs, "index.md", "# Home\n");
    write(&docs, "a.md", "# A\n");

    let cfg = Config::default();
    let opts = BuildOptions { timings: true, ..BuildOptions::default() };

    build_with(&cfg, root, &opts, &NoOpHost).unwrap();

    write(&docs, "a.md", "# A\n\nedited.\n");
    let r = build_with(&cfg, root, &opts, &NoOpHost).unwrap();
    let g = r.graph.unwrap();
    assert_eq!(g.cache_misses, 1);
    assert_eq!(g.cache_hits, 1);
}

#[test]
fn title_change_invalidates_dependents() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let docs = root.join("docs");
    write(&docs, "index.md", "# Home\n");
    write(&docs, "a.md", "# A\n");

    let cfg = Config::default();
    let opts = BuildOptions { timings: true, ..BuildOptions::default() };

    build_with(&cfg, root, &opts, &NoOpHost).unwrap();

    write(&docs, "a.md", "# A (renamed)\n");
    let r = build_with(&cfg, root, &opts, &NoOpHost).unwrap();
    let g = r.graph.unwrap();
    assert_eq!(g.cache_misses, 2);
    assert_eq!(g.cache_hits, 0);
}

#[test]
fn theme_change_invalidates_all() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let docs = root.join("docs");
    write(&docs, "a.md", "# A\n");
    write(&docs, "b.md", "# B\n");

    let opts = BuildOptions { timings: true, ..BuildOptions::default() };

    let cfg1 = Config { site_name: "v1".into(), ..Config::default() };
    build_with(&cfg1, root, &opts, &NoOpHost).unwrap();

    let cfg2 = Config { site_name: "v2".into(), ..Config::default() };
    let r = build_with(&cfg2, root, &opts, &NoOpHost).unwrap();
    assert_eq!(r.graph.unwrap().cache_misses, 2);
}
