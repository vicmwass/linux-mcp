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
        self.execute_with_stdin(server_id, command, &[], command_timeout)
            .await
    }

    pub async fn execute_sudo(
        &self,
        server_id: &str,
        command: &str,
        command_timeout: Duration,
    ) -> Result<CommandResult> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| anyhow!("Server not found: {server_id}"))?;
        let (command, mut stdin) =
            super::sudo::prepare(command, server.sudo_password_file.as_deref())?;
        let result = self
            .execute_with_stdin(server_id, &command, &stdin, command_timeout)
            .await;
        stdin.fill(0);
        result
    }

    pub async fn execute_with_stdin(
        &self,
        server_id: &str,
        command: &str,
        stdin: &[u8],
        command_timeout: Duration,
    ) -> Result<CommandResult> {
        let server = self
            .get_server(server_id)
            .ok_or_else(|| anyhow!("Server not found: {server_id}"))?;
        let key = Self::connection_key(server_id, &server.username);
        // Bound connection, queueing and execution together. Never replay a command
        // after exec was submitted: timeout/disconnect leaves its outcome uncertain.
        let operation = async {
            for attempt in 0..2 {
                self.connect_as(server_id, &server.username).await?;
                let connection = self
                    .connections
                    .read()
                    .await
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| anyhow!("Connection not found: {key}"))?;
                let mut session = connection.lock().await;
                let was_closed = session.is_closed();
                let result = if was_closed {
                    Err(anyhow!("Cached SSH connection is closed"))
                } else {
                    session.execute_with_stdin(command, stdin).await
                };
                let retry_safe = was_closed
                    || result.as_ref().err().is_some_and(|e| {
                        e.downcast_ref::<super::connection::ChannelOpenError>()
                            .is_some()
                    });
                if result.is_err() {
                    let mut connections = self.connections.write().await;
                    if connections
                        .get(&key)
                        .is_some_and(|c| Arc::ptr_eq(c, &connection))
                    {
                        connections.remove(&key);
                    }
                }
                drop(session);
                if attempt == 0 && retry_safe {
                    continue;
                }
                return result;
            }
            unreachable!()
        };
        match timeout(command_timeout, operation).await {
            Ok(result) => result,
            Err(_) => {
                self.connections.write().await.remove(&key);
                Err(anyhow!(
                    "Command execution timed out after {:?}; remote outcome may be unknown",
                    command_timeout
                ))
            }
        }
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
