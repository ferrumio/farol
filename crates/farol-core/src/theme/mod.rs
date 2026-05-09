mod embedded;
mod external;
pub mod manifest;

use std::{
    fs,
    path::{Path, PathBuf},
};

use include_dir::Dir;
use minijinja::{Environment, path_loader};

use crate::{
    config::ThemeConfig,
    error::{FarolError, Result},
    page::Page,
};

pub use manifest::ThemeManifest;

pub struct Theme {
    pub manifest: ThemeManifest,
    source: ThemeSource,
}

enum ThemeSource {
    Embedded(&'static Dir<'static>),
    External(PathBuf),
}

impl Theme {
    pub fn name(&self) -> &str {
        &self.manifest.theme.name
    }

    pub fn validate_layouts(&self, pages: &[Page]) -> Result<()> {
        let supported = &self.manifest.theme.layouts.supported;
        for page in pages {
            if !supported.iter().any(|l| l == &page.layout) {
                return Err(FarolError::ConfigInvalid {
                    message: format!(
                        "page `{}` uses layout `{}` which is not supported by theme `{}` (supported: {:?})",
                        page.relative.display(),
                        page.layout,
                        self.manifest.theme.name,
                        supported,
                    ),
                });
            }
        }
        Ok(())
    }
}

/// Resolve a theme from configuration.
pub fn resolve_from_config(theme_config: &ThemeConfig, project_root: &Path) -> Result<Theme> {
    // 1. Explicit path takes priority.
    if let Some(path) = &theme_config.path {
        let abs = if path.is_absolute() { path.clone() } else { project_root.join(path) };
        return load_external(&abs);
    }

    // 2. Check built-in names.
    if let Some(dir) = embedded::get_embedded(&theme_config.name) {
        return load_embedded(dir);
    }

    // 3. Check ~/.farol/themes/<name>/.
    if let Ok(home) = std::env::var("HOME") {
        let candidate = PathBuf::from(home).join(".farol").join("themes").join(&theme_config.name);
        if candidate.exists() {
            return load_external(&candidate);
        }
    }

    Err(FarolError::ThemeNotFound { name: theme_config.name.clone() })
}

fn load_embedded(dir: &'static Dir<'static>) -> Result<Theme> {
    let manifest_file = dir.get_file("theme.toml").ok_or_else(|| FarolError::ConfigInvalid {
        message: "embedded theme is missing theme.toml".into(),
    })?;
    let content = std::str::from_utf8(manifest_file.contents()).map_err(|e| {
        FarolError::ConfigInvalid { message: format!("theme.toml is not valid UTF-8: {e}") }
    })?;
    let manifest = manifest::parse(content)?;
    Ok(Theme { manifest, source: ThemeSource::Embedded(dir) })
}

fn load_external(path: &Path) -> Result<Theme> {
    if !path.exists() {
        return Err(FarolError::ThemeNotFound { name: path.display().to_string() });
    }
    external::validate_structure(path)?;
    let manifest = external::load_manifest(path)?;
    manifest::validate_version(&manifest, env!("CARGO_PKG_VERSION"))?;
    Ok(Theme { manifest, source: ThemeSource::External(path.to_path_buf()) })
}

/// Build a MiniJinja environment for the resolved theme, with optional
/// filesystem overrides from `overrides_dir`.
pub fn build_env(theme: &Theme, overrides_dir: Option<&Path>) -> Result<Environment<'static>> {
    let mut env = Environment::new();

    match &theme.source {
        ThemeSource::Embedded(dir) => {
            let templates_dir =
                dir.get_dir("templates").ok_or_else(|| FarolError::ConfigInvalid {
                    message: format!(
                        "theme `{}` is missing its templates directory",
                        theme.manifest.theme.name
                    ),
                })?;
            let base = templates_dir.path().to_path_buf();
            register_dir(&mut env, templates_dir, &base)?;
        }
        ThemeSource::External(path) => {
            let templates_path = path.join("templates");
            env.set_loader(path_loader(templates_path));
        }
    }

    // Overrides wrap the embedded/external set at the same logical name.
    if let Some(dir) = overrides_dir {
        if dir.exists() {
            let dir = dir.to_path_buf();
            env.set_loader(path_loader(dir));
        }
    }

    Ok(env)
}

/// Copy theme assets (CSS, JS, fonts) into `site_dir/assets/`.
pub fn copy_assets(theme: &Theme, site_dir: &Path) -> Result<()> {
    let target_root = site_dir.join("assets");
    fs::create_dir_all(&target_root).map_err(|e| FarolError::io(&target_root, e))?;

    // Copy shared JS if the theme requests it.
    if theme.manifest.theme.assets.shared_js {
        let shared = embedded::shared_assets();
        write_dir_contents(shared, &target_root)?;
    }

    // Copy theme-specific assets.
    match &theme.source {
        ThemeSource::Embedded(dir) => {
            if let Some(assets_dir) = dir.get_dir("assets") {
                write_embedded_assets(assets_dir, &target_root)?;
            }
        }
        ThemeSource::External(path) => {
            let assets_path = path.join("assets");
            if assets_path.exists() {
                copy_dir_recursive(&assets_path, &target_root)?;
            }
        }
    }

    Ok(())
}

fn register_dir(env: &mut Environment<'static>, dir: &Dir<'static>, base: &Path) -> Result<()> {
    for file in dir.files() {
        let path = file.path();
        let name = path.strip_prefix(base).unwrap_or(path).to_string_lossy().replace('\\', "/");
        let content =
            std::str::from_utf8(file.contents()).map_err(|e| FarolError::ConfigInvalid {
                message: format!("embedded template {} is not utf-8: {e}", path.display()),
            })?;
        env.add_template_owned(name, content.to_string()).map_err(|e| {
            FarolError::ConfigInvalid {
                message: format!("failed to register template {}: {e}", path.display()),
            }
        })?;
    }
    for sub in dir.dirs() {
        register_dir(env, sub, base)?;
    }
    Ok(())
}

fn write_embedded_assets(dir: &Dir<'_>, target: &Path) -> Result<()> {
    for file in dir.files() {
        let rel = file.path().strip_prefix(dir.path()).unwrap_or(file.path());
        let dest = target.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| FarolError::io(parent, e))?;
        }
        fs::write(&dest, file.contents()).map_err(|e| FarolError::io(&dest, e))?;
    }
    for sub in dir.dirs() {
        write_embedded_assets(sub, target)?;
    }
    Ok(())
}

fn write_dir_contents(dir: &Dir<'_>, target: &Path) -> Result<()> {
    for file in dir.files() {
        let rel = file.path().strip_prefix(dir.path()).unwrap_or(file.path());
        let dest = target.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| FarolError::io(parent, e))?;
        }
        fs::write(&dest, file.contents()).map_err(|e| FarolError::io(&dest, e))?;
    }
    for sub in dir.dirs() {
        write_dir_contents(sub, target)?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    for entry in fs::read_dir(src).map_err(|e| FarolError::io(src, e))? {
        let entry = entry.map_err(|e| FarolError::io(src, e))?;
        let path = entry.path();
        let rel = path.strip_prefix(src).unwrap_or(&path);
        let target = dest.join(rel);
        if path.is_dir() {
            fs::create_dir_all(&target).map_err(|e| FarolError::io(&target, e))?;
            copy_dir_recursive(&path, &target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| FarolError::io(parent, e))?;
            }
            fs::copy(&path, &target).map_err(|e| FarolError::io(&path, e))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ThemeConfig;

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
}
