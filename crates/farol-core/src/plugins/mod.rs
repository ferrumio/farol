//! Plugin system.
//!
//! - [`host`] defines the [`PluginHost`] trait plus [`NoOpHost`] and
//!   [`ChainedHost`] for composing multiple hosts.
//! - [`core`] contains the builtin plugins that ship with farol; each one
//!   is an independent [`PluginHost`] implementation using the public API.

pub mod core;
pub mod host;

pub use host::{ChainedHost, NoOpHost, PluginHost};
