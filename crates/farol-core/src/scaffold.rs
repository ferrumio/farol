use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    config::DEFAULT_CONFIG_FILENAME,
    error::{FarolError, Result},
};

const DEFAULT_CONFIG: &str = include_str!("scaffold_assets/farol.toml");
const INDEX_MD: &str = include_str!("scaffold_assets/index.md");
const GETTING_STARTED_MD: &str = include_str!("scaffold_assets/getting-started.md");

/// Create a new farol project at `target`. Fails if the target already exists
/// and is non-empty.
pub fn scaffold(target: impl AsRef<Path>) -> Result<PathBuf> {
    let target = target.as_ref();

    if target.exists()
        && fs::read_dir(target).map_err(|e| FarolError::io(target, e))?.next().is_some()
    {
        return Err(FarolError::ScaffoldExists { path: target.to_path_buf() });
    }

    create_dir(target)?;
    let docs = target.join("docs");
    create_dir(&docs)?;

    write(target.join(DEFAULT_CONFIG_FILENAME), DEFAULT_CONFIG)?;
    write(docs.join("index.md"), INDEX_MD)?;
    write(docs.join("getting-started.md"), GETTING_STARTED_MD)?;

    Ok(target.to_path_buf())
}

fn create_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|e| FarolError::io(path, e))
}

fn write(path: PathBuf, content: &str) -> Result<()> {
    fs::write(&path, content).map_err(|e| FarolError::io(&path, e))
}
