use std::{fs, path::Path};

use crate::error::{FarolError, Result};

use super::manifest::{self, ThemeManifest};

pub fn load_manifest(theme_dir: &Path) -> Result<ThemeManifest> {
    let manifest_path = theme_dir.join("theme.toml");
    if !manifest_path.exists() {
        return Err(FarolError::ConfigInvalid {
            message: format!("external theme at `{}` is missing theme.toml", theme_dir.display()),
        });
    }
    let content =
        fs::read_to_string(&manifest_path).map_err(|e| FarolError::io(&manifest_path, e))?;
    manifest::parse(&content)
}

pub fn validate_structure(theme_dir: &Path) -> Result<()> {
    let templates = theme_dir.join("templates");
    if !templates.exists() || !templates.is_dir() {
        return Err(FarolError::ConfigInvalid {
            message: format!(
                "external theme at `{}` is missing templates/ directory",
                theme_dir.display()
            ),
        });
    }
    Ok(())
}
