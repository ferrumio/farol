use std::path::{Component, Path, PathBuf};

/// Convert a source-relative path like `guide/install.md` into a site URL
/// like `/guide/install/`. Files named `index.md` collapse to the parent
/// directory URL.
pub fn site_url_for(relative: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in relative.components() {
        if let Component::Normal(os) = component {
            parts.push(os.to_string_lossy().into_owned());
        }
    }

    if let Some(last) = parts.last_mut() {
        let p = Path::new(last);
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
        if (ext == "md" || ext == "markdown") && stem == "index" {
            parts.pop();
        } else if ext == "md" || ext == "markdown" {
            *last = stem;
        }
    }

    let mut out = String::from("/");
    out.push_str(&parts.join("/"));
    if !out.ends_with('/') {
        out.push('/');
    }
    out
}

/// Convert a site URL like `/guide/install/` into the filesystem path written
/// under `site_dir`: `guide/install/index.html`.
pub fn output_path_for(url: &str) -> PathBuf {
    let trimmed = url.trim_matches('/');
    if trimmed.is_empty() {
        PathBuf::from("index.html")
    } else {
        PathBuf::from(trimmed).join("index.html")
    }
}

/// Classify a link target as internal-markdown, internal-asset, external, or
/// anchor-only.
#[derive(Debug, PartialEq, Eq)]
pub enum LinkKind<'a> {
    External,
    Anchor,
    InternalMarkdown(&'a str),
    InternalOther(&'a str),
}

pub fn classify_link(href: &str) -> LinkKind<'_> {
    if href.starts_with('#') {
        return LinkKind::Anchor;
    }
    if href.contains("://") || href.starts_with("mailto:") || href.starts_with("//") {
        return LinkKind::External;
    }
    let path_part = href.split('#').next().unwrap_or(href);
    let ext = Path::new(path_part)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if ext == "md" || ext == "markdown" {
        LinkKind::InternalMarkdown(href)
    } else {
        LinkKind::InternalOther(href)
    }
}

/// Resolve an internal markdown link relative to the current page's relative
/// path. Returns `(target_relative_md_path, anchor)`.
pub fn resolve_internal(page_relative: &Path, href: &str) -> Option<(PathBuf, Option<String>)> {
    let (path_part, anchor) = match href.split_once('#') {
        Some((p, a)) => (p, Some(a.to_string())),
        None => (href, None),
    };
    let base = page_relative.parent().unwrap_or_else(|| Path::new(""));
    let joined = base.join(path_part);
    Some((normalize(&joined), anchor))
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_index() {
        assert_eq!(site_url_for(Path::new("index.md")), "/");
    }

    #[test]
    fn pretty_url_for_page() {
        assert_eq!(site_url_for(Path::new("guide/install.md")), "/guide/install/");
    }

    #[test]
    fn nested_index() {
        assert_eq!(site_url_for(Path::new("guide/index.md")), "/guide/");
    }

    #[test]
    fn preserves_assets() {
        assert_eq!(site_url_for(Path::new("img/logo.png")), "/img/logo.png/");
    }

    #[test]
    fn output_paths() {
        assert_eq!(output_path_for("/"), PathBuf::from("index.html"));
        assert_eq!(output_path_for("/guide/install/"), PathBuf::from("guide/install/index.html"));
    }

    #[test]
    fn classifies_links() {
        assert_eq!(classify_link("https://example.com"), LinkKind::External);
        assert_eq!(classify_link("mailto:a@b.com"), LinkKind::External);
        assert_eq!(classify_link("#section"), LinkKind::Anchor);
        assert_eq!(classify_link("./other.md"), LinkKind::InternalMarkdown("./other.md"));
        assert_eq!(classify_link("../img/logo.png"), LinkKind::InternalOther("../img/logo.png"));
    }

    #[test]
    fn resolves_relative_markdown() {
        let (rel, anchor) = resolve_internal(Path::new("guide/install.md"), "../index.md").unwrap();
        assert_eq!(rel, PathBuf::from("index.md"));
        assert_eq!(anchor, None);
    }

    #[test]
    fn resolves_with_anchor() {
        let (rel, anchor) =
            resolve_internal(Path::new("guide/install.md"), "config.md#env").unwrap();
        assert_eq!(rel, PathBuf::from("guide/config.md"));
        assert_eq!(anchor.as_deref(), Some("env"));
    }
}
