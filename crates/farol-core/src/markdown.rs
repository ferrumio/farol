use std::{collections::HashSet, path::Path};

use markdown::{
    mdast::{Heading, Node},
    Constructs, Options, ParseOptions,
};

use crate::{
    error::{FarolError, Result},
    slug::unique_slug,
};

/// Parsed result of a markdown source.
#[derive(Debug, Clone)]
pub struct ParsedMarkdown {
    /// Raw HTML body.
    pub html: String,
    /// First H1 text, or `None` if no H1 exists.
    pub title: Option<String>,
    /// Flat heading list: `(level, text, slug)`.
    pub headings: Vec<(u8, String, String)>,
}

/// Parse markdown text into HTML + heading metadata.
pub fn parse(text: &str, path: &Path) -> Result<ParsedMarkdown> {
    let parse_opts = parse_options();
    let ast = markdown::to_mdast(text, &parse_opts).map_err(|e| FarolError::Frontmatter {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;

    let mut headings_src: Vec<(u8, String)> = Vec::new();
    collect_headings(&ast, &mut headings_src);

    let mut seen = HashSet::new();
    let headings: Vec<(u8, String, String)> = headings_src
        .into_iter()
        .map(|(lvl, txt)| {
            let slug = unique_slug(&txt, &mut seen);
            (lvl, txt, slug)
        })
        .collect();

    let title = headings.iter().find(|(lvl, _, _)| *lvl == 1).map(|(_, t, _)| t.clone());

    let opts = Options {
        parse: parse_opts,
        compile: markdown::CompileOptions {
            // Markdown comes from the site author, not untrusted users; HTML
            // passthrough is expected (plugins like syntax-highlight emit
            // ready-made HTML blocks in `on_page_markdown`).
            allow_dangerous_html: true,
            allow_dangerous_protocol: false,
            ..markdown::CompileOptions::gfm()
        },
    };
    let html = markdown::to_html_with_options(text, &opts).map_err(|e| {
        FarolError::Frontmatter { path: path.to_path_buf(), message: e.to_string() }
    })?;

    Ok(ParsedMarkdown { html, title, headings })
}

fn parse_options() -> ParseOptions {
    ParseOptions {
        constructs: Constructs {
            frontmatter: false, // frontmatter is stripped separately by `frontmatter::split`
            ..Constructs::gfm()
        },
        ..ParseOptions::gfm()
    }
}

fn collect_headings(node: &Node, out: &mut Vec<(u8, String)>) {
    if let Node::Heading(Heading { depth, children, .. }) = node {
        out.push((*depth, render_inline(children)));
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_headings(child, out);
        }
    }
}

fn render_inline(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(&t.value),
            Node::InlineCode(c) => out.push_str(&c.value),
            Node::Emphasis(e) => out.push_str(&render_inline(&e.children)),
            Node::Strong(s) => out.push_str(&render_inline(&s.children)),
            Node::Link(l) => out.push_str(&render_inline(&l.children)),
            Node::Delete(d) => out.push_str(&render_inline(&d.children)),
            _ => {
                if let Some(children) = node.children() {
                    out.push_str(&render_inline(children));
                }
            }
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn extracts_title_from_h1() {
        let p = PathBuf::from("p.md");
        let out = parse("# Hello World\n\nbody", &p).unwrap();
        assert_eq!(out.title.as_deref(), Some("Hello World"));
        assert!(out.html.contains("<h1>Hello World</h1>"));
    }

    #[test]
    fn collects_headings_with_slugs() {
        let src = "# One\n\n## Two\n\n## Two\n\n### Three";
        let out = parse(src, &PathBuf::from("p.md")).unwrap();
        let slugs: Vec<&str> = out.headings.iter().map(|(_, _, s)| s.as_str()).collect();
        assert_eq!(slugs, vec!["one", "two", "two-1", "three"]);
    }

    #[test]
    fn no_title_when_no_h1() {
        let out = parse("## Only H2\n", &PathBuf::from("p.md")).unwrap();
        assert!(out.title.is_none());
    }

    #[test]
    fn renders_gfm_tables() {
        let src = "| a | b |\n| - | - |\n| 1 | 2 |\n";
        let out = parse(src, &PathBuf::from("p.md")).unwrap();
        assert!(out.html.contains("<table>"));
        assert!(out.html.contains("<td>1</td>"));
    }

    #[test]
    fn renders_strikethrough() {
        let out = parse("~~gone~~", &PathBuf::from("p.md")).unwrap();
        assert!(out.html.contains("<del>gone</del>"));
    }

    #[test]
    fn renders_task_list() {
        let src = "- [x] done\n- [ ] todo\n";
        let out = parse(src, &PathBuf::from("p.md")).unwrap();
        assert!(out.html.contains("type=\"checkbox\""));
    }
}
