//! Hierarchical navigation tree built from the page list.
//!
//! A [`NavNode`] is either a page leaf (with a URL) or a section (a
//! directory in `docs_dir`) grouping child nodes. The tree is passed to
//! every template via `{{ nav }}`.
//!
//! Ordering is driven by frontmatter:
//! - `weight: N` (integer) ascending; pages without weight sort after.
//! - Then `nav_title:` if present, else page `title:`, else filename stem.
//!
//! Section labels come from `_section.md` inside the directory (its
//! frontmatter `title`), or from the directory name humanized.

use std::path::{Component, Path};

use serde::Serialize;

use crate::page::Page;

#[derive(Debug, Clone, Serialize)]
pub struct NavNode {
    pub title: String,
    pub url: Option<String>,
    pub weight: i32,
    pub children: Vec<NavNode>,
}

impl NavNode {
    pub fn is_section(&self) -> bool {
        self.url.is_none()
    }
}

/// Build the nav tree from the list of pages.
pub fn build(pages: &[Page]) -> Vec<NavNode> {
    // Split each page's relative path into (parent_dirs, filename).
    let mut root = RawNode::section("", "", i32::MAX);
    for page in pages {
        let weight = read_weight(page);
        let nav_title = read_nav_title(page);
        let segments = page_segments(&page.relative);
        insert_page(&mut root, &segments, page, weight, &nav_title);
    }
    let mut out: Vec<NavNode> = root.into_children().into_iter().map(convert).collect();
    out.sort_by(|a, b| a.weight.cmp(&b.weight).then_with(|| a.title.cmp(&b.title)));
    out
}

fn read_weight(page: &Page) -> i32 {
    page.frontmatter
        .get("weight")
        .and_then(|v| v.as_integer())
        .and_then(|v| i32::try_from(v).ok())
        .unwrap_or(i32::MAX / 2)
}

fn read_nav_title(page: &Page) -> String {
    page.frontmatter
        .get("nav_title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| page.title.clone())
}

/// Path segments used to locate the page inside the nav tree. `index.md`
/// represents the directory it sits in, so the index at `guide/index.md`
/// lives under the `guide` section directly.
fn page_segments(relative: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for c in relative.components() {
        if let Component::Normal(os) = c {
            out.push(os.to_string_lossy().into_owned());
        }
    }
    // Drop the trailing `index.md`/`index.markdown`/`_section.md` so the
    // page attaches to its parent section.
    if let Some(last) = out.last() {
        let l = last.to_ascii_lowercase();
        if l == "index.md" || l == "index.markdown" || l == "_section.md" {
            out.pop();
        }
    }
    out
}

fn insert_page(root: &mut RawNode, segments: &[String], page: &Page, weight: i32, nav_title: &str) {
    // Top-level index page: attach directly as a root leaf.
    if segments.is_empty() {
        root.children.push(RawNode::leaf(nav_title.to_string(), page.url.clone(), weight));
        return;
    }

    let mut node = root;
    for (i, seg) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        let is_md = seg.ends_with(".md") || seg.ends_with(".markdown");

        if is_last && is_md {
            // Leaf page inside the current section.
            node.children.push(RawNode::leaf(nav_title.to_string(), page.url.clone(), weight));
            return;
        }

        // Step into a section. Create if missing. Match by the original
        // segment so renaming via `nav_title` doesn't fragment the tree.
        let key = seg.as_str();
        let label = humanize(seg);
        let existing = node.children.iter_mut().position(|c| c.segment_key == key);
        node = match existing {
            Some(idx) => &mut node.children[idx],
            None => {
                node.children.push(RawNode::section(&label, key, i32::MAX));
                node.children.last_mut().unwrap()
            }
        };
    }
    // Index page case: `docs/guide/index.md` -> segments = ["guide"] and
    // we reached here because we stepped into `guide`. Attach the page as
    // the section's own URL so the label is clickable.
    if node.url.is_none() {
        node.url = Some(page.url.clone());
        node.weight = weight.min(node.weight);
        // Prefer the page's nav_title for the section label.
        if !nav_title.is_empty() {
            node.label = nav_title.to_string();
        }
    }
}

fn convert(raw: RawNode) -> NavNode {
    let mut children: Vec<NavNode> = raw.children.into_iter().map(convert).collect();
    children.sort_by(|a, b| a.weight.cmp(&b.weight).then_with(|| a.title.cmp(&b.title)));
    NavNode { title: raw.label, url: raw.url, weight: raw.weight, children }
}

fn humanize(segment: &str) -> String {
    let no_ext = Path::new(segment)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(segment)
        .replace(['-', '_'], " ");
    let mut chars = no_ext.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

#[derive(Debug)]
struct RawNode {
    /// Display label (post-`nav_title` override).
    label: String,
    /// Path segment the section was created from. Used to match sections
    /// across insertions so `nav_title` changing the label doesn't cause
    /// a duplicate section on the next page.
    segment_key: String,
    url: Option<String>,
    weight: i32,
    children: Vec<RawNode>,
}

impl RawNode {
    fn section(label: &str, key: &str, weight: i32) -> Self {
        Self {
            label: label.to_string(),
            segment_key: key.to_string(),
            url: None,
            weight,
            children: Vec::new(),
        }
    }
    fn leaf(label: String, url: String, weight: i32) -> Self {
        Self { label, segment_key: String::new(), url: Some(url), weight, children: Vec::new() }
    }
    fn into_children(self) -> Vec<RawNode> {
        self.children
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{frontmatter::Frontmatter, toc::TocEntry};

    fn p(rel: &str, title: &str, url: &str, weight: Option<i64>) -> Page {
        let mut fm = Frontmatter::new();
        if let Some(w) = weight {
            fm.insert("weight".into(), toml::Value::Integer(w));
        }
        Page {
            relative: PathBuf::from(rel),
            source_abs: PathBuf::from(format!("/tmp/{rel}")),
            url: url.into(),
            output: PathBuf::from("ignored"),
            title: title.into(),
            frontmatter: fm,
            body_html: String::new(),
            toc: Vec::<TocEntry>::new(),
            layout: "default".into(),
        }
    }

    #[test]
    fn flat_pages_are_listed() {
        let nav =
            build(&[p("index.md", "Home", "/", None), p("about.md", "About", "/about/", None)]);
        let titles: Vec<_> = nav.iter().map(|n| n.title.as_str()).collect();
        assert!(titles.contains(&"Home"));
        assert!(titles.contains(&"About"));
    }

    #[test]
    fn subdirs_become_sections() {
        let nav = build(&[
            p("guide/install.md", "Install", "/guide/install/", Some(1)),
            p("guide/config.md", "Config", "/guide/config/", Some(2)),
        ]);
        let guide = nav.iter().find(|n| n.title == "Guide").expect("guide section");
        assert!(guide.is_section());
        assert_eq!(guide.children.len(), 2);
        assert_eq!(guide.children[0].title, "Install");
        assert_eq!(guide.children[1].title, "Config");
    }

    #[test]
    fn weight_controls_order() {
        let nav = build(&[
            p("a.md", "A", "/a/", Some(10)),
            p("b.md", "B", "/b/", Some(1)),
            p("c.md", "C", "/c/", Some(5)),
        ]);
        let titles: Vec<_> = nav.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, vec!["B", "C", "A"]);
    }

    #[test]
    fn section_index_contributes_url() {
        let nav = build(&[
            p("guide/index.md", "Guide overview", "/guide/", Some(1)),
            p("guide/install.md", "Install", "/guide/install/", Some(2)),
        ]);
        let guide = nav.iter().find(|n| n.title == "Guide overview").unwrap();
        assert_eq!(guide.url.as_deref(), Some("/guide/"));
        assert_eq!(guide.children.len(), 1);
    }

    #[test]
    fn humanize_replaces_dashes() {
        assert_eq!(humanize("getting-started"), "Getting started");
        assert_eq!(humanize("api_reference"), "Api reference");
    }
}
