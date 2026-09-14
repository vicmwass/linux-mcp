use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub id: String,
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub private_key: String,
    pub environment: String,
}

#[derive(Debug, Deserialize)]
pub struct ServersConfig {
    pub servers: Vec<ServerConfig>,
}

impl ServersConfig {
    pub fn load(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration file: {path}"))?;

        let config: Self = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse configuration file: {path}"))?;

        Ok(config)
    }

    pub fn get_server(&self, id: &str) -> Option<&ServerConfig> {
        self.servers.iter().find(|server| server.id == id)
    }
}