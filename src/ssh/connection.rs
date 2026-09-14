use anyhow::{Context, Result};
use russh::{
    client,
    keys::{
        load_secret_key,
        key::{PrivateKeyWithHashAlg},
        PublicKeyOrCertificate,
        PublicKey,
    },
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::config::ServerConfig;

fn resolve_project_path(path: &str) -> Result<PathBuf> {
    let configured_path = Path::new(path);

    if configured_path.is_absolute() {
        return Ok(configured_path.to_path_buf());
    }

    Ok(std::env::current_dir()
        .context("Failed to determine the project working directory")?
        .join(configured_path))
}

pub struct ClientHandler {
    pub target_host: String,
    pub target_port: u16,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate, // 👈 Updated parameter type
    ) -> Result<bool, Self::Error> {
        // Normalize certificates and plain keys into a PublicKey.
        let key = match server_public_key {
            PublicKeyOrCertificate::PublicKey { key, .. } => key.clone(),
            PublicKeyOrCertificate::Certificate(cert) => {
                PublicKey::new(cert.public_key().clone(), "")
            }
        };

        // Allow the trust store to live outside the disposable container.
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let known_hosts_path = std::env::var_os("LINUX_MCP_KNOWN_HOSTS")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(&home).join(".ssh/known_hosts"));

        if !known_hosts_path.exists() {
            eprintln!("Warning: known_hosts file not found at {:?}", known_hosts_path);
            return Ok(false);
        }

        // Verify key against known_hosts
        let is_valid = russh::keys::check_known_hosts_path(
            &self.target_host,
            self.target_port,
            &key,
            &known_hosts_path,
        )
           .map_err(|_| russh::Error::UnknownKey)?;

        Ok(is_valid)
    }
}

pub struct SshConnection {
    session: client::Handle<ClientHandler>,
}

impl SshConnection {
    pub async fn connect(server: &ServerConfig, username: &str) -> Result<Self> {
        let private_key_path = resolve_project_path(&server.private_key)?;

        let key = load_secret_key(&private_key_path, None)
        .with_context(|| {
            format!(
                "Failed to load SSH private key: {}",
                private_key_path.display()
            )
        })?;

        let config = client::Config::default();

        let handler = ClientHandler {
            target_host: server.hostname.clone(),
            target_port: server.port,
        };

        let mut session = client::connect(
            Arc::new(config),
            (server.hostname.as_str(), server.port),
            handler,
        )
        .await
        .with_context(|| {
            format!(
                "Failed to connect to {}:{}",
                server.hostname,
                server.port
            )
        })?;

        let auth_result = session
            .authenticate_publickey(
                username.to_string(),
                PrivateKeyWithHashAlg::new(
                    Arc::new(key),
                    session
                        .best_supported_rsa_hash()
                        .await
                        .context("Failed to determine supported RSA hash")?
                        .flatten(),
                ),
            )
            .await
            .context("SSH public-key authentication failed")?;

        if !auth_result.success() {
            anyhow::bail!(
                "SSH authentication failed for user '{}'",
                username
            );
        }

        Ok(Self { session })
    }

    pub async fn execute(&mut self, command: &str) -> Result<CommandResult> {
        let channel = self
            .session
            .channel_open_session()
            .await
            .context("Failed to open SSH session channel")?;

        let mut channel = channel;

        channel
            .exec(true, command)
            .await
            .with_context(|| {
                format!("Failed to execute command: {command}")
            })?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = None;

        while let Some(message) = channel.wait().await {
            match message {
                russh::ChannelMsg::Data { data } => {
                    stdout.extend_from_slice(&data);
                }

                russh::ChannelMsg::ExtendedData { data, .. } => {
                    stderr.extend_from_slice(&data);
                }

                russh::ChannelMsg::ExitStatus { exit_status } => {
                    exit_code = Some(exit_status);
                }

                // Eof can arrive before or after ExitStatus; keep draining
                // until the channel actually closes instead of stopping here.
                _ => {}
            }
        }

        Ok(CommandResult {
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            exit_code: exit_code.context("Remote command did not return an exit status")?,
        })
    }
}

#[derive(Debug)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u32,
}