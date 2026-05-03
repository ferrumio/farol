//! farol-core: the engine.
//!
//! Forged in Rust. Lit for life.

pub mod config;
pub mod error;
pub mod files;
pub mod frontmatter;
pub mod scaffold;

pub use config::{Config, DEFAULT_CONFIG_FILENAME};
pub use error::{FarolError, Result};
pub use files::{File, FileKind, FileTree};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
