use std::{fs, path::Path};

use include_dir::{Dir, include_dir};
use minijinja::{Environment, path_loader};

use crate::error::{FarolError, Result};

const DEFAULT_THEME: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/theme/default");

/// Build a MiniJinja environment backed by the default theme templates, with
/// optional filesystem overrides from `overrides_dir`.
pub fn build_env(overrides_dir: Option<&Path>) -> Result<Environment<'static>> {
    let mut env = Environment::new();

    // Register every embedded template.
    let templates_dir =
        DEFAULT_THEME.get_dir("templates").ok_or_else(|| FarolError::ConfigInvalid {
            message: "default theme is missing its templates directory".into(),
        })?;
    let base = templates_dir.path().to_path_buf();
    register_dir(&mut env, templates_dir, &base)?;

    // Overrides wrap the embedded set at the same logical name.
    if let Some(dir) = overrides_dir {
        if dir.exists() {
            let dir = dir.to_path_buf();
            env.set_loader(path_loader(dir));
        }
    }

    Ok(env)
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

/// Copy the default theme's assets (CSS, JS, fonts) into `site_dir/assets/`.
pub fn copy_assets(site_dir: &Path) -> Result<()> {
    let assets_dir = DEFAULT_THEME.get_dir("assets").ok_or_else(|| FarolError::ConfigInvalid {
        message: "default theme is missing its assets directory".into(),
    })?;
    let target_root = site_dir.join("assets");
    fs::create_dir_all(&target_root).map_err(|e| FarolError::io(&target_root, e))?;
    write_dir_contents(assets_dir, &target_root)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_contains_default_template() {
        let env = build_env(None).unwrap();
        assert!(env.get_template("default.html").is_ok());
    }

    #[test]
    fn copy_assets_writes_css() {
        let tmp = tempfile::TempDir::new().unwrap();
        copy_assets(tmp.path()).unwrap();
        assert!(tmp.path().join("assets/base.css").exists());
    }
}
