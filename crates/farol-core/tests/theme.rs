use std::path::Path;

use farol_core::config::ThemeConfig;
use farol_core::theme::{build_env, copy_assets, resolve_from_config};

#[test]
fn resolves_default_theme() {
    let config = ThemeConfig::default();
    let theme = resolve_from_config(&config, Path::new("/tmp")).unwrap();
    assert_eq!(theme.name(), "default");
}

#[test]
fn resolves_api_theme() {
    let config = ThemeConfig { name: "api".into(), ..ThemeConfig::default() };
    let theme = resolve_from_config(&config, Path::new("/tmp")).unwrap();
    assert_eq!(theme.name(), "api");
    assert!(theme.manifest.theme.layouts.supported.contains(&"default".to_string()));
}

#[test]
fn resolves_book_theme() {
    let config = ThemeConfig { name: "book".into(), ..ThemeConfig::default() };
    let theme = resolve_from_config(&config, Path::new("/tmp")).unwrap();
    assert_eq!(theme.name(), "book");
}

#[test]
fn unknown_theme_errors() {
    let config = ThemeConfig { name: "nonexistent".into(), ..ThemeConfig::default() };
    let result = resolve_from_config(&config, Path::new("/tmp"));
    assert!(result.is_err());
}

#[test]
fn env_contains_default_template() {
    let config = ThemeConfig::default();
    let theme = resolve_from_config(&config, Path::new("/tmp")).unwrap();
    let env = build_env(&theme, None).unwrap();
    assert!(env.get_template("default.html").is_ok());
}

#[test]
fn copy_assets_writes_css_and_js() {
    let config = ThemeConfig::default();
    let theme = resolve_from_config(&config, Path::new("/tmp")).unwrap();
    let tmp = tempfile::TempDir::new().unwrap();
    copy_assets(&theme, tmp.path()).unwrap();
    assert!(tmp.path().join("assets/base.css").exists());
    assert!(tmp.path().join("assets/farol.js").exists());
    assert!(tmp.path().join("assets/search.js").exists());
}

#[test]
fn api_theme_has_own_css() {
    let config = ThemeConfig { name: "api".into(), ..ThemeConfig::default() };
    let theme = resolve_from_config(&config, Path::new("/tmp")).unwrap();
    let tmp = tempfile::TempDir::new().unwrap();
    copy_assets(&theme, tmp.path()).unwrap();
    assert!(tmp.path().join("assets/api.css").exists());
    assert!(tmp.path().join("assets/farol.js").exists());
}

#[test]
fn book_theme_has_own_css() {
    let config = ThemeConfig { name: "book".into(), ..ThemeConfig::default() };
    let theme = resolve_from_config(&config, Path::new("/tmp")).unwrap();
    let tmp = tempfile::TempDir::new().unwrap();
    copy_assets(&theme, tmp.path()).unwrap();
    assert!(tmp.path().join("assets/book.css").exists());
    assert!(tmp.path().join("assets/farol.js").exists());
}
