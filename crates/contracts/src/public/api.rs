//! Management API (`/api/v1`) request/response contracts.
//!
//! Conventions (decisions.md D8, Spec 00 "Public API conventions"):
//! - JSON bodies; success bodies are the resource itself, errors are [`ErrorEnvelope`].
//!   Every response carries the `x-request-id` header; error bodies repeat it.
//! - The owner is always the authenticated principal. No request type has an owner field
//!   and all reject unknown fields.
//! - Mutations of a server require the last observed `expected_revision`; stale values return
//!   `409 revision_conflict` with the current revision.
//! - Foreign and absent IDs both return `404 not_found`.
//! - Credential inputs are write-only ([`WriteOnlySecret`]); no response type contains one.
//! - Host-key *observation* is not trust; only enrollment of the observed fingerprint after
//!   independent verification sets `verified`.
//! - Approval decisions require an interactive browser session with CSRF protection.
//!
//! The authoritative route table is [`ROUTES`]; JSON Schemas are generated into
//! `schemas/management/` and examples live in `fixtures/management/`.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::canonical::IntentDigest;
use crate::public::errors::{ErrorCode, InvalidValue, PublicError, ViolationRule};
use crate::public::ids::{
    ApprovalId, AuditRecordId, OperationId, RequestId, ServerId, ServerRevision, Timestamp,
    impl_json_schema,
};
use crate::public::inventory::{CredentialKind, CredentialStatus, Hostname, SshPort};
use crate::public::operations::{
    ApprovalStatus, OperationRequest, RefusalReason, SubmissionState, ToolId,
};
use crate::public::pagination::{PageCursor, PageLimit};

pub const API_BASE: &str = "/api/v1";
pub const REQUEST_ID_HEADER: &str = "x-request-id";
pub const CSRF_HEADER: &str = "x-csrf-token";

// ---------------------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------------------

/// Error response body: `{"error": {...}}`. HTTP status is `error.code.http_status()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ErrorEnvelope {
    pub error: PublicError,
}

// ---------------------------------------------------------------------------------------
// Write-only secrets and credentials
// ---------------------------------------------------------------------------------------

/// Secret submitted by a client. Serializable so browser code can send it, never part of a
/// response type, and redacted in `Debug`. Servers convert it to backend credential material
/// immediately (`into_material`, feature `backend`).
#[derive(PartialEq, Eq)]
pub struct WriteOnlySecret(String);

impl WriteOnlySecret {
    pub const MAX_BYTES: usize = 16 * 1024;

    pub fn new(value: String) -> Result<Self, InvalidValue> {
        if value.is_empty() {
            return Err(InvalidValue::new(ViolationRule::TooShort));
        }
        if value.len() > Self::MAX_BYTES {
            return Err(InvalidValue::new(ViolationRule::TooLong));
        }
        if value.contains('\0') {
            return Err(InvalidValue::new(ViolationRule::InvalidFormat));
        }
        Ok(Self(value))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Moves the plaintext into zeroizing backend material.
    #[cfg(feature = "backend")]
    pub fn into_material(
        self,
        kind: CredentialKind,
    ) -> crate::backend::credentials::CredentialMaterial {
        crate::backend::credentials::CredentialMaterial::new(kind, self.0.into_bytes())
    }
}

impl fmt::Debug for WriteOnlySecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WriteOnlySecret(<redacted>)")
    }
}

impl Serialize for WriteOnlySecret {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for WriteOnlySecret {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl_json_schema!(WriteOnlySecret, "WriteOnlySecret", {
    "type": "string",
    "writeOnly": true,
    "minLength": 1,
    "maxLength": 16384
});

/// Credential to provision or rotate.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CredentialInput {
    /// OpenSSH private key text. Passphrase-protected keys are not accepted in v1.
    SshPrivateKey {
        private_key: WriteOnlySecret,
    },
    SudoPassword {
        password: WriteOnlySecret,
    },
    /// Use `sudo -n`; removes any stored sudo password.
    SudoPasswordless,
}

impl CredentialInput {
    pub fn kind(&self) -> CredentialKind {
        match self {
            CredentialInput::SshPrivateKey { .. } => CredentialKind::SshPrivateKey,
            CredentialInput::SudoPassword { .. } | CredentialInput::SudoPasswordless => {
                CredentialKind::SudoPassword
            }
        }
    }
}

/// `PUT /api/v1/servers/{server_id}/credentials` — provision or rotate one credential slot.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PutCredentialRequest {
    pub expected_revision: ServerRevision,
    pub credential: CredentialInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CredentialWriteAction {
    Provisioned,
    Rotated,
    Removed,
}

/// Response to credential writes: status and references only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CredentialWriteResponse {
    pub server_id: ServerId,
    /// New server revision; credential changes invalidate pooled sessions and approvals.
    pub server_revision: ServerRevision,
    pub action: CredentialWriteAction,
    pub credential: CredentialStatus,
}

/// Query for `DELETE` routes guarded by a revision: `?expected_revision=N`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ExpectedRevisionQuery {
    pub expected_revision: ServerRevision,
}

// ---------------------------------------------------------------------------------------
// Host trust
// ---------------------------------------------------------------------------------------

/// `POST /api/v1/servers/{server_id}/host-key/observe` — no body.
///
/// The server connects only far enough to read the host key (no authentication, no command).
/// The result is **not** trusted; the user compares the fingerprint out of band.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HostKeyObservation {
    pub server_id: ServerId,
    pub server_revision: ServerRevision,
    pub hostname: Hostname,
    pub port: SshPort,
    /// SSH key algorithm, e.g. `ssh-ed25519`.
    pub key_type: String,
    /// `SHA256:<base64>` fingerprint to verify independently.
    pub fingerprint: String,
    pub observed_at: Timestamp,
    /// Whether this differs from a currently verified key (a changed key blocks connections).
    pub differs_from_verified: bool,
}

/// `POST /api/v1/servers/{server_id}/host-key/enroll`.
///
/// `fingerprint` must equal the most recent observation for the current revision; enrollment
/// bumps the revision and sets host trust to `verified`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EnrollHostKeyRequest {
    pub expected_revision: ServerRevision,
    pub fingerprint: String,
}

impl EnrollHostKeyRequest {
    /// `SHA256:` followed by 43 unpadded base64 characters (OpenSSH format).
    pub fn validate_fingerprint(&self) -> Result<(), InvalidValue> {
        let body = self
            .fingerprint
            .strip_prefix("SHA256:")
            .ok_or(InvalidValue::new(ViolationRule::InvalidFormat))?;
        let ok = body.len() == 43
            && body
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/');
        if ok {
            Ok(())
        } else {
            Err(InvalidValue::new(ViolationRule::InvalidFormat))
        }
    }
}

// ---------------------------------------------------------------------------------------
// Approvals
// ---------------------------------------------------------------------------------------

/// `POST /api/v1/approvals` — request approval for an operation. The server resolves the
/// target, evaluates policy and stores the immutable intent shown to the approver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CreateApprovalRequest {
    pub operation: OperationRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    Reject,
}

/// `POST /api/v1/approvals/{approval_id}/decision` — browser session + CSRF only.
///
/// `intent_digest` must equal the digest of the stored intent the human was shown, so a
/// decision cannot apply to a different intent than the one displayed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ApprovalDecisionRequest {
    pub decision: ApprovalDecision,
    pub intent_digest: IntentDigest,
}

/// `GET /api/v1/approvals?status=&cursor=&limit=`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ApprovalQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ApprovalStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<PageCursor>,
    #[serde(default)]
    pub limit: PageLimit,
}

// ---------------------------------------------------------------------------------------
// Audit
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    ServerCreated,
    ServerUpdated,
    ServerDeleted,
    CredentialWritten,
    CredentialRemoved,
    HostKeyObserved,
    HostKeyEnrolled,
    HostKeyRevoked,
    ApprovalRequested,
    ApprovalDecided,
    ApprovalConsumed,
    OperationRefused,
    OperationSubmissionPending,
    OperationSubmitted,
    OperationCompleted,
    OperationOutcomeUnknown,
    AuthorizationFailed,
}

/// Outcome status recorded in audit (output is not stored by default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcomeStatus {
    Succeeded,
    Refused,
    FailedBeforeSubmission,
    UnknownAfterSubmission,
}

/// One owner-scoped audit record.
///
/// For authorization failures against resources the caller does not own, `server_id`,
/// `server_revision` and `tool` are omitted so foreign metadata never appears.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AuditRecord {
    pub id: AuditRecordId,
    pub request_id: RequestId,
    pub action: AuditAction,
    pub operation_id: Option<OperationId>,
    pub server_id: Option<ServerId>,
    pub server_revision: Option<ServerRevision>,
    pub tool: Option<ToolId>,
    /// Arguments with sensitive values replaced by `"<redacted>"`.
    pub redacted_arguments: Option<serde_json::Value>,
    pub approval_id: Option<ApprovalId>,
    pub submission: Option<SubmissionState>,
    pub outcome: Option<AuditOutcomeStatus>,
    pub refusal_reason: Option<RefusalReason>,
    pub exit_code: Option<u32>,
    pub recorded_at: Timestamp,
}

/// `GET /api/v1/audit?server_id=&operation_id=&since=&until=&cursor=&limit=`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AuditQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<ServerId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<OperationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<PageCursor>,
    #[serde(default)]
    pub limit: PageLimit,
}

// ---------------------------------------------------------------------------------------
// Route table
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// How the caller must authenticate for a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteAuth {
    /// Browser session (with CSRF on unsafe methods) or delegated/MCP bearer token.
    SessionOrToken,
    /// Interactive browser session with CSRF token only (human decisions).
    InteractiveSessionWithCsrf,
}

/// One management route. Schema names refer to files in `schemas/management/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RouteSpec {
    pub id: &'static str,
    pub method: Method,
    pub path: &'static str,
    /// Any one of these scopes is sufficient (OAuth scope strings).
    pub scopes: &'static [&'static str],
    pub auth: RouteAuth,
    pub query: Option<&'static str>,
    pub request: Option<&'static str>,
    pub response: Option<&'static str>,
    pub success_status: u16,
    pub errors: &'static [ErrorCode],
}

use ErrorCode::{
    ApprovalRequired as E428, Forbidden as E403, NotFound as E404, OutcomeUnknown as E502,
    RevisionConflict as E409, Timeout as E504, Unauthenticated as E401, Unavailable as E503,
    Validation as E422,
};

const READ_ERRORS: &[ErrorCode] = &[E401, E403, E404, E422, E503];
const WRITE_ERRORS: &[ErrorCode] = &[E401, E403, E404, E409, E422, E503];
const NETWORK_ERRORS: &[ErrorCode] = &[E401, E403, E404, E409, E422, E503, E504];

/// Every management route. Routes not listed here do not exist in v1.
pub const ROUTES: &[RouteSpec] = &[
    RouteSpec {
        id: "list_servers",
        method: Method::Get,
        path: "/api/v1/servers",
        scopes: &["inventory:read"],
        auth: RouteAuth::SessionOrToken,
        query: Some("PageRequest"),
        request: None,
        response: Some("ServerPage"),
        success_status: 200,
        errors: &[E401, E403, E422, E503],
    },
    RouteSpec {
        id: "create_server",
        method: Method::Post,
        path: "/api/v1/servers",
        scopes: &["inventory:write"],
        auth: RouteAuth::SessionOrToken,
        query: None,
        request: Some("CreateServerRequest"),
        response: Some("ServerSummary"),
        success_status: 201,
        errors: &[E401, E403, E422, E503],
    },
    RouteSpec {
        id: "get_server",
        method: Method::Get,
        path: "/api/v1/servers/{server_id}",
        scopes: &["inventory:read"],
        auth: RouteAuth::SessionOrToken,
        query: None,
        request: None,
        response: Some("ServerSummary"),
        success_status: 200,
        errors: READ_ERRORS,
    },
    RouteSpec {
        id: "update_server",
        method: Method::Patch,
        path: "/api/v1/servers/{server_id}",
        scopes: &["inventory:write"],
        auth: RouteAuth::SessionOrToken,
        query: None,
        request: Some("UpdateServerRequest"),
        response: Some("ServerSummary"),
        success_status: 200,
        errors: WRITE_ERRORS,
    },
    RouteSpec {
        id: "delete_server",
        method: Method::Delete,
        path: "/api/v1/servers/{server_id}",
        scopes: &["inventory:write"],
        auth: RouteAuth::SessionOrToken,
        query: Some("ExpectedRevisionQuery"),
        request: None,
        response: None,
        success_status: 204,
        errors: WRITE_ERRORS,
    },
    RouteSpec {
        id: "put_credential",
        method: Method::Put,
        path: "/api/v1/servers/{server_id}/credentials",
        scopes: &["credentials:write"],
        auth: RouteAuth::SessionOrToken,
        query: None,
        request: Some("PutCredentialRequest"),
        response: Some("CredentialWriteResponse"),
        success_status: 200,
        errors: WRITE_ERRORS,
    },
    RouteSpec {
        id: "delete_credential",
        method: Method::Delete,
        path: "/api/v1/servers/{server_id}/credentials/{kind}",
        scopes: &["credentials:write"],
        auth: RouteAuth::SessionOrToken,
        query: Some("ExpectedRevisionQuery"),
        request: None,
        response: Some("CredentialWriteResponse"),
        success_status: 200,
        errors: WRITE_ERRORS,
    },
    RouteSpec {
        id: "observe_host_key",
        method: Method::Post,
        path: "/api/v1/servers/{server_id}/host-key/observe",
        scopes: &["inventory:write"],
        auth: RouteAuth::SessionOrToken,
        query: None,
        request: None,
        response: Some("HostKeyObservation"),
        success_status: 200,
        errors: NETWORK_ERRORS,
    },
    RouteSpec {
        id: "enroll_host_key",
        method: Method::Post,
        path: "/api/v1/servers/{server_id}/host-key/enroll",
        scopes: &["inventory:write"],
        auth: RouteAuth::SessionOrToken,
        query: None,
        request: Some("EnrollHostKeyRequest"),
        response: Some("ServerSummary"),
        success_status: 200,
        errors: WRITE_ERRORS,
    },
    RouteSpec {
        id: "revoke_host_key",
        method: Method::Delete,
        path: "/api/v1/servers/{server_id}/host-key",
        scopes: &["inventory:write"],
        auth: RouteAuth::SessionOrToken,
        query: Some("ExpectedRevisionQuery"),
        request: None,
        response: Some("ServerSummary"),
        success_status: 200,
        errors: WRITE_ERRORS,
    },
    RouteSpec {
        id: "create_approval",
        method: Method::Post,
        path: "/api/v1/approvals",
        scopes: &["approvals:request"],
        auth: RouteAuth::SessionOrToken,
        query: None,
        request: Some("CreateApprovalRequest"),
        response: Some("ApprovalSummary"),
        success_status: 201,
        errors: &[E401, E403, E404, E409, E422, E503],
    },
    RouteSpec {
        id: "list_approvals",
        method: Method::Get,
        path: "/api/v1/approvals",
        scopes: &["approvals:request", "approvals:decide"],
        auth: RouteAuth::SessionOrToken,
        query: Some("ApprovalQuery"),
        request: None,
        response: Some("ApprovalPage"),
        success_status: 200,
        errors: &[E401, E403, E422, E503],
    },
    RouteSpec {
        id: "get_approval",
        method: Method::Get,
        path: "/api/v1/approvals/{approval_id}",
        scopes: &["approvals:request", "approvals:decide"],
        auth: RouteAuth::SessionOrToken,
        query: None,
        request: None,
        response: Some("ApprovalSummary"),
        success_status: 200,
        errors: READ_ERRORS,
    },
    RouteSpec {
        id: "decide_approval",
        method: Method::Post,
        path: "/api/v1/approvals/{approval_id}/decision",
        scopes: &["approvals:decide"],
        auth: RouteAuth::InteractiveSessionWithCsrf,
        query: None,
        request: Some("ApprovalDecisionRequest"),
        response: Some("ApprovalSummary"),
        success_status: 200,
        errors: &[E401, E403, E404, E409, E422, E428, E503],
    },
    RouteSpec {
        id: "list_audit",
        method: Method::Get,
        path: "/api/v1/audit",
        scopes: &["audit:read"],
        auth: RouteAuth::SessionOrToken,
        query: Some("AuditQuery"),
        request: None,
        response: Some("AuditPage"),
        success_status: 200,
        errors: &[E401, E403, E422, E503],
    },
    RouteSpec {
        id: "get_audit_record",
        method: Method::Get,
        path: "/api/v1/audit/{audit_record_id}",
        scopes: &["audit:read"],
        auth: RouteAuth::SessionOrToken,
        query: None,
        request: None,
        response: Some("AuditRecord"),
        success_status: 200,
        errors: READ_ERRORS,
    },
];

/// Error codes the management API never returns (operation results carry these as outcomes).
pub const NON_MANAGEMENT_ERRORS: &[ErrorCode] = &[E502];

pub fn route(id: &str) -> Option<&'static RouteSpec> {
    ROUTES.iter().find(|r| r.id == id)
}

pub type ServerPage = crate::public::pagination::Page<crate::public::inventory::ServerSummary>;
pub type ApprovalPage = crate::public::pagination::Page<crate::public::operations::ApprovalSummary>;
pub type AuditPage = crate::public::pagination::Page<AuditRecord>;
