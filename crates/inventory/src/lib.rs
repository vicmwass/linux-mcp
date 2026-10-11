//! Spec 01 inventory persistence bootstrap.
//!
//! This crate owns PostgreSQL schema and owner-scoped inventory boundaries. It does
//! not expose SSH execution, arbitrary secret paths, or authentication adapters.

#![forbid(unsafe_code)]

pub mod config;
pub mod credentials;
pub mod destinations;
pub mod migrations;
pub mod repository;
pub mod resolver;
pub mod trust;

pub use config::{InventoryConfig, InventoryConfigError};
pub use migrations::run_migrations;
