use std::path::{Path, PathBuf};

use miette::{NamedSource, SourceSpan};
use serde::{Deserialize, Serialize};

use crate::error::{ConfigParseError, FarolError, Result};

pub const DEFAULT_CONFIG_FILENAME: &str = "farol.toml";

/// Top-level site configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_site_name")]
    pub site_name: String,

    #[serde(default)]
    pub site_url: Option<String>,

    #[serde(default)]
    pub site_description: Option<String>,

    #[serde(default)]
    pub repo_url: Option<String>,

    #[serde(default)]
    pub edit_uri: Option<String>,

    #[serde(default = "default_docs_dir")]
    pub docs_dir: PathBuf,

    #[serde(default = "default_site_dir")]
    pub site_dir: PathBuf,

    #[serde(default)]
    pub theme: ThemeConfig,

    #[serde(default)]
    pub plugins: PluginsConfig,

    #[serde(default)]
    pub extras: toml::Table,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
    #[serde(default = "default_theme_name")]
    pub name: String,
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub palette: Option<String>,
    #[serde(default)]
    pub primary: Option<String>,
    #[serde(default)]
    pub accent: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginsConfig {
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub disabled: Vec<String>,
}

impl PluginsConfig {
    /// Decide whether a plugin identified by `name` should run.
    ///
    /// Rules:
    /// - If `enabled` is non-empty, it is a whitelist: only plugins listed
    ///   there run. `disabled` is ignored in this mode to avoid conflicting
    ///   intent.
    /// - If `enabled` is empty, every plugin runs except those in `disabled`.
    pub fn is_plugin_enabled(&self, name: &str) -> bool {
        if !self.enabled.is_empty() {
            return self.enabled.iter().any(|n| n == name);
        }
        !self.disabled.iter().any(|n| n == name)
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self { name: default_theme_name(), path: None, palette: None, primary: None, accent: None }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            site_name: default_site_name(),
            site_url: None,
            site_description: None,
            repo_url: None,
            edit_uri: None,
            docs_dir: default_docs_dir(),
            site_dir: default_site_dir(),
            theme: ThemeConfig::default(),
            plugins: PluginsConfig::default(),
            extras: toml::Table::new(),
        }
    }
}

fn default_site_name() -> String {
    "My Docs".to_string()
}
fn default_docs_dir() -> PathBuf {
    PathBuf::from("docs")
}
fn default_site_dir() -> PathBuf {
    PathBuf::from("site")
}
fn default_theme_name() -> String {
    "default".to_string()
}

impl Config {
    /// Load and validate a config from a TOML file on disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|e| FarolError::io(path, e))?;
        Self::from_str(&text, path)
    }

    /// Parse config from a string, attributing errors to `source_path`.
    pub fn from_str(text: &str, source_path: impl AsRef<Path>) -> Result<Self> {
        let source_path = source_path.as_ref();
        let source_name = source_path.display().to_string();

        let config: Self = toml::from_str(text).map_err(|e| {
            let span = span_from_toml_error(&e, text);
            FarolError::ConfigParse(Box::new(ConfigParseError {
                src: NamedSource::new(source_name.clone(), text.to_string()),
                span,
                help: Some("check the syntax and make sure all keys are recognized".into()),
                message: e.message().to_string(),
            }))
        })?;

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.site_name.trim().is_empty() {
            return Err(FarolError::ConfigInvalid { message: "site_name cannot be empty".into() });
        }
        Ok(())
    }
}

/// Convert a toml::de::Error span into a miette SourceSpan, falling back to
/// the start of the file if no span is available.
fn span_from_toml_error(err: &toml::de::Error, text: &str) -> SourceSpan {
    if let Some(range) = err.span() {
        (range.start, range.end.saturating_sub(range.start)).into()
    } else {
        (0, text.len().min(1)).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let cfg =
            PluginsConfig { enabled: vec!["a".into()], disabled: vec!["a".into(), "b".into()] };
        assert!(cfg.is_plugin_enabled("a"));
        assert!(!cfg.is_plugin_enabled("b"));
    }
}
