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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::frontmatter::Frontmatter;

    fn page(rel: &str) -> Page {
        Page {
            relative: PathBuf::from(rel),
            source_abs: PathBuf::from(format!("/tmp/{rel}")),
            url: "/x/".into(),
            output: PathBuf::from("x/index.html"),
            title: "x".into(),
            frontmatter: Frontmatter::new(),
            body_html: String::new(),
            toc: Vec::new(),
        }
    }

    #[test]
    fn no_repo_no_link() {
        let out = EditOnGitPlugin
            .on_page_html("<p>body</p>".into(), &page("a.md"), &Config::default())
            .unwrap();
        assert!(!out.contains("edit-on-git"));
    }

    #[test]
    fn builds_github_url() {
        let cfg = Config {
            repo_url: Some("https://github.com/ferrumio/farol".into()),
            edit_uri: Some("edit/main/docs/".into()),
            ..Config::default()
        };
        let out = EditOnGitPlugin
            .on_page_html("<p>body</p>".into(), &page("guide/install.md"), &cfg)
            .unwrap();
        assert!(out.contains(
            r#"href="https://github.com/ferrumio/farol/edit/main/docs/guide/install.md""#
        ));
        assert!(out.contains(r#"target="_blank""#));
    }

    #[test]
    fn default_edit_uri_applied() {
        let cfg =
            Config { repo_url: Some("https://github.com/foo/bar".into()), ..Config::default() };
        let out =
            EditOnGitPlugin.on_page_html("<p>body</p>".into(), &page("index.md"), &cfg).unwrap();
        assert!(out.contains(r#"href="https://github.com/foo/bar/edit/main/docs/index.md""#));
    }
}
