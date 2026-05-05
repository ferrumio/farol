//! Builtin search plugin.
//!
//! Collects each rendered page's plain-text content on `on_page_html`,
//! builds a `tantivy` index on `on_post_build`, then serializes it as
//! compact JSON under `site/assets/search/`. The default theme's
//! JS client loads these files and performs the query side of search.

use std::sync::Mutex;

use crate::{
    search::{self, SearchEntry},
    Config, Page, PluginHost, Result,
};

pub struct SearchPlugin {
    entries: Mutex<Vec<SearchEntry>>,
}

impl Default for SearchPlugin {
    fn default() -> Self {
        Self { entries: Mutex::new(Vec::new()) }
    }
}

impl PluginHost for SearchPlugin {
    fn name(&self) -> &str {
        "search"
    }

    fn plugins(&self) -> Vec<String> {
        vec!["search".into()]
    }

    fn on_page_html(&self, html: String, page: &Page, _config: &Config) -> Result<String> {
        let body = strip_tags(&html);
        self.entries.lock().unwrap().push(SearchEntry {
            url: page.url.clone(),
            title: page.title.clone(),
            section: None,
            body,
        });
        Ok(html)
    }

    fn on_post_build(&self, site_dir: &std::path::Path, config: &Config) -> Result<()> {
        let entries: Vec<SearchEntry> = {
            let mut guard = self.entries.lock().unwrap();
            std::mem::take(&mut *guard)
        };
        // Plugins can still mutate entries here via the chained host path.
        // (The plugin's own `on_search_index` is the identity.)
        let index = search::build_index(&entries)?;
        search::write_to_site(site_dir, &index)?;
        let _ = config;
        Ok(())
    }
}

/// Remove HTML tags and collapse whitespace. Used to produce plain-text
/// bodies for the search index. UTF-8 safe (walks by chars, not bytes).
fn strip_tags(html: &str) -> String {
    // First pass: drop `<script>...</script>` and `<style>...</style>` blocks.
    let cleaned = drop_scripted(html);

    // Second pass: drop everything between `<` and `>`.
    let mut buf = String::with_capacity(cleaned.len());
    let mut in_tag = false;
    for c in cleaned.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => buf.push(c),
            _ => {}
        }
    }

    // Decode the common entities we emit elsewhere.
    let decoded = buf
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    // Collapse whitespace.
    let mut out = String::with_capacity(decoded.len());
    let mut last_was_ws = false;
    for c in decoded.chars() {
        if c.is_whitespace() {
            if !last_was_ws && !out.is_empty() {
                out.push(' ');
                last_was_ws = true;
            }
        } else {
            out.push(c);
            last_was_ws = false;
        }
    }
    out.trim_end().to_string()
}

/// Remove `<script>...</script>` and `<style>...</style>` blocks entirely,
/// case-insensitive.
fn drop_scripted(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0;
    while cursor < html.len() {
        // Find next <script or <style (whichever comes first).
        let script_at = lower[cursor..].find("<script").map(|i| (cursor + i, "</script>"));
        let style_at = lower[cursor..].find("<style").map(|i| (cursor + i, "</style>"));
        let next = match (script_at, style_at) {
            (Some(s), Some(t)) => {
                if s.0 <= t.0 {
                    Some(s)
                } else {
                    Some(t)
                }
            }
            (s, t) => s.or(t),
        };
        let Some((start, end_tag)) = next else {
            out.push_str(&html[cursor..]);
            break;
        };
        out.push_str(&html[cursor..start]);
        let after = start + 1;
        let end_idx = lower[after..].find(end_tag).map(|i| after + i + end_tag.len());
        match end_idx {
            Some(e) => cursor = e,
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;
    use crate::frontmatter::Frontmatter;

    fn page(url: &str, title: &str) -> Page {
        Page {
            relative: PathBuf::from(format!("{title}.md")),
            source_abs: PathBuf::from(format!("/tmp/{title}.md")),
            url: url.into(),
            output: PathBuf::from(format!("{url}index.html")),
            title: title.into(),
            frontmatter: Frontmatter::new(),
            body_html: String::new(),
            toc: Vec::new(),
            layout: "default".to_string(),
        }
    }

    #[test]
    fn strips_tags_and_keeps_text() {
        let html = "<p>Hello <em>world</em>, this is <code>farol</code>.</p>";
        assert_eq!(strip_tags(html), "Hello world, this is farol.");
    }

    #[test]
    fn collapses_whitespace_and_drops_script() {
        let html = "<p>keep</p>\n\n<script>var x = 1;</script><p>also</p>";
        let out = strip_tags(html);
        assert!(!out.contains("var x"));
        assert!(out.contains("keep"));
        assert!(out.contains("also"));
    }

    #[test]
    fn writes_search_assets_on_post_build() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path();

        let plugin = SearchPlugin::default();
        let cfg = Config::default();
        for (url, title, html) in [
            ("/a/", "Alpha", "<p>Alpha is the first</p>"),
            ("/b/", "Beta", "<p>Beta is the second letter</p>"),
        ] {
            plugin.on_page_html(html.into(), &page(url, title), &cfg).unwrap();
        }
        plugin.on_post_build(site, &cfg).unwrap();

        let docs_path = site.join("assets/search/docs.json");
        let index_path = site.join("assets/search/index.json");
        assert!(docs_path.exists());
        assert!(index_path.exists());

        let docs: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&docs_path).unwrap()).unwrap();
        assert_eq!(docs.as_array().unwrap().len(), 2);

        let index: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&index_path).unwrap()).unwrap();
        assert_eq!(index["version"], 1);
        let map = index["index"].as_object().unwrap();
        assert!(map.contains_key("alpha"), "missing alpha in {map:?}");
    }
}
