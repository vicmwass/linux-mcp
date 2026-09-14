use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

use crate::config::{ServerConfig, ServersConfig};

use super::{CommandResult, SshConnection};

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub struct SshManager {
    servers: HashMap<String, ServerConfig>,
    connections: RwLock<HashMap<String, Arc<Mutex<SshConnection>>>>,
}
impl SshManager {
    pub fn new(config: ServersConfig) -> Self {
        let servers = config
            .servers
            .into_iter()
            .map(|server| (server.id.clone(), server))
            .collect();

        Self {
            servers,
            connections: RwLock::new(HashMap::new()),
        }
    }

    pub async fn connect(&self, server_id: &str) -> Result<()> {
        let server = self
            .servers
            .get(server_id)
            .ok_or_else(|| anyhow!("Server not found: {server_id}"))?;

        self.connect_as(server_id, &server.username).await
    }

    async fn connect_as(&self, server_id: &str, username: &str) -> Result<()> {
        let connection_key = Self::connection_key(server_id, username);

        // Check whether we already have a connection.
        {
            let connections = self.connections.read().await;

            if connections.contains_key(&connection_key) {
                return Ok(());
            }
        }

        // Get the server configuration.
        let server = self
            .servers
            .get(server_id)
            .ok_or_else(|| anyhow!("Server not found: {server_id}"))?;

        // Establish the SSH connection.
        let connection = SshConnection::connect(server, username).await?;

        // Store it.
        {
            let mut connections = self.connections.write().await;

            // Check again because another task may have connected
            // while we were establishing this connection.
            if connections.contains_key(&connection_key) {
                return Ok(());
            }

            connections.insert(connection_key, Arc::new(Mutex::new(connection)));
        }

        Ok(())
    }

    fn connection_key(server_id: &str, username: &str) -> String {
        format!("{server_id}::{username}")
    }

    pub async fn execute(
        &self,
        server_id: &str,
        command: &str,
        command_timeout: Duration,
    ) -> Result<CommandResult> {
        let server = self
            .servers
            .get(server_id)
            .ok_or_else(|| anyhow!("Server not found: {server_id}"))?;

        // A sudo command connects as root, over the same configured key.
        let username = if crate::commands::is_sudo_command(command) {
            "root"
        } else {
            server.username.as_str()
        };

        self.connect_as(server_id, username).await?;

        let connection_key = Self::connection_key(server_id, username);

        let connection = {
            let connections = self.connections.read().await;

            connections
                .get(&connection_key)
                .cloned()
                .ok_or_else(|| anyhow!("Connection not found: {connection_key}"))?
        };

        let mut connection = connection.lock().await;

        let result = timeout(command_timeout, connection.execute(command))
            .await
            .map_err(|_| anyhow!("Command execution timed out after {:?}", command_timeout))??;

        Ok(result)
    }

    pub fn get_server(&self, server_id: &str) -> Option<&ServerConfig> {
        self.servers.get(server_id)
    }

    pub fn list_servers(&self) -> Vec<ServerInfo> {
    self.servers
        .values()
        .map(|server| ServerInfo {
            id: server.id.clone(),
            hostname: server.hostname.clone(),
            port: server.port,
            username: server.username.clone(),
            environment: server.environment.clone(),
        })
        .collect()
}

    pub fn is_connected(&self, server_id: &str) -> bool {
        let prefix = format!("{server_id}::");

        self.connections
            .try_read()
            .map(|connections| connections.keys().any(|key| key.starts_with(&prefix)))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub id: String,
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub environment: String,
}