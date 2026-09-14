mod config;
mod mcp;
mod ssh;
mod security;
mod tools;
mod commands;
mod capabilities;
mod execution;

use anyhow::Result;
use rmcp::{
    service::ServiceExt,
    transport::stdio,
};

use config::ServersConfig;
use mcp::server::LinuxMcpServer;
use ssh::SshManager;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_writer(std::io::stderr)
        .init();

    let config = ServersConfig::load("config/servers.toml")?;

    let ssh_manager = SshManager::new(config);

    let server = LinuxMcpServer::new(ssh_manager);

    let service = server.serve(stdio()).await?;

    service.waiting().await?;

    Ok(())
}