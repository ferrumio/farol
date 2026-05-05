//! Injects prev/next navigation links at the bottom of each page.
//!
//! Uses the order pages appear during the build (alphabetical by relative
//! path for v0.1; a custom nav order comes in v0.2). Collects pages on
//! `on_page_html`, then at `on_post_build` patches each written HTML file
//! with a `<nav class="prev-next">` block.

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::{Config, FarolError, Page, PluginHost, Result};

pub struct PrevNextPlugin {
    pages: Mutex<Vec<PageInfo>>,
}

#[derive(Clone)]
struct PageInfo {
    url: String,
    title: String,
    output: PathBuf,
}

impl Default for PrevNextPlugin {
    fn default() -> Self {
        Self { pages: Mutex::new(Vec::new()) }
    }
}

impl PluginHost for PrevNextPlugin {
    fn name(&self) -> &str {
        "prev-next"
    }

    fn plugins(&self) -> Vec<String> {
        vec!["prev-next".into()]
    }

    fn on_page_html(&self, html: String, page: &Page, _config: &Config) -> Result<String> {
        self.pages.lock().unwrap().push(PageInfo {
            url: page.url.clone(),
            title: page.title.clone(),
            output: page.output.clone(),
        });
        Ok(html)
    }

    fn on_post_build(&self, site_dir: &Path, _config: &Config) -> Result<()> {
        let mut pages: Vec<PageInfo> = {
            let mut guard = self.pages.lock().unwrap();
            std::mem::take(&mut *guard)
        };
        pages.sort_by(|a, b| a.url.cmp(&b.url));

        for i in 0..pages.len() {
            let prev = if i == 0 { None } else { pages.get(i - 1) };
            let next = pages.get(i + 1);
            if prev.is_none() && next.is_none() {
                continue;
            }
            patch_page(site_dir, &pages[i], prev, next)?;
        }
        Ok(())
    }
}

fn patch_page(
    site_dir: &Path,
    current: &PageInfo,
    prev: Option<&PageInfo>,
    next: Option<&PageInfo>,
) -> Result<()> {
    let path = site_dir.join(&current.output);
    let html = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let block = render_nav(prev, next);
    let patched = if let Some(idx) = html.rfind("</main>") {
        let mut out = String::with_capacity(html.len() + block.len());
        out.push_str(&html[..idx]);
        out.push_str(&block);
        out.push_str(&html[idx..]);
        out
    } else if let Some(idx) = html.rfind("</body>") {
        let mut out = String::with_capacity(html.len() + block.len());
        out.push_str(&html[..idx]);
        out.push_str(&block);
        out.push_str(&html[idx..]);
        out
    } else {
        format!("{html}{block}")
    };

    std::fs::write(&path, patched).map_err(|e| FarolError::io(&path, e))?;
    Ok(())
}

fn render_nav(prev: Option<&PageInfo>, next: Option<&PageInfo>) -> String {
    let mut out = String::from(r#"<nav class="prev-next" aria-label="Page navigation">"#);
    match prev {
        Some(p) => out.push_str(&format!(
            r#"<a class="prev" href="{url}"><span class="hint">Previous</span><span class="title">{title}</span></a>"#,
            url = escape_attr(&p.url),
            title = escape_html(&p.title),
        )),
        None => out.push_str(r#"<span class="prev disabled"></span>"#),
    }
    match next {
        Some(n) => out.push_str(&format!(
            r#"<a class="next" href="{url}"><span class="hint">Next</span><span class="title">{title}</span></a>"#,
            url = escape_attr(&n.url),
            title = escape_html(&n.title),
        )),
        None => out.push_str(r#"<span class="next disabled"></span>"#),
    }
    out.push_str("</nav>");
    out
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::frontmatter::Frontmatter;

    fn p(url: &str, title: &str, out: &str) -> Page {
        Page {
            relative: PathBuf::from(format!("{title}.md")),
            source_abs: PathBuf::from(format!("/tmp/{title}.md")),
            url: url.into(),
            output: PathBuf::from(out),
            title: title.into(),
            frontmatter: Frontmatter::new(),
            body_html: String::new(),
            toc: Vec::new(),
            layout: "default".to_string(),
        }
    }

    #[test]
    fn inserts_nav_on_middle_page() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path();
        let plugin = PrevNextPlugin::default();
        let cfg = Config::default();

        for name in ["a", "b", "c"] {
            std::fs::create_dir_all(site.join(name)).unwrap();
            std::fs::write(site.join(name).join("index.html"), format!("<main>{name}</main>"))
                .unwrap();
            let pg = p(&format!("/{name}/"), name, &format!("{name}/index.html"));
            plugin.on_page_html(pg.body_html.clone(), &pg, &cfg).unwrap();
        }

        plugin.on_post_build(site, &cfg).unwrap();

        let b = std::fs::read_to_string(site.join("b").join("index.html")).unwrap();
        assert!(b.contains(r#"class="prev-next""#));
        assert!(b.contains(r#"href="/a/""#));
        assert!(b.contains(r#"href="/c/""#));
    }

    #[test]
    fn first_page_only_has_next() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path();
        let plugin = PrevNextPlugin::default();
        let cfg = Config::default();

        for name in ["a", "b"] {
            std::fs::create_dir_all(site.join(name)).unwrap();
            std::fs::write(site.join(name).join("index.html"), format!("<main>{name}</main>"))
                .unwrap();
            let pg = p(&format!("/{name}/"), name, &format!("{name}/index.html"));
            plugin.on_page_html(pg.body_html.clone(), &pg, &cfg).unwrap();
        }
        plugin.on_post_build(site, &cfg).unwrap();

        let a = std::fs::read_to_string(site.join("a").join("index.html")).unwrap();
        assert!(a.contains(r#"class="prev disabled""#));
        assert!(a.contains(r#"href="/b/""#));
    }

    #[test]
    fn single_page_skips_nav() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path();
        let plugin = PrevNextPlugin::default();
        let cfg = Config::default();
        std::fs::write(site.join("index.html"), "<main>only</main>").unwrap();
        let pg = p("/", "only", "index.html");
        plugin.on_page_html(pg.body_html.clone(), &pg, &cfg).unwrap();
        plugin.on_post_build(site, &cfg).unwrap();

        let out = std::fs::read_to_string(site.join("index.html")).unwrap();
        assert!(!out.contains("prev-next"));
    }
}
