//! Minimal MCP-adapter consumer used by the probes and by `examples/mcp_adapter.rs`.
//!
//! It is a mock consumer, not a production adapter: it shows how an MCP service (Spec 04)
//! maps tool calls onto the shared contracts and returns the `linux-mcp/v1` envelope. It
//! never interprets `confirm` as approval and never executes without the Driver's sequence.

use linux_mcp_contracts::backend::services::RequestContext;
use linux_mcp_contracts::public::errors::{PublicError, ValidationError, ViolationRule};
use linux_mcp_contracts::public::ids::ServerId;
use linux_mcp_contracts::public::mcp::{
    ApprovalRequiredResult, ToolResultBody, ToolResultEnvelope, legacy_tool,
};
use linux_mcp_contracts::public::operations::{
    CanonicalArguments, OperationOutcome, OperationRequest, ToolSchemaVersion,
};
use linux_mcp_contracts::test_support::memory::block_on;
use linux_mcp_contracts::test_support::scenarios::{Driven, Driver};
use serde_json::{Map, Value};

/// What an MCP server sends back.
#[derive(Debug)]
pub enum Reply {
    /// A tool result (MCP `result`), with the MCP `isError` flag.
    Result {
        envelope: ToolResultEnvelope,
        is_error: bool,
    },
    /// A JSON-RPC protocol error (unknown tool), distinct from tool results.
    ProtocolError { code: i64, message: &'static str },
}

pub struct McpAdapter {
    pub driver: Driver,
    pub ctx: RequestContext,
}

fn invalid(ctx: &RequestContext, field: &str) -> PublicError {
    PublicError::validation(
        ctx.request_id(),
        ValidationError::single(field, ViolationRule::Required),
    )
}

impl McpAdapter {
    pub fn call(&self, tool: &str, arguments: Value) -> Reply {
        let envelope = |body| ToolResultEnvelope::new(self.ctx.request_id(), body);
        let Some(spec) = legacy_tool(tool) else {
            return Reply::ProtocolError {
                code: -32602,
                message: "tool not found",
            };
        };
        let error = |error: PublicError| Reply::Result {
            envelope: envelope(ToolResultBody::Error { error }),
            is_error: true,
        };
        let Some(args) = arguments.as_object() else {
            return error(invalid(&self.ctx, "arguments"));
        };
        if let Some(missing) = spec.required.iter().find(|r| !args.contains_key(**r)) {
            return error(invalid(&self.ctx, missing));
        }
        let Some(server_id) = args
            .get("server_id")
            .and_then(Value::as_str)
            .and_then(|s| ServerId::parse(s).ok())
        else {
            return error(invalid(&self.ctx, "server_id"));
        };
        // `server_id` becomes the target; everything else is the canonical argument object.
        // `confirm` stays an ordinary argument: intent only, never approval evidence.
        let rest: Map<String, Value> = args
            .iter()
            .filter(|(k, _)| k.as_str() != "server_id")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let Ok(arguments) = CanonicalArguments::new(Value::Object(rest)) else {
            return error(invalid(&self.ctx, "arguments"));
        };
        let request = OperationRequest {
            server_id,
            tool: spec.tool_id(),
            tool_schema_version: ToolSchemaVersion::V1,
            arguments,
        };
        match block_on(self.driver.run(&self.ctx, request)) {
            Err(e) => error(e),
            Ok(Driven::ApprovalRequired(approval)) => Reply::Result {
                envelope: envelope(ToolResultBody::ApprovalRequired(ApprovalRequiredResult {
                    approval_id: approval.approval_id,
                    status: approval.status,
                    intent_digest: approval.intent_digest.clone(),
                    expires_at: approval.expires_at,
                    dashboard_path: format!("/approvals/{}", approval.approval_id),
                })),
                is_error: false,
            },
            Ok(Driven::Finished(result)) => {
                let is_error = !matches!(result.outcome, OperationOutcome::Succeeded { .. });
                Reply::Result {
                    envelope: envelope(ToolResultBody::Outcome {
                        operation_id: result.operation_id,
                        server_id: result.server_id,
                        submission: result.submission,
                        outcome: result.outcome.clone(),
                    }),
                    is_error,
                }
            }
        }
    }
}

/// Short human-readable text accompanying the structured result.
pub fn render_text(reply: &Reply) -> String {
    match reply {
        Reply::ProtocolError { message, .. } => format!("protocol error: {message}"),
        Reply::Result { envelope, .. } => match &envelope.result {
            ToolResultBody::ApprovalRequired(a) => {
                format!("Approval required: open {} to decide.", a.dashboard_path)
            }
            ToolResultBody::Error { error } => {
                format!("Error ({}): {}", error.code(), error.message())
            }
            ToolResultBody::Outcome { outcome, .. } => match outcome {
                OperationOutcome::Succeeded { exit_status, .. } => {
                    format!("Completed: {exit_status:?}")
                }
                OperationOutcome::Refused { reason, .. } => format!("Refused: {reason:?}"),
                OperationOutcome::FailedBeforeSubmission { cause } => {
                    format!("Not executed: {cause:?}")
                }
                OperationOutcome::UnknownAfterSubmission { cause, .. } => {
                    format!("Outcome unknown ({cause:?}); it may have run and will not be retried.")
                }
            },
        },
    }
}
