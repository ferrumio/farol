//! Parsing of extended fenced code block info strings + file/region include.
//!
//! A block fence is
//!
//! ```text
//! ```<lang> [attr=value ...]
//! ```
//!
//! where each attribute is either `bare` (boolean flag) or `key=value`
//! (the value may be quoted with `"..."`).
//!
//! Recognized attrs:
//! - `title="..."` or `title=auto`
//! - `file="./path"` - include file content, relative to the markdown source
//! - `lines="10-25"` or `lines="10"` or `lines="10-25,40"`
//! - `region="name"` - extract a named region from the file
//! - `linenums` or `linenums="start=N"`
//! - `no-copy`
//! - `hl_lines="1 3-5"` (mkdocs-material compat)

use std::path::{Path, PathBuf};

use crate::error::FarolError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeBlockAttrs {
    pub lang: String,
    pub title: Option<String>,
    pub file: Option<String>,
    pub lines: Option<Vec<(usize, usize)>>,
    pub region: Option<String>,
    pub linenums: bool,
    pub linenums_start: usize,
    pub hl_lines: Vec<usize>,
    pub no_copy: bool,
}

impl CodeBlockAttrs {
    pub fn parse(info_string: &str) -> Self {
        let mut out = CodeBlockAttrs { linenums_start: 1, ..Default::default() };
        let mut tokens = tokenize(info_string);
        if let Some(first) = tokens.first() {
            if !first.contains('=') {
                out.lang = tokens.remove(0);
            }
        }
        for tok in tokens {
            match tok.split_once('=') {
                None => match tok.as_str() {
                    "linenums" => out.linenums = true,
                    "no-copy" => out.no_copy = true,
                    _ => {}
                },
                Some((k, v)) => {
                    let v = unquote(v);
                    match k {
                        "title" => out.title = Some(v.to_string()),
                        "file" => out.file = Some(v.to_string()),
                        "lines" => out.lines = Some(parse_line_ranges(&v)),
                        "region" => out.region = Some(v.to_string()),
                        "linenums" => {
                            out.linenums = true;
                            if let Some(rest) = v.strip_prefix("start=") {
                                if let Ok(n) = rest.parse() {
                                    out.linenums_start = n;
                                }
                            }
                        }
                        "hl_lines" => out.hl_lines = parse_hl_lines(&v),
                        _ => {}
                    }
                }
            }
        }
        out
    }

    /// Derive a title: explicit `title=` > basename of `file=` > None.
    pub fn effective_title(&self) -> Option<String> {
        if let Some(t) = &self.title {
            if t == "auto" {
                return self.file.as_ref().and_then(basename);
            }
            return Some(t.clone());
        }
        self.file.as_ref().and_then(basename)
    }
}

fn basename(path: &String) -> Option<String> {
    Path::new(path).file_name().and_then(|s| s.to_str()).map(|s| s.to_string())
}

/// Split the info string into whitespace-separated tokens, respecting
/// double-quoted values so `title="hello world"` stays a single token.
fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut in_quote = false;
    for c in s.chars() {
        if c == '"' {
            in_quote = !in_quote;
            buf.push(c);
            continue;
        }
        if c.is_whitespace() && !in_quote {
            if !buf.is_empty() {
                out.push(std::mem::take(&mut buf));
            }
        } else {
            buf.push(c);
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

fn unquote(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn parse_line_ranges(s: &str) -> Vec<(usize, usize)> {
    s.split(',')
        .filter_map(|part| {
            let part = part.trim();
            match part.split_once('-') {
                Some((a, b)) => Some((a.trim().parse().ok()?, b.trim().parse().ok()?)),
                None => {
                    let n: usize = part.parse().ok()?;
                    Some((n, n))
                }
            }
        })
        .collect()
}

fn parse_hl_lines(s: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for part in s.split(|c: char| c.is_whitespace() || c == ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.split_once('-') {
            Some((a, b)) => {
                if let (Ok(a), Ok(b)) = (a.parse::<usize>(), b.parse::<usize>()) {
                    for n in a..=b {
                        out.push(n);
                    }
                }
            }
            None => {
                if let Ok(n) = part.parse::<usize>() {
                    out.push(n);
                }
            }
        }
    }
    out
}

/// Resolve a `file=` path relative to the markdown source and return its
/// contents, possibly filtered by `lines` / `region`.
pub fn read_include(
    page_source: &Path,
    attrs: &CodeBlockAttrs,
    comment_prefixes: &[&'static str],
) -> Result<String, FarolError> {
    let rel = match &attrs.file {
        Some(f) => f,
        None => return Ok(String::new()),
    };
    let abs = resolve_path(page_source, rel);
    let content = std::fs::read_to_string(&abs).map_err(|e| FarolError::io(&abs, e))?;

    let body = if let Some(region_name) = &attrs.region {
        extract_region(&content, region_name, comment_prefixes).ok_or_else(|| {
            FarolError::ConfigInvalid {
                message: format!("region `{}` not found in {}", region_name, abs.display()),
            }
        })?
    } else if let Some(ranges) = &attrs.lines {
        extract_lines(&content, ranges)
    } else {
        content
    };

    Ok(dedent(&body))
}

fn resolve_path(page_source: &Path, rel: &str) -> PathBuf {
    let base = page_source.parent().unwrap_or_else(|| Path::new("."));
    normalize(&base.join(rel))
}

fn normalize(p: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn extract_lines(content: &str, ranges: &[(usize, usize)]) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut out = String::new();
    for (start, end) in ranges {
        let s = start.saturating_sub(1);
        let e = (*end).min(lines.len());
        for line in &lines[s..e] {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Extract a `# region: name ... # endregion: name` block, stripping the
/// markers themselves and preserving any nested content untouched.
fn extract_region(content: &str, name: &str, comment_prefixes: &[&'static str]) -> Option<String> {
    let start_tags: Vec<String> =
        comment_prefixes.iter().map(|p| format!("{p} region: {name}")).collect();
    let end_tags: Vec<String> =
        comment_prefixes.iter().map(|p| format!("{p} endregion: {name}")).collect();

    let mut in_region = false;
    let mut out = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !in_region {
            if start_tags.iter().any(|t| trimmed == t || trimmed.starts_with(t)) {
                in_region = true;
            }
            continue;
        }
        if end_tags.iter().any(|t| trimmed == t || trimmed.starts_with(t)) {
            return Some(out);
        }
        // Strip any nested marker line so output stays clean.
        if is_region_marker(trimmed, comment_prefixes) {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if in_region { Some(out) } else { None }
}

fn is_region_marker(line: &str, prefixes: &[&'static str]) -> bool {
    for p in prefixes {
        let prefix_region = format!("{p} region:");
        let prefix_endregion = format!("{p} endregion:");
        if line.starts_with(&prefix_region) || line.starts_with(&prefix_endregion) {
            return true;
        }
    }
    false
}

fn dedent(text: &str) -> String {
    let min_indent = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    if min_indent == 0 {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if line.len() >= min_indent {
            out.push_str(&line[min_indent..]);
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_lang() {
        let a = CodeBlockAttrs::parse("python");
        assert_eq!(a.lang, "python");
        assert!(a.file.is_none());
    }

    #[test]
    fn parses_file_and_title() {
        let a = CodeBlockAttrs::parse(r#"python file="./examples/x.py" title="my example""#);
        assert_eq!(a.lang, "python");
        assert_eq!(a.file.as_deref(), Some("./examples/x.py"));
        assert_eq!(a.title.as_deref(), Some("my example"));
    }

    #[test]
    fn parses_lines() {
        let a = CodeBlockAttrs::parse(r#"py lines="10-25,40""#);
        assert_eq!(a.lines, Some(vec![(10, 25), (40, 40)]));
    }

    #[test]
    fn parses_linenums_start() {
        let a = CodeBlockAttrs::parse(r#"py linenums="start=42""#);
        assert!(a.linenums);
        assert_eq!(a.linenums_start, 42);
    }

    #[test]
    fn effective_title_auto_from_file() {
        let a = CodeBlockAttrs::parse(r#"py file="./examples/foo.py""#);
        assert_eq!(a.effective_title().as_deref(), Some("foo.py"));
    }

    #[test]
    fn hl_lines_parsed() {
        let a = CodeBlockAttrs::parse(r#"py hl_lines="1 3-5""#);
        assert_eq!(a.hl_lines, vec![1, 3, 4, 5]);
    }

    #[test]
    fn regions_extract_and_dedent() {
        let src = "\
pre
# region: body
    def x():
        return 1
# endregion: body
post
";
        let got = extract_region(src, "body", &["#"]).unwrap();
        let cleaned = dedent(&got);
        assert_eq!(cleaned, "def x():\n    return 1\n");
    }

    #[test]
    fn nested_regions_are_isolated() {
        let src = "\
# region: outer
outer-before
# region: inner
inner-content
# endregion: inner
outer-after
# endregion: outer
";
        let inner = extract_region(src, "inner", &["#"]).unwrap();
        assert_eq!(inner.trim(), "inner-content");
    }

    #[test]
    fn extract_lines_range() {
        let src = "one\ntwo\nthree\nfour\nfive\n";
        let out = extract_lines(src, &[(2, 3)]);
        assert_eq!(out, "two\nthree\n");
    }

    #[test]
    fn multiple_line_ranges() {
        let src = "a\nb\nc\nd\ne\n";
        let out = extract_lines(src, &[(1, 2), (4, 4)]);
        assert_eq!(out, "a\nb\nd\n");
    }
}
