//! Plugin host abstraction.
//!
//! `farol-core` never loads Python or WASM. It just calls hook methods on a
//! [`PluginHost`] trait object at the right points in the build. Concrete
//! hosts live in their own crates:
//!
//! - `farol-py` - Python plugins via PyO3 + pluggy + entry points
//! - (future) `farol-wasm` - WebAssembly component plugins
//!
//! Tests and the standalone `cargo build` path use [`NoOpHost`], which passes
//! every hook through unchanged.

use std::path::Path;

use crate::{config::Config, error::Result, files::FileTree, page::Page};

/// The full set of hooks a plugin runtime can provide.
///
/// All methods have default implementations that pass their input through
/// unchanged, so runtimes only implement the hooks they actually dispatch.
pub trait PluginHost: Send + Sync {
    /// Called once after `farol.toml` is loaded. Plugins may mutate config.
    fn on_config(&self, config: Config) -> Result<Config> {
        Ok(config)
    }

    /// Called once after the filesystem walk. Plugins may add, remove, or
    /// reorder files before the build sees them.
    fn on_files(&self, files: FileTree, _config: &Config) -> Result<FileTree> {
        Ok(files)
    }

    /// Called after the nav tree is built (placeholder for v0.2 when nav
    /// becomes a real object). For v0.1 the nav is implicit.
    fn on_nav(&self, _pages: &[Page], _config: &Config) -> Result<()> {
        Ok(())
    }

    /// Called per page with the raw Markdown *after* frontmatter is stripped.
    /// Plugins may rewrite the Markdown before parsing.
    fn on_page_markdown(&self, markdown: String, _page: &Page, _config: &Config) -> Result<String> {
        Ok(markdown)
    }

    /// Called per page with the rendered HTML body, after internal link
    /// resolution. Plugins may rewrite the HTML before the template wraps it.
    fn on_page_html(&self, html: String, _page: &Page, _config: &Config) -> Result<String> {
        Ok(html)
    }

    /// Called once at the end of a build, after all pages and assets are
    /// written. Useful for post-processing (sitemap augmentations, etc.).
    fn on_post_build(&self, _site_dir: &Path, _config: &Config) -> Result<()> {
        Ok(())
    }

    /// Human-readable identifier for logging / error messages.
    fn name(&self) -> &str {
        "no-op"
    }

    /// Names of plugins currently registered with this host.
    fn plugins(&self) -> Vec<String> {
        Vec::new()
    }
}

/// A [`PluginHost`] that does nothing. Used when no plugin runtime is
/// attached (pure-Rust builds, unit tests).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpHost;

impl PluginHost for NoOpHost {}

use std::sync::Arc;

/// Composes multiple hosts. Hooks run in order - earlier hosts see the
/// original input; later hosts see the accumulated result.
///
/// Used to apply Rust builtin plugins after a user's Python host transforms.
pub struct ChainedHost {
    hosts: Vec<Arc<dyn PluginHost>>,
    display_name: String,
}

impl ChainedHost {
    pub fn new(hosts: Vec<Arc<dyn PluginHost>>) -> Self {
        let display_name =
            hosts.iter().map(|h| h.name().to_string()).collect::<Vec<_>>().join(" + ");
        Self { hosts, display_name }
    }

    /// Convenience constructor accepting owned boxes. Internally converts to
    /// `Arc` so additional references can be cheaply shared.
    pub fn from_boxes(hosts: Vec<Box<dyn PluginHost>>) -> Self {
        Self::new(hosts.into_iter().map(Arc::from).collect())
    }
}

impl PluginHost for ChainedHost {
    fn name(&self) -> &str {
        &self.display_name
    }

    fn plugins(&self) -> Vec<String> {
        let mut out = Vec::new();
        for h in &self.hosts {
            out.extend(h.plugins());
        }
        out
    }

    fn on_config(&self, mut config: Config) -> Result<Config> {
        for h in &self.hosts {
            config = h.on_config(config)?;
        }
        Ok(config)
    }

    fn on_files(&self, mut files: FileTree, config: &Config) -> Result<FileTree> {
        for h in &self.hosts {
            files = h.on_files(files, config)?;
        }
        Ok(files)
    }

    fn on_nav(&self, pages: &[Page], config: &Config) -> Result<()> {
        for h in &self.hosts {
            h.on_nav(pages, config)?;
        }
        Ok(())
    }

    fn on_page_markdown(
        &self,
        mut markdown: String,
        page: &Page,
        config: &Config,
    ) -> Result<String> {
        for h in &self.hosts {
            markdown = h.on_page_markdown(markdown, page, config)?;
        }
        Ok(markdown)
    }

    fn on_page_html(&self, mut html: String, page: &Page, config: &Config) -> Result<String> {
        for h in &self.hosts {
            html = h.on_page_html(html, page, config)?;
        }
        Ok(html)
    }

    fn on_post_build(&self, site_dir: &Path, config: &Config) -> Result<()> {
        for h in &self.hosts {
            h.on_post_build(site_dir, config)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::frontmatter::Frontmatter;

    #[test]
    fn no_op_passes_through() {
        let host = NoOpHost;
        let cfg = Config::default();
        let cfg2 = host.on_config(cfg.clone()).unwrap();
        assert_eq!(cfg.site_name, cfg2.site_name);

        let md = host.on_page_markdown("# hi".into(), &sample_page(), &cfg).unwrap();
        assert_eq!(md, "# hi");
    }

    fn sample_page() -> Page {
        Page {
            relative: PathBuf::from("index.md"),
            source_abs: PathBuf::from("/tmp/index.md"),
            url: "/".into(),
            output: PathBuf::from("index.html"),
            title: "hi".into(),
            frontmatter: Frontmatter::new(),
            body_html: String::new(),
            toc: Vec::new(),
        }
    }
}
