use std::path::PathBuf;

use farol_core::PluginHost;
use farol_core::config::Config;
use farol_core::frontmatter::Frontmatter;
use farol_core::page::Page;
use farol_core::plugins::core::edit_on_git::EditOnGitPlugin;

fn page(rel: &str) -> Page {
    Page {
        relative: PathBuf::from(rel),
        source_abs: PathBuf::from(format!("/tmp/{rel}")),
        url: "/x/".into(),
        output: PathBuf::from("x/index.html"),
        title: "x".into(),
        frontmatter: Frontmatter::new(),
        body_html: String::new(),
        toc: Vec::new(),
        layout: "default".to_string(),
    }
}

#[test]
fn no_repo_no_link() {
    let out = EditOnGitPlugin
        .on_page_html("<p>body</p>".into(), &page("a.md"), &Config::default())
        .unwrap();
    assert!(!out.contains("edit-on-git"));
}

#[test]
fn builds_github_url() {
    let cfg = Config {
        repo_url: Some("https://github.com/ferrumio/farol".into()),
        edit_uri: Some("edit/main/docs/".into()),
        ..Config::default()
    };
    let out = EditOnGitPlugin
        .on_page_html("<p>body</p>".into(), &page("guide/install.md"), &cfg)
        .unwrap();
    assert!(
        out.contains(r#"href="https://github.com/ferrumio/farol/edit/main/docs/guide/install.md""#)
    );
    assert!(out.contains(r#"target="_blank""#));
}

#[test]
fn default_edit_uri_applied() {
    let cfg = Config { repo_url: Some("https://github.com/foo/bar".into()), ..Config::default() };
    let out = EditOnGitPlugin.on_page_html("<p>body</p>".into(), &page("index.md"), &cfg).unwrap();
    assert!(out.contains(r#"href="https://github.com/foo/bar/edit/main/docs/index.md""#));
}
