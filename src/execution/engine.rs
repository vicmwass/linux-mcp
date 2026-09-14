use anyhow::{bail, Result};

use crate::{
    commands::ensure_non_interactive_sudo,
    ssh::SshManager,
};
use crate::execution::result::ExecutionResult;

use super::plan::{ExecutionPayload, ExecutionPlan};

use super::limits::ExecutionLimits;

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

    pub async fn execute(
        &self,
        plan: &ExecutionPlan,
    ) -> Result<ExecutionResult> {
        let command = match &plan.payload {
            ExecutionPayload::Command(command) => {
                ensure_non_interactive_sudo(command)
            }

            ExecutionPayload::Script {
                interpreter,
                content,
            } => {
                self.build_script_command(
                    interpreter,
                    content,
                )?
            }
        };

        let result = self
        .ssh_manager
        .execute(
            &plan.server_id,
            &command,
            self.limits.timeout,
        )
        .await?;

        Ok(ExecutionResult {
        server_id: plan.server_id.clone(),
        success: result.exit_code == 0,
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
        operations: plan.analysis.operations.clone(),
        risk: plan.analysis.risk.clone(),
    })
    }

    /// Execute a structured inspection command as the configured non-root user.
    /// The caller is responsible for validating and shell-quoting each argv entry.
    pub async fn execute_inspection(
        &self,
        server_id: &str,
        command: &str,
    ) -> Result<ExecutionResult> {
        let result = self.ssh_manager.execute(
            server_id,
            command,
            self.limits.timeout,
        ).await?;

        Ok(ExecutionResult {
            server_id: server_id.to_string(),
            success: result.exit_code == 0,
            stdout: result.stdout,
            stderr: result.stderr,
            exit_code: result.exit_code,
            operations: vec![crate::commands::Operation::ReadOnly],
            risk: crate::commands::RiskLevel::Low,
        })
    }

    fn build_script_command(
        &self,
        interpreter: &str,
        content: &str,
    ) -> Result<String> {
        // Script content is not sudo-normalized; no MCP tool exposes scripts yet.
        match interpreter {
            "bash" => {
                Ok(format!(
                    "bash -c {}",
                    shell_escape(content)
                ))
            }

            "sh" => {
                Ok(format!(
                    "sh -c {}",
                    shell_escape(content)
                ))
            }

            "python3" => {
                Ok(format!(
                    "python3 -c {}",
                    shell_escape(content)
                ))
            }

            "python" => {
                Ok(format!(
                    "python -c {}",
                    shell_escape(content)
                ))
            }

            _ => {
                bail!(
                    "Unsupported script interpreter: {}",
                    interpreter
                );
            }
        }
    }
}

fn shell_escape(value: &str) -> String {
    format!(
        "'{}'",
        value.replace('\'', "'\\''")
    )
}
