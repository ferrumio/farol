//! GitHub-flavored alerts rendered as admonition boxes.
//!
//! Input syntax (GFM alert):
//!
//! ```markdown
//! > [!NOTE]
//! > Highlights information that users should take into account.
//!
//! > [!WARNING]
//! > Critical content demanding immediate user attention due to potential risks.
//! ```
//!
//! Types recognized: `NOTE`, `TIP`, `IMPORTANT`, `WARNING`, `CAUTION`.
//!
//! The plugin post-processes the rendered HTML and replaces matching
//! `<blockquote>` elements with `<div class="admonition {type}">`, adding
//! a header so the theme can style them.

use crate::{Config, Page, PluginHost, Result};

pub struct AdmonitionsPlugin;

const TYPES: &[(&str, &str)] = &[
    ("NOTE", "Note"),
    ("TIP", "Tip"),
    ("IMPORTANT", "Important"),
    ("WARNING", "Warning"),
    ("CAUTION", "Caution"),
];

impl PluginHost for AdmonitionsPlugin {
    fn name(&self) -> &str {
        "admonitions"
    }

    fn plugins(&self) -> Vec<String> {
        vec!["admonitions".into()]
    }

    fn on_page_html(&self, html: String, _page: &Page, _config: &Config) -> Result<String> {
        Ok(transform(&html))
    }
}

fn transform(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0;
    let bytes = html.as_bytes();
    while cursor < bytes.len() {
        if let Some(start) = html[cursor..].find("<blockquote>") {
            let bq_start = cursor + start;
            out.push_str(&html[cursor..bq_start]);
            let inner_start = bq_start + "<blockquote>".len();
            if let Some(end) = html[inner_start..].find("</blockquote>") {
                let inner_end = inner_start + end;
                let inner = &html[inner_start..inner_end];
                match detect_alert(inner) {
                    Some((kind, title, rest)) => {
                        out.push_str(&format!(
                            r#"<div class="admonition {slug}"><div class="admonition-title">{title}</div>"#,
                            slug = kind.to_ascii_lowercase(),
                        ));
                        out.push_str(rest.trim_start());
                        out.push_str("</div>");
                    }
                    None => {
                        out.push_str("<blockquote>");
                        out.push_str(inner);
                        out.push_str("</blockquote>");
                    }
                }
                cursor = inner_end + "</blockquote>".len();
            } else {
                out.push_str(&html[bq_start..]);
                break;
            }
        } else {
            out.push_str(&html[cursor..]);
            break;
        }
    }
    out
}

/// If the first meaningful content matches `[!TYPE]`, return
/// `(TYPE, title, remaining_inner)`.
fn detect_alert(inner: &str) -> Option<(&'static str, &'static str, &str)> {
    // markdown-rs typically emits the blockquote as
    //   <blockquote>\n<p>[!NOTE]\ntext</p>\n</blockquote>
    // so we look for a `<p>[!` prefix after leading whitespace.
    let trimmed = inner.trim_start();
    let after_p = trimmed.strip_prefix("<p>")?;
    let after_mark = after_p.strip_prefix("[!")?;
    let close = after_mark.find(']')?;
    let kind = &after_mark[..close];

    let (canon, title) = TYPES.iter().copied().find(|(k, _)| k.eq_ignore_ascii_case(kind))?;

    // Skip past the closing bracket to find the actual alert body inside
    // the blockquote's inner HTML.
    let prefix_len = inner.len() - inner.trim_start().len();
    let consumed = prefix_len + 3 /* "<p>" */ + 2 /* "[!" */ + close + 1;
    let mut tail = &inner[consumed..];
    tail = tail.trim_start_matches(|c: char| c.is_whitespace());
    tail = tail.strip_prefix("<br />").unwrap_or(tail);
    tail = tail.strip_prefix("<br/>").unwrap_or(tail);
    tail = tail.trim_start_matches(|c: char| c.is_whitespace());

    Some((canon, title, tail))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_alert_is_transformed() {
        let input = "<blockquote>\n<p>[!NOTE]<br />\nHello world</p>\n</blockquote>";
        let out = transform(input);
        assert!(out.contains(r#"<div class="admonition note">"#));
        assert!(out.contains(r#"<div class="admonition-title">Note</div>"#));
        assert!(out.contains("Hello world"));
    }

    #[test]
    fn unknown_tag_is_left_alone() {
        let input = "<blockquote>\n<p>[!MYSTERY]<br />\nHello</p>\n</blockquote>";
        let out = transform(input);
        assert!(out.contains("<blockquote>"));
        assert!(!out.contains("admonition"));
    }

    #[test]
    fn plain_blockquote_unchanged() {
        let input = "<blockquote>\n<p>normal quote</p>\n</blockquote>";
        let out = transform(input);
        assert_eq!(out, input);
    }

    #[test]
    fn multiple_alerts_transformed() {
        let input = "<blockquote>\n<p>[!NOTE]<br />\nfirst</p>\n</blockquote><p>gap</p>\
            <blockquote>\n<p>[!WARNING]<br />\nsecond</p>\n</blockquote>";
        let out = transform(input);
        assert!(out.contains("admonition note"));
        assert!(out.contains("admonition warning"));
        assert!(out.contains("<p>gap</p>"));
    }

    #[test]
    fn case_insensitive() {
        let input = "<blockquote>\n<p>[!note]<br />\nlower</p>\n</blockquote>";
        let out = transform(input);
        assert!(out.contains("admonition note"));
    }
}
