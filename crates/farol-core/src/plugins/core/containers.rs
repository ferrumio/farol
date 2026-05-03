//! `::: tabs` and `::: files` container directives.
//!
//! Rendered at `on_page_markdown` time, before the highlight plugin, so
//! code blocks inside still go through syntax highlighting normally. The
//! containers only wrap child blocks in a tabbed / grouped structure.
//!
//! `::: tabs` expects child sections separated by `###` headings; each
//! heading becomes a tab label. `::: files` is the same but uses `####`
//! and renders the sections stacked (no tab switching).

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{Config, Page, PluginHost, Result};

static GROUP_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct ContainersPlugin;

impl PluginHost for ContainersPlugin {
    fn name(&self) -> &str {
        "containers"
    }

    fn plugins(&self) -> Vec<String> {
        vec!["containers".into()]
    }

    fn on_page_markdown(&self, markdown: String, _page: &Page, _config: &Config) -> Result<String> {
        Ok(transform(&markdown))
    }
}

fn transform(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let lines: Vec<&str> = markdown.split_inclusive('\n').collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        if trimmed.starts_with(":::") {
            let after = trimmed.trim_start_matches(':').trim();
            let kind = after.split_whitespace().next().unwrap_or("");
            if kind == "tabs" || kind == "files" {
                // Find the closing `:::` line.
                let mut depth = 1;
                let mut end = i + 1;
                while end < lines.len() {
                    let t = lines[end].trim();
                    if t.starts_with(":::") {
                        let body = t.trim_start_matches(':').trim();
                        let body_kind = body.split_whitespace().next().unwrap_or("");
                        if body.is_empty() {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        } else if body_kind == "tabs" || body_kind == "files" {
                            depth += 1;
                        }
                    }
                    end += 1;
                }

                if end < lines.len() {
                    let inner: String = lines[i + 1..end].concat();
                    let rendered = match kind {
                        "tabs" => render_tabs(&inner),
                        "files" => render_files(&inner),
                        _ => inner,
                    };
                    out.push_str(&rendered);
                    i = end + 1;
                    continue;
                }
            }
        }

        out.push_str(line);
        i += 1;
    }
    out
}

#[derive(Debug, Clone)]
struct Section {
    label: String,
    body: String,
}

/// Split container body into `(label, body)` sections by `###` (tabs) or
/// `####` (files) heading lines.
fn split_sections(inner: &str, depth_marker: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<Section> = None;
    for line in inner.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(depth_marker) {
            if rest.starts_with(' ') || rest.starts_with('\t') || rest.is_empty() {
                if let Some(s) = current.take() {
                    sections.push(s);
                }
                let label = rest.trim().trim_matches('#').trim().to_string();
                current = Some(Section { label, body: String::new() });
                continue;
            }
        }
        if let Some(s) = current.as_mut() {
            s.body.push_str(line);
        }
    }
    if let Some(s) = current.take() {
        sections.push(s);
    }
    sections
}

fn render_tabs(inner: &str) -> String {
    let sections = split_sections(inner, "###");
    if sections.is_empty() {
        return inner.to_string();
    }
    let group_id = GROUP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut out = String::new();
    out.push_str(&format!(r#"<div class="farol-tabs" data-group="{group_id}">"#));
    // Tab buttons (inert without JS; the first panel is shown via .active).
    out.push_str(r#"<div class="tab-buttons" role="tablist">"#);
    for (idx, s) in sections.iter().enumerate() {
        let active = if idx == 0 { " active" } else { "" };
        out.push_str(&format!(
            r#"<button type="button" class="tab-button{active}" role="tab" data-tab="{idx}">{label}</button>"#,
            label = escape_html(&s.label),
        ));
    }
    out.push_str("</div>");

    out.push_str(r#"<div class="tab-panels">"#);
    for (idx, s) in sections.iter().enumerate() {
        let active = if idx == 0 { " active" } else { "" };
        // The body must stay valid markdown so the markdown parser + the
        // highlight plugin process it. We wrap in a raw HTML div by
        // emitting the open tag, a blank line, then the body, then close.
        out.push_str(&format!(r#"<div class="tab-panel{active}" data-tab="{idx}">"#));
        out.push('\n');
        out.push('\n');
        out.push_str(&s.body);
        out.push_str("</div>");
    }
    out.push_str("</div>");
    out.push_str("</div>\n");
    out
}

fn render_files(inner: &str) -> String {
    let sections = split_sections(inner, "####");
    if sections.is_empty() {
        return inner.to_string();
    }
    let mut out = String::new();
    out.push_str(r#"<div class="farol-files">"#);
    for s in &sections {
        out.push_str(&format!(
            r#"<div class="file-entry"><div class="file-label">{}</div>"#,
            escape_html(&s.label),
        ));
        out.push('\n');
        out.push('\n');
        out.push_str(&s.body);
        out.push_str("</div>");
    }
    out.push_str("</div>\n");
    out
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_wraps_sections() {
        let md = "::: tabs\n### Python\n```python\nprint(1)\n```\n\n### Rust\n```rust\nfn main() {}\n```\n:::\n";
        let out = transform(md);
        assert!(out.contains("farol-tabs"));
        assert!(out.contains(r#"data-tab="0""#));
        assert!(out.contains(r#"data-tab="1""#));
        assert!(out.contains("Python"));
        assert!(out.contains("Rust"));
        // Code blocks remain as markdown fences for the highlight plugin.
        assert!(out.contains("```python"));
        assert!(out.contains("```rust"));
    }

    #[test]
    fn files_wraps_sections() {
        let md =
            "::: files\n#### config.toml\n```toml\nkey = 1\n```\n\n#### main.py\n```python\nprint(1)\n```\n:::\n";
        let out = transform(md);
        assert!(out.contains("farol-files"));
        assert!(out.contains("file-label"));
        assert!(out.contains("config.toml"));
        assert!(out.contains("main.py"));
    }

    #[test]
    fn unterminated_container_left_alone() {
        let md = "::: tabs\n### Python\n```py\nx\n```\nno-end\n";
        let out = transform(md);
        assert!(out.contains("::: tabs"));
    }

    #[test]
    fn unknown_container_passes_through() {
        let md = "::: whatever\ncontent\n:::\n";
        let out = transform(md);
        assert!(out.contains("::: whatever"));
    }

    #[test]
    fn nested_content_preserved() {
        let md = "before\n::: tabs\n### A\nhello\n### B\nworld\n:::\nafter\n";
        let out = transform(md);
        assert!(out.starts_with("before"));
        assert!(out.contains("hello"));
        assert!(out.contains("world"));
        assert!(out.trim_end().ends_with("after"));
    }
}
