use tempfile::TempDir;

use farol_core::PluginHost;
use farol_core::config::Config;
use farol_core::plugins::core::redirects::RedirectsPlugin;

#[test]
fn writes_meta_refresh_file() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let site = root.join("site");
    std::fs::create_dir_all(&site).unwrap();

    std::fs::write(
        root.join("redirects.toml"),
        r#"[redirects]
"/old/guide/" = "/guide/new/"
"#,
    )
    .unwrap();

    RedirectsPlugin.on_post_build(&site, &Config::default()).unwrap();

    let redirect_file = site.join("old").join("guide").join("index.html");
    assert!(redirect_file.exists());
    let content = std::fs::read_to_string(&redirect_file).unwrap();
    assert!(content.contains(r#"content="0; url=/guide/new/""#));
    assert!(content.contains(r#"canonical" href="/guide/new/""#));
}

#[test]
fn external_targets_ok() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let site = root.join("site");
    std::fs::create_dir_all(&site).unwrap();

    std::fs::write(
        root.join("redirects.toml"),
        r#"[redirects]
"/legacy/" = "https://example.com/new"
"#,
    )
    .unwrap();

    RedirectsPlugin.on_post_build(&site, &Config::default()).unwrap();
    let content = std::fs::read_to_string(site.join("legacy").join("index.html")).unwrap();
    assert!(content.contains("https://example.com/new"));
}

#[test]
fn missing_file_is_no_op() {
    let tmp = TempDir::new().unwrap();
    let site = tmp.path().join("site");
    std::fs::create_dir_all(&site).unwrap();
    RedirectsPlugin.on_post_build(&site, &Config::default()).unwrap();
    assert_eq!(std::fs::read_dir(&site).unwrap().count(), 0);
}

#[test]
fn invalid_toml_errors() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let site = root.join("site");
    std::fs::create_dir_all(&site).unwrap();
    std::fs::write(root.join("redirects.toml"), "not valid toml = = =").unwrap();
    assert!(RedirectsPlugin.on_post_build(&site, &Config::default()).is_err());
}
