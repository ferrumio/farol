use std::{
    fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::error::{FarolError, Result};

const HASHED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "css", "js"];

/// Copy an asset from `src` to the site tree, optionally applying a content
/// hash to the filename for cache-busting. Returns the URL path (with leading
/// slash) the asset is reachable at.
pub fn copy_asset(src: &Path, relative: &Path, site_dir: &Path, hashed: bool) -> Result<String> {
    let bytes = fs::read(src).map_err(|e| FarolError::io(src, e))?;
    let target_relative = if hashed && should_hash(relative) {
        hashed_name(relative, &bytes)
    } else {
        relative.to_path_buf()
    };
    let dest = site_dir.join(&target_relative);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| FarolError::io(parent, e))?;
    }
    fs::write(&dest, &bytes).map_err(|e| FarolError::io(&dest, e))?;

    Ok(format!("/{}", target_relative.to_string_lossy().replace('\\', "/")))
}

fn should_hash(relative: &Path) -> bool {
    relative
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| HASHED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn hashed_name(relative: &Path, bytes: &[u8]) -> PathBuf {
    let digest = Sha256::digest(bytes);
    let hash = hex(&digest[..4]);
    let stem = relative.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
    let ext = relative.extension().and_then(|s| s.to_str()).unwrap_or("");
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let new_name =
        if ext.is_empty() { format!("{stem}.{hash}") } else { format!("{stem}.{hash}.{ext}") };
    parent.join(new_name)
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}
