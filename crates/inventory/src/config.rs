//! Typed configuration for the inventory database boundary.

use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryConfig {
    pub database_url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_acquire_timeout_seconds")]
    pub acquire_timeout_seconds: u64,
}

impl InventoryConfig {
    pub fn validate(&self) -> Result<(), InventoryConfigError> {
        if self.database_url.trim().is_empty() {
            return Err(InventoryConfigError::MissingDatabaseUrl);
        }
        if self.max_connections == 0 {
            return Err(InventoryConfigError::InvalidMaxConnections);
        }
        if self.acquire_timeout_seconds == 0 {
            return Err(InventoryConfigError::InvalidAcquireTimeout);
        }
        Ok(())
    }

    pub fn acquire_timeout(&self) -> Duration {
        Duration::from_secs(self.acquire_timeout_seconds)
    }
}

fn default_max_connections() -> u32 {
    10
}

fn default_acquire_timeout_seconds() -> u64 {
    5
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InventoryConfigError {
    #[error("database URL is required")]
    MissingDatabaseUrl,
    #[error("max_connections must be greater than zero")]
    InvalidMaxConnections,
    #[error("acquire_timeout_seconds must be greater than zero")]
    InvalidAcquireTimeout,
}
