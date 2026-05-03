//! Adds a copy-to-clipboard button to every code block.
//!
//! On the server side we only wrap each `<pre>` with a container and a
//! `<button>` element. The click handler and clipboard write live in the
//! theme's vanilla JS (served by the default theme).

use crate::{Config, Page, PluginHost, Result};

pub struct CodeCopyPlugin;

impl PluginHost for CodeCopyPlugin {
    fn name(&self) -> &str {
        "code-copy"
    }

    fn plugins(&self) -> Vec<String> {
        vec!["code-copy".into()]
    }

    fn on_page_html(&self, html: String, _page: &Page, _config: &Config) -> Result<String> {
        Ok(inject_buttons(&html))
    }
}

fn inject_buttons(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0;
    let len = html.len();

    while cursor < len {
        match html[cursor..].find("<pre>") {
            Some(rel_start) => {
                let start = cursor + rel_start;
                out.push_str(&html[cursor..start]);
                // Only wrap raw <pre>, not our highlight block (which already
                // has a container we can target).
                out.push_str(
                    r#"<div class="code-copy-wrap"><button class="code-copy" type="button" aria-label="Copy code">Copy</button>"#,
                );
                // Consume original <pre> opening to its matching </pre>
                let close = match html[start..].find("</pre>") {
                    Some(i) => start + i + "</pre>".len(),
                    None => {
                        out.push_str(&html[start..]);
                        return out;
                    }
                };
                out.push_str(&html[start..close]);
                out.push_str("</div>");
                cursor = close;
            }
            None => {
                out.push_str(&html[cursor..]);
                break;
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_pre_and_adds_button() {
        let html = "<pre><code>x</code></pre>";
        let out = inject_buttons(html);
        assert!(out.starts_with(r#"<div class="code-copy-wrap">"#));
        assert!(out.contains(r#"class="code-copy""#));
        assert!(out.contains("<pre><code>x</code></pre>"));
        assert!(out.ends_with("</div>"));
    }

    #[test]
    fn multiple_pre_blocks() {
        let html = "<pre>a</pre>text<pre>b</pre>";
        let out = inject_buttons(html);
        assert_eq!(out.matches("code-copy-wrap").count(), 2);
        assert!(out.contains("text"));
    }

    #[test]
    fn unterminated_pre_left_as_is() {
        let html = "<pre>no end";
        let out = inject_buttons(html);
        assert!(out.contains("<pre>no end"));
    }

    #[test]
    fn no_pre_no_change() {
        assert_eq!(inject_buttons("<p>hi</p>"), "<p>hi</p>");
    }
}
