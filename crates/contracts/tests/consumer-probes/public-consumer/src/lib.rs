// Browser-style dashboard consumer: uses only the `public` feature of the contracts crate.
//
// It is compiled for the host and for `wasm32-unknown-unknown` by `tests/consumer-probes`.
// Anything backend-only (Principal, credential material, service traits) is not importable
// here, which is the point of the probe.

use linux_mcp_contracts::prelude::*;
use linux_mcp_contracts::public::api::{ApprovalPage, ErrorEnvelope, ServerPage};
use linux_mcp_contracts::public::errors::ValidationError;
use linux_mcp_contracts::public::inventory::{
    ApprovalMode, DisplayName, Hostname, SshPort, SshUsername,
};

/// Parses a `GET /api/v1/servers` response body.
pub fn parse_server_page(body: &str) -> Result<ServerPage, serde_json::Error> {
    serde_json::from_str(body)
}

/// One table row for the servers page.
pub fn server_row(summary: &ServerSummary) -> String {
    format!(
        "{} | {}:{} | {:?} | rev {} | trust {:?}",
        summary.display_name,
        summary.hostname,
        summary.port.get(),
        summary.environment,
        summary.revision,
        summary.host_trust.status,
    )
}

/// Builds a `POST /api/v1/servers` body from form fields. Validation errors name fields only.
pub fn build_create_request(
    name: &str,
    hostname: &str,
    port: u16,
    username: &str,
) -> Result<String, ValidationError> {
    let field = |f: &str, e| ValidationError::from_invalid(f, e);
    let request = CreateServerRequest {
        display_name: DisplayName::new(name).map_err(|e| field("display_name", e))?,
        hostname: Hostname::new(hostname).map_err(|e| field("hostname", e))?,
        port: SshPort::new(port).map_err(|e| field("port", e))?,
        username: SshUsername::new(username).map_err(|e| field("username", e))?,
        environment: Environment::Development,
        enabled: true,
        approval_mode: ApprovalMode::default(),
    };
    Ok(serde_json::to_string(&request).expect("request serializes"))
}

/// Text for an error banner: status code and fixed message, never provider detail.
pub fn error_banner(body: &str) -> Result<(u16, String), serde_json::Error> {
    let envelope: ErrorEnvelope = serde_json::from_str(body)?;
    Ok((envelope.error.http_status(), envelope.error.message().to_owned()))
}

/// Text for the approvals page.
pub fn approval_rows(body: &str) -> Result<Vec<String>, serde_json::Error> {
    let page: ApprovalPage = serde_json::from_str(body)?;
    Ok(page
        .items
        .iter()
        .map(|a: &ApprovalSummary| {
            format!("{} {:?} {} risk={:?}", a.approval_id, a.status, a.intent.tool(), a.intent.risk())
        })
        .collect())
}

/// Chooses the stale-edit message from a conflict error.
pub fn is_revision_conflict(body: &str) -> bool {
    serde_json::from_str::<ErrorEnvelope>(body)
        .map(|e| e.error.code() == ErrorCode::RevisionConflict)
        .unwrap_or(false)
}

pub fn contract_versions() -> (&'static str, &'static str) {
    (linux_mcp_contracts::CONTRACT_VERSION, linux_mcp_contracts::CANONICALIZATION_VERSION)
}
