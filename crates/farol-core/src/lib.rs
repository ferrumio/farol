//! farol-core: the engine.
//!
//! Forged in Rust. Lit for life.

pub mod assets;
pub mod build;
pub mod cache;
pub mod config;
pub mod error;
pub mod files;
pub mod frontmatter;
pub mod graph;
pub mod hash;
pub mod links;
pub mod markdown;
pub mod page;
pub mod plugin;
pub mod scaffold;
pub mod slug;
pub mod theme;
pub mod toc;
pub mod url;

pub use build::{build, build_with, BuildOptions, BuildReport};
pub use cache::Cache;
pub use config::{Config, DEFAULT_CONFIG_FILENAME};
pub use error::{FarolError, Result};
pub use files::{File, FileKind, FileTree};
pub use graph::{Graph, Node, NodeOutcome, NodeTiming, Report as GraphReport};
pub use hash::{Hash, Hasher};
pub use page::Page;
pub use plugin::{NoOpHost, PluginHost};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
