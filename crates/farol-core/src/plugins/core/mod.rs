//! Builtin plugins shipped with the engine.
//!
//! Each plugin implements [`crate::PluginHost`] and is stacked onto the
//! user-provided host via [`crate::ChainedHost`]. They demonstrate the
//! public plugin API by using nothing else: no private hooks, no core
//! privileges.

pub mod admonitions;
pub mod anchor_links;
pub mod code_copy;
pub mod code_include;
pub mod containers;
pub mod edit_on_git;
pub mod highlight;
pub mod prev_next;
pub mod reading_time;
pub mod redirects;
pub mod search;
pub mod sitemap;

use crate::PluginHost;

/// Return all builtin hosts ready to be composed.
///
/// The order matters: markdown/html transformations run top-to-bottom, so
/// earlier entries see input the later ones have not yet touched.
///
/// Containers expand to raw HTML with `<div>` wrappers that still contain
/// fenced code blocks, so they must run *before* the highlight plugin.
pub fn all() -> Vec<Box<dyn PluginHost>> {
    vec![
        Box::new(containers::ContainersPlugin),
        Box::new(highlight::HighlightPlugin::new()),
        Box::new(admonitions::AdmonitionsPlugin),
        Box::new(anchor_links::AnchorLinksPlugin),
        Box::new(code_copy::CodeCopyPlugin),
        Box::new(reading_time::ReadingTimePlugin),
        Box::new(edit_on_git::EditOnGitPlugin),
        Box::new(prev_next::PrevNextPlugin::default()),
        Box::new(search::SearchPlugin::default()),
        Box::new(redirects::RedirectsPlugin),
        Box::new(sitemap::SitemapPlugin::default()),
    ]
}
