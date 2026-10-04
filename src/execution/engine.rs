use anyhow::Result;

use super::limits::ExecutionLimits;
use crate::execution::{ExecutionAnalysis, result::ExecutionResult};
use crate::ssh::SshManager;

pub struct ExecutionEngine<'a> {
    ssh_manager: &'a SshManager,
    limits: ExecutionLimits,
}

impl<'a> ExecutionEngine<'a> {
    pub fn new(ssh_manager: &'a SshManager) -> Self {
        Self {
            ssh_manager,
            limits: ExecutionLimits::default(),
        }
    }

    pub async fn execute_structured(
        &self,
        server_id: &str,
        command: &str,
        analysis: &ExecutionAnalysis,
        sudo: bool,
    ) -> Result<ExecutionResult> {
        let result = if sudo {
            self.ssh_manager
                .execute_sudo(server_id, command, self.limits.timeout)
                .await?
        } else {
            self.ssh_manager
                .execute(server_id, command, self.limits.timeout)
                .await?
        };
        Ok(ExecutionResult {
            server_id: server_id.to_owned(),
            success: result.exit_code == 0,
            stdout: result.stdout,
            stderr: result.stderr,
            exit_code: result.exit_code,
            operations: analysis.operations.clone(),
            risk: analysis.risk.clone(),
        })
    }
}
