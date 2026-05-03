//! Builtin sitemap + robots.txt generation, ported from the old inline
//! implementation in `build.rs` so we dogfood the public `on_post_build`
//! hook.
//!
//! Because the hook doesn't see the page list directly, we stash the pages
//! in a side-channel via a global registration during `on_page_html`. That's
//! ugly, so the next iteration will enrich `on_post_build` with a site
//! summary. For v0.1 it's enough.
//!
//! To keep this self-contained, the plugin tracks the (url, title) pairs it
//! has seen during the build and emits them on `on_post_build`.

use std::{path::Path, sync::Mutex};

use crate::{Config, FarolError, Page, PluginHost, Result};

pub struct SitemapPlugin {
    seen: Mutex<Vec<String>>,
}

impl Default for SitemapPlugin {
    fn default() -> Self {
        Self { seen: Mutex::new(Vec::new()) }
    }
}

impl SitemapPlugin {
    pub const fn name_str() -> &'static str {
        "sitemap"
    }
}

impl PluginHost for SitemapPlugin {
    fn name(&self) -> &str {
        Self::name_str()
    }

    fn plugins(&self) -> Vec<String> {
        vec![Self::name_str().to_string()]
    }

    fn on_page_html(&self, html: String, page: &Page, _config: &Config) -> Result<String> {
        self.seen.lock().unwrap().push(page.url.clone());
        Ok(html)
    }

    fn on_post_build(&self, site_dir: &Path, config: &Config) -> Result<()> {
        let urls: Vec<String> = {
            let mut guard = self.seen.lock().unwrap();
            std::mem::take(&mut *guard)
        };

        let base = config.site_url.clone().unwrap_or_default();
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
        for url in &urls {
            xml.push_str("  <url><loc>");
            xml.push_str(&format!("{}{}", base.trim_end_matches('/'), url));
            xml.push_str("</loc></url>\n");
        }
        xml.push_str("</urlset>\n");

        let sitemap_path = site_dir.join("sitemap.xml");
        std::fs::write(&sitemap_path, xml).map_err(|e| FarolError::io(&sitemap_path, e))?;

        let mut robots = String::from("User-agent: *\nAllow: /\n");
        if let Some(url) = &config.site_url {
            robots.push_str(&format!("Sitemap: {}/sitemap.xml\n", url.trim_end_matches('/')));
        }
        let robots_path = site_dir.join("robots.txt");
        std::fs::write(&robots_path, robots).map_err(|e| FarolError::io(&robots_path, e))?;

        Ok(())
    }
}
