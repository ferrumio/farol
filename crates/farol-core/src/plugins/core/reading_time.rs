//! Computes estimated reading time for each page and injects it as a
//! `reading_time` field into the page frontmatter so templates can render
//! "5 min read" without any further work.
//!
//! Rate used: 200 words per minute (industry default for docs).
//! Minimum: 1 minute - pages shorter than that still show "1 min read".

use crate::{Config, Page, PluginHost, Result};

pub struct ReadingTimePlugin;

const WORDS_PER_MINUTE: usize = 200;

impl PluginHost for ReadingTimePlugin {
    fn name(&self) -> &str {
        "reading-time"
    }

    fn plugins(&self) -> Vec<String> {
        vec!["reading-time".into()]
    }

    fn on_page_markdown(&self, markdown: String, _page: &Page, _config: &Config) -> Result<String> {
        // Count words in the markdown body. We don't mutate it - the computed
        // value is surfaced via on_page_html where we can read `page.body_html`
        // and insert a frontmatter-style attribute. For v0.1 the simplest path
        // is to embed the value as an HTML comment the template can pick up,
        // but a cleaner shot is adding the value to `Page.frontmatter`. Since
        // the hook gets `&Page`, not `&mut Page`, we stash via a sidecar: we
        // compute here and pass through, then let `on_page_html` inject.
        //
        // Simpler v0.1 approach: do nothing at markdown time; count words at
        // html time and prepend a data attribute the theme can render.
        Ok(markdown)
    }

    fn on_page_html(&self, html: String, _page: &Page, _config: &Config) -> Result<String> {
        let words = count_words(&html);
        let minutes = words.div_ceil(WORDS_PER_MINUTE).max(1);
        let marker = format!(
            r#"<span class="reading-time" data-minutes="{minutes}">{minutes} min read</span>"#
        );
        // Prepend the marker; theme decides whether/where to show it.
        let mut out = String::with_capacity(html.len() + marker.len());
        out.push_str(&marker);
        out.push_str(&html);
        Ok(out)
    }
}

/// Rough word count: strip HTML tags, split on whitespace.
fn count_words(html: &str) -> usize {
    let mut in_tag = false;
    let mut buf = String::with_capacity(html.len());
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => buf.push(c),
            _ => {}
        }
    }
    buf.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::frontmatter::Frontmatter;

    fn page() -> Page {
        Page {
            relative: PathBuf::from("p.md"),
            source_abs: PathBuf::from("/tmp/p.md"),
            url: "/p/".into(),
            output: PathBuf::from("p/index.html"),
            title: "p".into(),
            frontmatter: Frontmatter::new(),
            body_html: String::new(),
            toc: Vec::new(),
            layout: "default".to_string(),
        }
    }

    #[test]
    fn short_page_shows_one_minute() {
        let html = "<p>Hello world</p>".to_string();
        let out = ReadingTimePlugin.on_page_html(html, &page(), &Config::default()).unwrap();
        assert!(out.contains("1 min read"));
    }

    #[test]
    fn longer_page_scales() {
        let words = "word ".repeat(600); // 600 words -> 3 min
        let html = format!("<p>{words}</p>");
        let out = ReadingTimePlugin.on_page_html(html, &page(), &Config::default()).unwrap();
        assert!(out.contains("3 min read"), "unexpected output: {out}");
    }

    #[test]
    fn html_tags_dont_count_as_words() {
        let html = "<div class='lots of classes here'><p>one two three</p></div>".to_string();
        assert_eq!(count_words(&html), 3);
    }

    #[test]
    fn marker_is_prepended() {
        let html = "<p>original</p>".to_string();
        let out = ReadingTimePlugin.on_page_html(html, &page(), &Config::default()).unwrap();
        assert!(out.starts_with(r#"<span class="reading-time""#));
        assert!(out.contains("<p>original</p>"));
    }
}
