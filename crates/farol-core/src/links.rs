use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::url::{LinkKind, classify_link, resolve_internal};

/// A rewrite applied to the HTML of a page: `old_href -> new_href`.
#[derive(Debug, Clone)]
pub struct LinkRewrite {
    pub from: String,
    pub to: String,
}

/// A link that could not be resolved to an existing page.
#[derive(Debug, Clone)]
pub struct BrokenLink {
    pub page: PathBuf,
    pub href: String,
    pub reason: &'static str,
}

/// Resolve all markdown links found in `html` against a map of known pages
/// (`relative_md_path -> site_url`). Returns rewrites to apply plus a list of
/// broken links for reporting.
pub fn resolve_in_html(
    page_relative: &Path,
    html: &str,
    known_pages: &HashMap<PathBuf, String>,
) -> (Vec<LinkRewrite>, Vec<BrokenLink>) {
    let mut rewrites = Vec::new();
    let mut broken = Vec::new();

    for href in extract_hrefs(html) {
        match classify_link(&href) {
            LinkKind::InternalMarkdown(_) => {
                if let Some((target, anchor)) = resolve_internal(page_relative, &href) {
                    if let Some(url) = known_pages.get(&target) {
                        let mut new_href = url.clone();
                        if let Some(a) = anchor {
                            new_href.push('#');
                            new_href.push_str(&a);
                        }
                        rewrites.push(LinkRewrite { from: href, to: new_href });
                    } else {
                        broken.push(BrokenLink {
                            page: page_relative.to_path_buf(),
                            href,
                            reason: "no such page",
                        });
                    }
                }
            }
            LinkKind::External | LinkKind::Anchor | LinkKind::InternalOther(_) => {}
        }
    }

    (rewrites, broken)
}

/// Apply rewrites to HTML. Simple string replace is correct here because each
/// `from` is the exact original href emitted by markdown-rs in `href="..."`.
pub fn apply_rewrites(html: &str, rewrites: &[LinkRewrite]) -> String {
    let mut out = html.to_string();
    for r in rewrites {
        let old = format!(r#"href="{}""#, r.from);
        let new = format!(r#"href="{}""#, r.to);
        out = out.replace(&old, &new);
    }
    out
}

/// Extract the contents of every `href="..."` attribute in a small-HTML string.
/// This is intentionally simple: markdown-rs emits predictable output, and we
/// only need to find the hrefs it wrote.
fn extract_hrefs(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let needle = "href=\"";
    let bytes = html.as_bytes();
    let mut i = 0;
    while i + needle.len() < bytes.len() {
        if &bytes[i..i + needle.len()] == needle.as_bytes() {
            let start = i + needle.len();
            if let Some(end) = html[start..].find('"') {
                out.push(html[start..start + end].to_string());
                i = start + end + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn rewrites_internal_markdown_link() {
        let mut known = HashMap::new();
        known.insert(PathBuf::from("other.md"), "/other/".to_string());

        let html = r#"<p>See <a href="./other.md">here</a>.</p>"#;
        let (rw, broken) = resolve_in_html(Path::new("index.md"), html, &known);
        assert_eq!(rw.len(), 1);
        assert!(broken.is_empty());

        let out = apply_rewrites(html, &rw);
        assert!(out.contains(r#"href="/other/""#));
    }

    #[test]
    fn broken_link_reported() {
        let known = HashMap::new();
        let html = r#"<a href="missing.md">x</a>"#;
        let (_, broken) = resolve_in_html(Path::new("index.md"), html, &known);
        assert_eq!(broken.len(), 1);
    }

    #[test]
    fn external_and_anchor_ignored() {
        let known = HashMap::new();
        let html = r##"<a href="https://x.com">e</a> <a href="#top">t</a>"##;
        let (rw, broken) = resolve_in_html(Path::new("p.md"), html, &known);
        assert!(rw.is_empty());
        assert!(broken.is_empty());
    }

    #[test]
    fn preserves_anchor_in_rewrite() {
        let mut known = HashMap::new();
        known.insert(PathBuf::from("guide.md"), "/guide/".to_string());
        let html = r#"<a href="./guide.md#section">s</a>"#;
        let (rw, _) = resolve_in_html(Path::new("index.md"), html, &known);
        assert_eq!(rw[0].to, "/guide/#section");
    }
}
