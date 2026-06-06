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
