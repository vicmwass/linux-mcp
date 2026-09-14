use crate::commands::{Operation, RiskLevel};

use super::plan::ExecutionPayload;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub server_id: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u32,
    pub operations: Vec<Operation>,
    pub risk: RiskLevel,
}

impl ExecutionResult {
    pub fn from_command_result(
        server_id: String,
        result: crate::ssh::CommandResult,
        payload: &ExecutionPayload,
        operations: Vec<Operation>,
        risk: RiskLevel,
    ) -> Self {
        Self {
            server_id,
            success: result.exit_code == 0,
            stdout: result.stdout,
            stderr: result.stderr,
            exit_code: result.exit_code,
            operations,
            risk,
        }
    }

    pub fn payload_type(
        payload: &ExecutionPayload,
    ) -> &'static str {
        match payload {
            ExecutionPayload::Command(_) => "command",
            ExecutionPayload::Script { .. } => "script",
        }
    }
}