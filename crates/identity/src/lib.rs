//! Installation-scoped identity persistence and test-issuer boundary.
//!
//! Browser/MCP protocol validation is implemented in later steps. This crate
//! deliberately keeps test issuer material behind an explicit test profile.

#![forbid(unsafe_code)]

pub mod config;
pub mod migrations;
pub mod test_issuer;

pub use config::{IdentityConfig, IdentityConfigError};
pub use migrations::run_migrations;
