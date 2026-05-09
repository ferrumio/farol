use serde::{Deserialize, Serialize};

use crate::error::{FarolError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeManifest {
    pub theme: ThemeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMetadata {
    pub name: String,
    pub version: String,
    pub min_farol_version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    pub layouts: LayoutsConfig,
    #[serde(default)]
    pub assets: AssetsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutsConfig {
    pub supported: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetsConfig {
    #[serde(default = "default_true")]
    pub shared_js: bool,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(default)]
    pub js: Vec<String>,
}

impl Default for AssetsConfig {
    fn default() -> Self {
        Self { shared_js: true, css: Vec::new(), js: Vec::new() }
    }
}

fn default_true() -> bool {
    true
}

pub fn parse(content: &str) -> Result<ThemeManifest> {
    toml::from_str(content)
        .map_err(|e| FarolError::ConfigInvalid { message: format!("invalid theme.toml: {e}") })
}

pub fn validate_version(manifest: &ThemeManifest, farol_version: &str) -> Result<()> {
    let required = &manifest.theme.min_farol_version;
    if !version_satisfies(farol_version, required) {
        return Err(FarolError::ConfigInvalid {
            message: format!(
                "theme `{}` requires farol >= {} but current version is {}",
                manifest.theme.name, required, farol_version
            ),
        });
    }
    Ok(())
}

fn version_satisfies(current: &str, minimum: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<&str> = v.split('.').collect();
        let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };
    parse(current) >= parse(minimum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_manifest() {
        let toml = r#"
[theme]
name = "test"
version = "0.1.0"
min_farol_version = "0.0.3"

[theme.layouts]
supported = ["default", "landing"]

[theme.assets]
shared_js = true
css = ["base.css"]
"#;
        let m = parse(toml).unwrap();
        assert_eq!(m.theme.name, "test");
        assert_eq!(m.theme.layouts.supported, vec!["default", "landing"]);
        assert!(m.theme.assets.shared_js);
    }

    #[test]
    fn version_check() {
        assert!(version_satisfies("0.0.3", "0.0.3"));
        assert!(version_satisfies("0.1.0", "0.0.3"));
        assert!(!version_satisfies("0.0.2", "0.0.3"));
    }
}
