use std::path::PathBuf;

use serde::Serialize;

use crate::{frontmatter::Frontmatter, toc::TocEntry};

/// A single rendered page, ready to be written to disk or passed to a template.
#[derive(Debug, Clone, Serialize)]
pub struct Page {
    /// Source path relative to `docs_dir` (e.g. `guide/install.md`).
    pub relative: PathBuf,
    /// Site URL (e.g. `/guide/install/`).
    pub url: String,
    /// Output path under `site_dir` (e.g. `guide/install/index.html`).
    pub output: PathBuf,
    /// Page title (frontmatter `title:` > first H1 > filename stem).
    pub title: String,
    /// Frontmatter as a generic TOML table.
    pub frontmatter: Frontmatter,
    /// Rendered body HTML (after link resolution).
    pub body_html: String,
    /// Nested table of contents.
    pub toc: Vec<TocEntry>,
}
