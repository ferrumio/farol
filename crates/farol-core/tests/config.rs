use std::path::PathBuf;

use farol_core::FarolError;
use farol_core::config::{Config, PluginsConfig};

#[test]
fn parses_minimal() {
    let text = r#"site_name = "hello""#;
    let cfg = Config::from_str(text, "farol.toml").unwrap();
    assert_eq!(cfg.site_name, "hello");
    assert_eq!(cfg.theme.name, "default");
}

#[test]
fn defaults_when_empty() {
    let cfg = Config::from_str("", "farol.toml").unwrap();
    assert_eq!(cfg.site_name, "My Docs");
    assert_eq!(cfg.docs_dir, PathBuf::from("docs"));
}

#[test]
fn rejects_empty_site_name() {
    let text = r#"site_name = """#;
    let err = Config::from_str(text, "farol.toml").unwrap_err();
    matches!(err, FarolError::ConfigInvalid { .. });
}

#[test]
fn rejects_unknown_top_level_key() {
    let text = r#"
site_name = "ok"
unknown_key = "nope"
"#;
    assert!(Config::from_str(text, "farol.toml").is_err());
}

#[test]
fn parse_error_points_at_location() {
    let text = "site_name = \nnot_a_value";
    let err = Config::from_str(text, "farol.toml").unwrap_err();
    assert!(matches!(err, FarolError::ConfigParse { .. }));
}

#[test]
fn plugins_lists() {
    let text = r#"
[plugins]
enabled = ["search", "sitemap"]
disabled = ["rss"]
"#;
    let cfg = Config::from_str(text, "farol.toml").unwrap();
    assert_eq!(cfg.plugins.enabled, vec!["search", "sitemap"]);
    assert_eq!(cfg.plugins.disabled, vec!["rss"]);
}

#[test]
fn plugin_filter_default_enables_all() {
    let cfg = PluginsConfig::default();
    assert!(cfg.is_plugin_enabled("anything"));
}

#[test]
fn plugin_filter_whitelist_excludes_others() {
    let cfg = PluginsConfig { enabled: vec!["a".into(), "b".into()], ..Default::default() };
    assert!(cfg.is_plugin_enabled("a"));
    assert!(cfg.is_plugin_enabled("b"));
    assert!(!cfg.is_plugin_enabled("c"));
}

#[test]
fn plugin_filter_blacklist_excludes_only_listed() {
    let cfg = PluginsConfig { disabled: vec!["x".into()], ..Default::default() };
    assert!(cfg.is_plugin_enabled("a"));
    assert!(!cfg.is_plugin_enabled("x"));
}

#[test]
fn plugin_filter_whitelist_wins_over_blacklist() {
    let cfg = PluginsConfig { enabled: vec!["a".into()], disabled: vec!["a".into(), "b".into()] };
    assert!(cfg.is_plugin_enabled("a"));
    assert!(!cfg.is_plugin_enabled("b"));
}
