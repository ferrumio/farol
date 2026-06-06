//! Adds an "Edit this page" link to each page, pointing at the markdown
//! source on the project's git forge.
//!
//! Driven by two config fields (already in `Config`):
//! - `repo_url` - root of the repo (e.g. `https://github.com/ferrumio/farol`)
//! - `edit_uri` - path suffix for the edit view (e.g. `edit/main/docs/`)
//!
//! For each page, injects a `<a class="edit-on-git">` tag right before
//! the closing `</main>` (or `</body>`, fallback) pointing at
//! `<repo_url>/<edit_uri>/<page.relative>`.

use crate::{Config, Page, PluginHost, Result};

pub struct EditOnGitPlugin;

impl PluginHost for EditOnGitPlugin {
    fn name(&self) -> &str {
        "edit-on-git"
    }

    fn plugins(&self) -> Vec<String> {
        vec!["edit-on-git".into()]
    }

    fn on_page_html(&self, html: String, page: &Page, config: &Config) -> Result<String> {
        let Some(url) = build_url(page, config) else {
            return Ok(html);
        };
        let link = format!(
            r#"<a class="edit-on-git" href="{url}" target="_blank" rel="noopener noreferrer">Edit this page</a>"#,
            url = escape_attr(&url),
        );
        // Prepend so the theme can position it at the top right, typical for
        // docs sites (Material, Docusaurus).
        let mut out = String::with_capacity(html.len() + link.len());
        out.push_str(&link);
        out.push_str(&html);
        Ok(out)
    }
}

fn build_url(page: &Page, config: &Config) -> Option<String> {
    let repo = config.repo_url.as_deref()?.trim_end_matches('/');
    let edit = config.edit_uri.as_deref().unwrap_or("edit/main/docs/").trim_matches('/');
    let rel = page.relative.to_string_lossy().replace('\\', "/");
    Some(format!("{repo}/{edit}/{rel}"))
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
}
