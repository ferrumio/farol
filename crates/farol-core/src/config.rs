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
