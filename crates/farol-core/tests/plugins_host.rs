use std::path::PathBuf;

use farol_core::config::Config;
use farol_core::frontmatter::Frontmatter;
use farol_core::page::Page;
use farol_core::{NoOpHost, PluginHost};

fn sample_page() -> Page {
    Page {
        relative: PathBuf::from("index.md"),
        source_abs: PathBuf::from("/tmp/index.md"),
        url: "/".into(),
        output: PathBuf::from("index.html"),
        title: "hi".into(),
        frontmatter: Frontmatter::new(),
        body_html: String::new(),
        toc: Vec::new(),
        layout: "default".to_string(),
    }
}

#[test]
fn no_op_passes_through() {
    let host = NoOpHost;
    let cfg = Config::default();
    let cfg2 = host.on_config(cfg.clone()).unwrap();
    assert_eq!(cfg.site_name, cfg2.site_name);

    let md = host.on_page_markdown("# hi".into(), &sample_page(), &cfg).unwrap();
    assert_eq!(md, "# hi");
}
