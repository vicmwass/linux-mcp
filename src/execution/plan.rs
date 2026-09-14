use crate::commands::{Operation, RiskLevel};

use super::analyzer::ExecutionAnalysis;

#[derive(Debug, Clone)]
pub enum ExecutionPayload {
    Command(String),

    Script {
        interpreter: String,
        content: String,
    },
}

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub server_id: String,

    pub payload: ExecutionPayload,

    pub analysis: ExecutionAnalysis,

    pub confirmed: bool,
}

impl ExecutionPlan {
    pub fn command(
        server_id: String,
        command: String,
        analysis: ExecutionAnalysis,
        confirmed: bool,
    ) -> Self {
        Self {
            server_id,
            payload: ExecutionPayload::Command(command),
            analysis,
            confirmed,
        }
    }

    pub fn script(
        server_id: String,
        interpreter: String,
        content: String,
        analysis: ExecutionAnalysis,
        confirmed: bool,
    ) -> Self {
        Self {
            server_id,
            payload: ExecutionPayload::Script {
                interpreter,
                content,
            },
            analysis,
            confirmed,
        }
    }

    pub fn operations(&self) -> &[Operation] {
        &self.analysis.operations
    }

    pub fn risk(&self) -> &RiskLevel {
        &self.analysis.risk
    }
}