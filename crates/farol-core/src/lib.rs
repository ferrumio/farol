//! farol-core: the engine.
//!
//! Forged in Rust. Lit for life.

pub mod assets;
pub mod build;
pub mod config;
pub mod error;
pub mod files;
pub mod frontmatter;
pub mod links;
pub mod markdown;
pub mod page;
pub mod scaffold;
pub mod slug;
pub mod theme;
pub mod toc;
pub mod url;

pub use build::{build, BuildReport};
pub use config::{Config, DEFAULT_CONFIG_FILENAME};
pub use error::{FarolError, Result};
pub use files::{File, FileKind, FileTree};
pub use page::Page;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
