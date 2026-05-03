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
            url: "/".into(),
            output: PathBuf::from("index.html"),
            title: "hi".into(),
            frontmatter: Frontmatter::new(),
            body_html: String::new(),
            toc: Vec::new(),
        }
    }
}
