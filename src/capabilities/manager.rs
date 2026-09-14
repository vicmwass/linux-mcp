use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::sync::RwLock;

use crate::ssh::SshManager;

use super::{detect, ServerCapabilities};

pub struct CapabilityManager {
    ssh_manager: Arc<SshManager>,
    capabilities: RwLock<HashMap<String, ServerCapabilities>>,
}

impl CapabilityManager {
    pub fn new(ssh_manager: Arc<SshManager>) -> Self {
        Self {
            ssh_manager,
            capabilities: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get(
        &self,
        server_id: &str,
    ) -> Result<ServerCapabilities> {
        {
            let capabilities = self.capabilities.read().await;

            if let Some(cached) = capabilities.get(server_id) {
                return Ok(cached.clone());
            }
        }

        let detected =
            detect(
                &self.ssh_manager,
                server_id,
            )
            .await?;

        let mut capabilities =
            self.capabilities.write().await;

        capabilities.insert(
            server_id.to_string(),
            detected.clone(),
        );

        Ok(detected)
    }

    pub async fn refresh(
        &self,
        server_id: &str,
    ) -> Result<ServerCapabilities> {
        let detected =
            detect(
                &self.ssh_manager,
                server_id,
            )
            .await?;

        let mut capabilities =
            self.capabilities.write().await;

        capabilities.insert(
            server_id.to_string(),
            detected.clone(),
        );

        Ok(detected)
    }

    pub async fn invalidate(
        &self,
        server_id: &str,
    ) -> Result<()> {
        let mut capabilities =
            self.capabilities.write().await;

        if capabilities.remove(server_id).is_none() {
            return Err(anyhow!(
                "No cached capabilities found for server '{}'",
                server_id
            ));
        }

        Ok(())
    }
}