//! Inject heading anchors so readers can deep-link to sections.
//!
//! `markdown-rs` emits plain `<h2>Some title</h2>`. We add the slug-based
//! `id` and an invisible anchor icon inside the heading so users can hover
//! and copy the link. The slug is computed from the heading text using the
//! same function as the TOC so anchors stay in sync.

use crate::{slug::slugify, Config, Page, PluginHost, Result};

pub struct AnchorLinksPlugin;

impl PluginHost for AnchorLinksPlugin {
    fn name(&self) -> &str {
        "anchor-links"
    }

    fn plugins(&self) -> Vec<String> {
        vec!["anchor-links".into()]
    }

    fn on_page_html(&self, html: String, _page: &Page, _config: &Config) -> Result<String> {
        Ok(inject_anchors(&html))
    }
}

fn inject_anchors(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0;
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    while cursor < len {
        let rest = &html[cursor..];
        // Match <hN> for N in 2..=4.
        let (tag_len, depth) = match rest.as_bytes().get(0..4) {
            Some(b"<h2>") => (4, 2),
            Some(b"<h3>") => (4, 3),
            Some(b"<h4>") => (4, 4),
            _ => {
                let next = char_boundary_after(html, cursor);
                out.push_str(&html[cursor..next]);
                cursor = next;
                continue;
            }
        };

        let close_tag = match depth {
            2 => "</h2>",
            3 => "</h3>",
            4 => "</h4>",
            _ => unreachable!(),
        };
        let close_idx = match rest[tag_len..].find(close_tag) {
            Some(i) => tag_len + i,
            None => {
                let next = char_boundary_after(html, cursor);
                out.push_str(&html[cursor..next]);
                cursor = next;
                continue;
            }
        };

        let inner = &rest[tag_len..close_idx];
        let text = strip_tags(inner);
        let base_slug = slugify(&text);
        let mut slug = base_slug.clone();
        let mut n = 1;
        while seen.contains(&slug) {
            slug = format!("{base_slug}-{n}");
            n += 1;
        }
        seen.insert(slug.clone());

        out.push_str(&format!(
            r##"<h{depth} id="{slug}">{inner}<a class="heading-anchor" href="#{slug}" aria-label="Permalink">#</a></h{depth}>"##
        ));
        cursor += close_idx + close_tag.len();
    }

    out
}

/// Advance `cursor` by one UTF-8 char. Returns the index after it.
fn char_boundary_after(s: &str, cursor: usize) -> usize {
    let bytes = s.as_bytes();
    let mut i = cursor + 1;
    while i < bytes.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_id_and_anchor() {
        let html = "<h2>Hello World</h2>";
        let out = inject_anchors(html);
        assert!(out.contains(r#"id="hello-world""#));
        assert!(out.contains(r##"href="#hello-world""##));
        assert!(out.contains("heading-anchor"));
    }

    #[test]
    fn h3_supported() {
        let out = inject_anchors("<h3>Sub</h3>");
        assert!(out.contains(r#"<h3 id="sub">"#));
    }

    #[test]
    fn duplicates_get_suffixes() {
        let out = inject_anchors("<h2>Same</h2><h2>Same</h2>");
        assert!(out.contains(r#"id="same""#));
        assert!(out.contains(r#"id="same-1""#));
    }

    #[test]
    fn preserves_inline_markup() {
        let out = inject_anchors("<h2>Hello <em>world</em></h2>");
        assert!(out.contains(r#"id="hello-world""#));
        assert!(out.contains("<em>world</em>"));
    }

    #[test]
    fn leaves_other_content_alone() {
        let out = inject_anchors("<p>nothing</p>");
        assert_eq!(out, "<p>nothing</p>");
    }
}
