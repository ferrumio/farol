//! Server-side syntax highlighting + extended fence metadata.
//!
//! Intercepts fenced code blocks before the markdown parser sees them,
//! parses the info string attributes, optionally includes external files
//! (region / line ranges), runs syntax highlighting via `syntect`, and
//! emits ready-to-render HTML.
//!
//! Info string attributes (see `code_include::CodeBlockAttrs` for the
//! canonical parser):
//!
//! - `title="..."` - renders a header above the block
//! - `file="./path"` - include content from disk (relative to the page)
//! - `lines="10-25"` - filter to line ranges
//! - `region="name"` - extract a named region (uses `# region: name` /
//!   `# endregion: name` markers in the source)
//! - `linenums` / `linenums="start=N"` - show line numbers
//! - `hl_lines="1 3-5"` - explicit highlight (MkDocs-material compat)
//! - `no-copy` - hide the copy button for this block
//!
//! Inline markers in the code itself (`# !mark`, `// !add`, `// !del`,
//! `# !focus`) still work and compose with `hl_lines`.

use std::path::Path;

use syntect::{
    highlighting::{Style, Theme, ThemeSet},
    html::{IncludeBackground, styled_line_to_highlighted_html},
    parsing::{SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};

use super::code_include::{CodeBlockAttrs, read_include};
use crate::{Config, Page, PluginHost, Result};

pub struct HighlightPlugin {
    syntaxes: SyntaxSet,
    theme: Theme,
}

impl HighlightPlugin {
    pub fn new() -> Self {
        // `two-face` bundles ~220 sublime-syntax definitions from `bat`,
        // covering TypeScript, TOML, Dockerfile, Kotlin, Swift, Zig and
        // most modern languages that syntect's own defaults miss.
        let syntaxes = two_face::syntax::extra_newlines();
        let themes = ThemeSet::load_defaults();
        let theme = themes.themes["base16-ocean.light"].clone();
        Self { syntaxes, theme }
    }

    fn syntax_for(&self, token: &str) -> Option<&SyntaxReference> {
        if token.is_empty() {
            return None;
        }
        self.syntaxes
            .find_syntax_by_token(token)
            .or_else(|| self.syntaxes.find_syntax_by_name(token))
            .or_else(|| self.syntaxes.find_syntax_by_extension(token))
    }

    fn highlight_block(&self, info_string: &str, body: &str, page_source: &Path) -> Result<String> {
        let attrs = CodeBlockAttrs::parse(info_string);
        let comment_prefixes = comment_prefixes_for(&attrs.lang);

        // If `file="..."` is present and the body is empty, pull from disk.
        let code: String = if attrs.file.is_some() && body.trim().is_empty() {
            read_include(page_source, &attrs, &comment_prefixes)?
        } else {
            body.to_string()
        };

        // Strip markers, collect per-line markers.
        let mut cleaned_lines: Vec<String> = Vec::new();
        let mut markers: Vec<LineMarker> = Vec::new();
        for raw in LinesWithEndings::from(&code) {
            let (marker, clean) = extract_marker(raw, &comment_prefixes);
            cleaned_lines.push(clean);
            markers.push(marker);
        }
        let has_focus = markers.iter().any(|m| matches!(m, LineMarker::Focus));

        // Apply hl_lines on top (merge with inline markers).
        for n in &attrs.hl_lines {
            let idx = n.saturating_sub(1);
            if let Some(slot) = markers.get_mut(idx) {
                if *slot == LineMarker::None {
                    *slot = LineMarker::Mark;
                }
            }
        }

        // Highlight each line.
        let syntax = self.syntax_for(&attrs.lang);
        let mut line_htmls: Vec<String> = Vec::new();
        if let Some(syntax) = syntax {
            let mut highlighter = syntect::easy::HighlightLines::new(syntax, &self.theme);
            for line in &cleaned_lines {
                let ranges: Vec<(Style, &str)> = highlighter
                    .highlight_line(line, &self.syntaxes)
                    .unwrap_or_else(|_| vec![(Style::default(), line.as_str())]);
                let html = styled_line_to_highlighted_html(&ranges, IncludeBackground::No)
                    .unwrap_or_else(|_| escape_html(line));
                line_htmls.push(html);
            }
        } else {
            for line in &cleaned_lines {
                line_htmls.push(escape_html(line));
            }
        }

        // Render.
        Ok(render_block(&attrs, &line_htmls, &markers, has_focus))
    }
}

fn render_block(
    attrs: &CodeBlockAttrs,
    line_htmls: &[String],
    markers: &[LineMarker],
    has_focus: bool,
) -> String {
    let mut out = String::new();

    let classes = {
        let mut c = String::from("farol-codeblock");
        if attrs.linenums {
            c.push_str(" with-linenums");
        }
        if attrs.no_copy {
            c.push_str(" no-copy");
        }
        c
    };
    out.push_str(&format!(r#"<div class="{classes}" data-lang="{}">"#, escape_attr(&attrs.lang)));

    if let Some(title) = attrs.effective_title() {
        out.push_str(&format!(
            r#"<div class="codeblock-header"><span class="filename">{}</span></div>"#,
            escape_html(&title)
        ));
    }

    out.push_str("<pre><code>");
    for (idx, (html, marker)) in line_htmls.iter().zip(markers.iter()).enumerate() {
        let class = line_class(marker, has_focus);
        out.push_str(&format!(r#"<span class="{class}">"#));
        if attrs.linenums {
            let n = attrs.linenums_start + idx;
            out.push_str(&format!(r#"<span class="linenum">{n}</span>"#));
        }
        out.push_str(html);
        out.push_str("</span>");
    }
    out.push_str("</code></pre></div>");

    out
}

impl Default for HighlightPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginHost for HighlightPlugin {
    fn name(&self) -> &str {
        "syntax-highlight"
    }

    fn plugins(&self) -> Vec<String> {
        vec!["syntax-highlight".into()]
    }

    fn on_page_markdown(&self, markdown: String, page: &Page, _config: &Config) -> Result<String> {
        let source = page.source_abs.clone();
        rewrite_fenced_blocks(&markdown, |info, code| self.highlight_block(info, code, &source))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineMarker {
    None,
    Mark,
    Add,
    Del,
    Focus,
}

fn line_class(m: &LineMarker, has_focus: bool) -> &'static str {
    match (m, has_focus) {
        (LineMarker::Mark, _) => "line mark",
        (LineMarker::Add, _) => "line add",
        (LineMarker::Del, _) => "line del",
        (LineMarker::Focus, _) => "line focus",
        (LineMarker::None, true) => "line dim",
        (LineMarker::None, false) => "line",
    }
}

fn extract_marker(line: &str, prefixes: &[&'static str]) -> (LineMarker, String) {
    let stripped = line.trim_end_matches(['\n', '\r']);
    let newline = &line[stripped.len()..];

    for prefix in prefixes {
        if let Some(idx) = stripped.rfind(prefix) {
            let after = stripped[idx + prefix.len()..].trim();
            let marker = match after {
                "!mark" => LineMarker::Mark,
                "!add" => LineMarker::Add,
                "!del" => LineMarker::Del,
                "!focus" => LineMarker::Focus,
                _ => continue,
            };
            let mut out = stripped[..idx].trim_end_matches([' ', '\t']).to_string();
            out.push_str(newline);
            return (marker, out);
        }
    }
    (LineMarker::None, line.to_string())
}

pub(crate) fn comment_prefixes_for(lang: &str) -> Vec<&'static str> {
    match lang.to_ascii_lowercase().as_str() {
        "rust" | "rs" | "c" | "cpp" | "c++" | "cs" | "csharp" | "java" | "kotlin" | "kt"
        | "swift" | "go" | "js" | "javascript" | "ts" | "typescript" | "jsx" | "tsx" | "dart"
        | "scala" | "groovy" | "php" => vec!["//"],
        "python" | "py" | "ruby" | "rb" | "shell" | "sh" | "bash" | "zsh" | "fish" | "yaml"
        | "yml" | "toml" | "ini" | "conf" | "dockerfile" | "docker" | "r" | "perl" | "pl"
        | "elixir" | "ex" | "exs" | "makefile" | "make" => vec!["#"],
        "sql" | "haskell" | "hs" | "ada" | "lua" => vec!["--"],
        "html" | "xml" | "svg" | "vue" | "svelte" => vec!["<!--"],
        "css" | "scss" | "sass" | "less" => vec!["/*"],
        _ => vec!["//", "#"],
    }
}

/// Walk the markdown, forward each fenced block's info string + body to `cb`.
/// `cb` returns the replacement HTML (or bubbles up a build error).
pub(crate) fn rewrite_fenced_blocks(
    markdown: &str,
    mut cb: impl FnMut(&str, &str) -> Result<String>,
) -> Result<String> {
    let mut out = String::with_capacity(markdown.len());
    let mut cursor = 0;
    let len = markdown.len();

    while cursor < len {
        let line_end = markdown[cursor..].find('\n').map(|n| cursor + n).unwrap_or(len);
        let line = &markdown[cursor..line_end];
        let stripped = line.trim_start();

        if !stripped.starts_with("```") {
            out.push_str(line);
            if line_end < len {
                out.push('\n');
            }
            cursor = line_end + 1;
            continue;
        }

        let info_string = stripped[3..].trim();

        let body_start = if line_end < len { line_end + 1 } else { len };
        let mut scan = body_start;
        let mut body_end = None;
        let mut closing_end = None;
        while scan < len {
            let nl = markdown[scan..].find('\n').map(|n| scan + n).unwrap_or(len);
            let candidate = markdown[scan..nl].trim();
            if candidate.starts_with("```") {
                body_end = Some(scan);
                closing_end = Some(nl);
                break;
            }
            scan = nl + 1;
        }

        match (body_end, closing_end) {
            (Some(b), Some(c)) => {
                let code = &markdown[body_start..b];
                out.push_str(&cb(info_string, code)?);
                out.push('\n');
                cursor = if c < len { c + 1 } else { len };
            }
            _ => {
                out.push_str(&markdown[cursor..]);
                break;
            }
        }
    }

    Ok(out)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
}
