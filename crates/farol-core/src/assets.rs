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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn copies_asset_with_hash() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.png");
        fs::write(&src, b"data").unwrap();
        let site = tmp.path().join("site");
        let url = copy_asset(&src, Path::new("img/logo.png"), &site, true).unwrap();

        assert!(url.starts_with("/img/logo."));
        assert!(url.ends_with(".png"));
        let final_path = site.join(url.trim_start_matches('/'));
        assert!(final_path.exists());
    }

    #[test]
    fn non_hashed_extension_keeps_name() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("a.pdf");
        fs::write(&src, b"x").unwrap();
        let site = tmp.path().join("site");
        let url = copy_asset(&src, Path::new("docs/a.pdf"), &site, true).unwrap();
        assert_eq!(url, "/docs/a.pdf");
    }

    #[test]
    fn no_hash_when_disabled() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("a.png");
        fs::write(&src, b"x").unwrap();
        let site = tmp.path().join("site");
        let url = copy_asset(&src, Path::new("a.png"), &site, false).unwrap();
        assert_eq!(url, "/a.png");
    }
}
