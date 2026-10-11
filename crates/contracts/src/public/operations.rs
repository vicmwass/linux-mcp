//! Operation intent, approval and outcome DTOs (decisions.md D9, D10).
//!
//! Layers are separate types so that none can stand in for another:
//!
//! 1. [`OperationRequest`] — what a caller asks for. No owner, no approval, no authority.
//! 2. [`OperationIntent`] — the server-created, immutable binding of identity, installation,
//!    server/revision, tool + schema version, canonical arguments, risk and expiry, with its
//!    [`IntentDigest`]. Shown to the human approver instead of agent-provided text.
//! 3. [`PolicyVerdict`] — preflight result (`backend::operation_authority::PolicyEvaluation`).
//! 4. Approval status ([`ApprovalStatus`], [`ApprovalSummary`]) — human decision, public view.
//! 5. `backend::operation_authority::ExecutionAuthority` — backend-only, the only input
//!    accepted by `Executor::submit`. Nothing in this module can create it.
//!
//! Outcomes distinguish *refused*, *failed before submission*, *succeeded* (remote command
//! completed; its exit status is reported separately) and *unknown after submission*.
//! [`OperationOutcome::transport_failure`] makes the submission boundary explicit so a
//! disconnect after submission can never be reported as a definite failure.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::canonical::{self, CANONICALIZATION_VERSION, CanonicalError, IntentDigest};
use crate::public::errors::{InvalidValue, ViolationRule};
use crate::public::ids::{
    ApprovalId, InstallationId, OperationId, ServerId, ServerRevision, Timestamp, UserId,
    impl_json_schema,
};

/// Default per-stream output bound (matches the current `ExecutionLimits`).
pub const DEFAULT_OUTPUT_LIMIT_BYTES: usize = 1024 * 1024;
/// Longest allowed intent lifetime.
pub const MAX_INTENT_LIFETIME_MILLIS: u64 = 24 * 60 * 60 * 1000;

/// Tool name, `^[a-z][a-z0-9_]{0,63}$` (e.g. `execute_command`, `delete_user`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ToolId(String);

impl ToolId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidValue> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 64
            && value.as_bytes()[0].is_ascii_lowercase()
            && value
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if valid {
            Ok(Self(value))
        } else {
            Err(InvalidValue::new(ViolationRule::InvalidFormat))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ToolId {
    type Error = InvalidValue;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ToolId> for String {
    fn from(value: ToolId) -> Self {
        value.0
    }
}

impl std::fmt::Display for ToolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl_json_schema!(ToolId, "ToolId", {
    "type": "string",
    "pattern": "^[a-z][a-z0-9_]{0,63}$"
});

/// Version of a tool's argument schema; bumping it invalidates approvals for older shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct ToolSchemaVersion(u32);

impl ToolSchemaVersion {
    pub const V1: ToolSchemaVersion = ToolSchemaVersion(1);

    pub fn new(value: u32) -> Result<Self, InvalidValue> {
        if value == 0 {
            Err(InvalidValue::new(ViolationRule::OutOfRange))
        } else {
            Ok(Self(value))
        }
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for ToolSchemaVersion {
    type Error = InvalidValue;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ToolSchemaVersion> for u32 {
    fn from(value: ToolSchemaVersion) -> Self {
        value.0
    }
}

impl_json_schema!(ToolSchemaVersion, "ToolSchemaVersion", {
    "type": "integer",
    "minimum": 1
});

/// Risk classification, same levels as the current command classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Tool arguments in canonical form. Always a JSON object; validated on construction and
/// on deserialization, so an instance is always canonicalizable and secret-free by key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Value", into = "Value")]
pub struct CanonicalArguments {
    value: Value,
    canonical: String,
}

impl CanonicalArguments {
    pub fn new(value: Value) -> Result<Self, CanonicalError> {
        if !value.is_object() {
            return Err(CanonicalError::NotAnObject);
        }
        let canonical = canonical::canonicalize(&value)?;
        let value = canonical::normalize(&value)?;
        Ok(Self { value, canonical })
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn canonical(&self) -> &str {
        &self.canonical
    }
}

impl TryFrom<Value> for CanonicalArguments {
    type Error = CanonicalError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CanonicalArguments> for Value {
    fn from(value: CanonicalArguments) -> Self {
        value.value
    }
}

impl_json_schema!(CanonicalArguments, "CanonicalArguments", {
    "type": "object",
    "description": "Tool arguments; canonicalized with lmcp-canon-v1 (no floats, no secret-named keys)."
});

/// What a caller asks to run. Identity comes from the authenticated context, never from here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct OperationRequest {
    pub server_id: ServerId,
    pub tool: ToolId,
    pub tool_schema_version: ToolSchemaVersion,
    pub arguments: CanonicalArguments,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IntentError {
    #[error("intent must expire after it is created and within the maximum lifetime")]
    InvalidLifetime,
    #[error("unsupported canonicalization version {0}")]
    UnsupportedCanonicalization(String),
    #[error(transparent)]
    Canonical(#[from] CanonicalError),
}

/// Immutable, server-created operation binding (spec 00 `OperationIntent`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct OperationIntent {
    operation_id: OperationId,
    canonicalization: String,
    installation_id: InstallationId,
    owner: UserId,
    server_id: ServerId,
    server_revision: ServerRevision,
    tool: ToolId,
    tool_schema_version: ToolSchemaVersion,
    arguments: CanonicalArguments,
    risk: RiskLevel,
    created_at: Timestamp,
    expires_at: Timestamp,
}

/// Fields required to create an intent. Built by the execution/governance backend from the
/// trusted principal and resolved target; `owner` is never copied from a request body.
#[derive(Debug, Clone)]
pub struct IntentParts {
    pub operation_id: OperationId,
    pub installation_id: InstallationId,
    pub owner: UserId,
    pub server_id: ServerId,
    pub server_revision: ServerRevision,
    pub tool: ToolId,
    pub tool_schema_version: ToolSchemaVersion,
    pub arguments: CanonicalArguments,
    pub risk: RiskLevel,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}

impl OperationIntent {
    pub fn new(parts: IntentParts) -> Result<Self, IntentError> {
        let lifetime_ok = parts.expires_at > parts.created_at
            && parts
                .created_at
                .checked_add_millis(MAX_INTENT_LIFETIME_MILLIS)
                .is_some_and(|max| parts.expires_at <= max);
        if !lifetime_ok {
            return Err(IntentError::InvalidLifetime);
        }
        Ok(Self {
            operation_id: parts.operation_id,
            canonicalization: CANONICALIZATION_VERSION.to_owned(),
            installation_id: parts.installation_id,
            owner: parts.owner,
            server_id: parts.server_id,
            server_revision: parts.server_revision,
            tool: parts.tool,
            tool_schema_version: parts.tool_schema_version,
            arguments: parts.arguments,
            risk: parts.risk,
            created_at: parts.created_at,
            expires_at: parts.expires_at,
        })
    }

    /// Rejects intents produced under a canonicalization version this build does not know.
    pub fn check_version(&self) -> Result<(), IntentError> {
        if self.canonicalization == CANONICALIZATION_VERSION {
            Ok(())
        } else {
            Err(IntentError::UnsupportedCanonicalization(
                self.canonicalization.clone(),
            ))
        }
    }

    /// Canonical text of the approval binding: everything except `operation_id` and
    /// `created_at`, which identify the request rather than what is approved.
    pub fn binding_canonical(&self) -> String {
        let binding = serde_json::json!({
            "canonicalization": self.canonicalization,
            "installation_id": self.installation_id,
            "owner": self.owner,
            "server_id": self.server_id,
            "server_revision": self.server_revision,
            "tool": self.tool,
            "tool_schema_version": self.tool_schema_version,
            "arguments": self.arguments.value(),
            "risk": self.risk,
            "expires_at": self.expires_at,
        });
        canonical::canonicalize(&binding).expect("validated intent canonicalizes")
    }

    pub fn digest(&self) -> IntentDigest {
        IntentDigest::of_canonical(&self.binding_canonical())
    }

    pub fn is_expired_at(&self, now: Timestamp) -> bool {
        now >= self.expires_at
    }

    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }
    pub fn canonicalization(&self) -> &str {
        &self.canonicalization
    }
    pub fn installation_id(&self) -> InstallationId {
        self.installation_id
    }
    pub fn owner(&self) -> UserId {
        self.owner
    }
    pub fn server_id(&self) -> ServerId {
        self.server_id
    }
    pub fn server_revision(&self) -> ServerRevision {
        self.server_revision
    }
    pub fn tool(&self) -> &ToolId {
        &self.tool
    }
    pub fn tool_schema_version(&self) -> ToolSchemaVersion {
        self.tool_schema_version
    }
    pub fn arguments(&self) -> &CanonicalArguments {
        &self.arguments
    }
    pub fn risk(&self) -> RiskLevel {
        self.risk
    }
    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }
    pub fn expires_at(&self) -> Timestamp {
        self.expires_at
    }
}

/// Why an operation was refused. A refusal is a normal result, not a transport error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RefusalReason {
    /// Security policy denied the operation; approval cannot override this.
    PolicyDenied,
    /// The command/tool could not be classified or uses unsupported constructs.
    UnsupportedOperation,
    /// The target lacks a required capability (e.g. no package manager detected).
    CapabilityMissing,
    /// Human approval is required and no valid consumed approval was presented.
    ApprovalRequired,
    /// The presented approval is expired, consumed, rejected or bound to different inputs.
    ApprovalInvalid,
    /// Legacy stdio only: the tool's own `confirm` flag was required but not set.
    ConfirmationRequired,
    /// Target disabled, host key unverified, or credentials missing.
    TargetUnavailable,
    /// The server revision changed after the intent was created.
    RevisionChanged,
    /// Tool-level argument validation failed (e.g. invalid username).
    InvalidArguments,
}

/// Preflight verdict for an intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "verdict", rename_all = "snake_case", deny_unknown_fields)]
pub enum PolicyVerdict {
    Allowed,
    RequiresApproval,
    Denied { reason: RefusalReason },
}

/// Human approval state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    /// Consumed by exactly one dispatch; never reset, even if the outcome is unknown.
    Consumed,
    /// Target revision, identity or permissions changed after approval.
    Invalidated,
}

impl ApprovalStatus {
    pub fn is_terminal(self) -> bool {
        !matches!(self, ApprovalStatus::Pending | ApprovalStatus::Approved)
    }
}

/// Public view of an approval, including the stored intent the human approves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ApprovalSummary {
    pub approval_id: ApprovalId,
    pub status: ApprovalStatus,
    pub intent: OperationIntent,
    pub intent_digest: IntentDigest,
    pub requested_at: Timestamp,
    pub decided_at: Option<Timestamp>,
    pub expires_at: Timestamp,
}

/// Where an operation is relative to the submission boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SubmissionState {
    /// Nothing was sent to the remote host.
    NotSubmitted,
    /// A durable pending record exists; the exec request may or may not have been sent.
    SubmissionPending,
    /// The exec request was sent to the remote host.
    Submitted,
    /// The remote command completed and reported a status.
    Completed,
}

impl SubmissionState {
    /// After this point the command may have run, so a lost result is `unknown`, never failed.
    pub fn may_have_run(self) -> bool {
        !matches!(self, SubmissionState::NotSubmitted)
    }
}

/// Remote exit status, independent of transport/policy status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExitStatus {
    Code { code: u32 },
    Signal { signal: String },
}

impl ExitStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, ExitStatus::Code { code: 0 })
    }
}

/// Output stream bounded to a byte limit, truncated on a UTF-8 boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct BoundedOutput {
    pub text: String,
    pub truncated: bool,
    /// Byte length of the original stream before truncation.
    pub original_bytes: u64,
    /// Invalid UTF-8 sequences were replaced with U+FFFD.
    pub lossy: bool,
}

impl BoundedOutput {
    pub fn empty() -> Self {
        Self {
            text: String::new(),
            truncated: false,
            original_bytes: 0,
            lossy: false,
        }
    }

    pub fn from_bytes(bytes: &[u8], limit: usize) -> Self {
        let decoded = String::from_utf8_lossy(bytes);
        let lossy = matches!(decoded, std::borrow::Cow::Owned(_));
        let mut text = decoded.into_owned();
        let truncated = text.len() > limit;
        if truncated {
            let mut cut = limit;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
        }
        Self {
            text,
            truncated,
            original_bytes: bytes.len() as u64,
            lossy,
        }
    }
}

/// Post-change verification, reported separately from the exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Verification {
    NotAttempted,
    Verified,
    Mismatch,
    Unknown,
}

/// Failure that happened before anything was sent to the remote host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PreSubmissionFailure {
    Timeout,
    Cancelled,
    ConnectionFailed,
    HostKeyRejected,
    AuthenticationFailed,
    CredentialUnavailable,
    AuditUnavailable,
    Unavailable,
}

/// Why the outcome of a submitted operation is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum UnknownCause {
    Timeout,
    Disconnected,
    Cancelled,
    /// Recovery found a pending dispatch without a recorded result.
    Unreconciled,
}

/// Transport-level failure cause before the submission state is taken into account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportFailure {
    Timeout,
    Cancelled,
    Disconnected,
    ConnectionFailed,
}

/// Spec 00 `OperationOutcome`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationOutcome {
    /// The remote command ran to completion. A non-zero exit status is still `succeeded`
    /// at this level; inspect `exit_status`.
    Succeeded {
        exit_status: ExitStatus,
        stdout: BoundedOutput,
        stderr: BoundedOutput,
        verification: Verification,
    },
    /// Not executed because policy, approval or validation refused it.
    Refused {
        reason: RefusalReason,
        /// Short policy-generated explanation; never contains secrets or foreign metadata.
        detail: Option<String>,
    },
    /// Nothing was submitted to the remote host.
    FailedBeforeSubmission { cause: PreSubmissionFailure },
    /// The command may have run; it will not be replayed automatically.
    UnknownAfterSubmission {
        cause: UnknownCause,
        partial_stdout: Option<BoundedOutput>,
        partial_stderr: Option<BoundedOutput>,
    },
}

impl OperationOutcome {
    pub const MAX_DETAIL_CHARS: usize = 512;

    pub fn refused(reason: RefusalReason, detail: Option<&str>) -> Self {
        OperationOutcome::Refused {
            reason,
            detail: detail.map(|d| d.chars().take(Self::MAX_DETAIL_CHARS).collect()),
        }
    }

    /// Maps a transport failure using the submission boundary. Once the state may have run,
    /// the result is always `unknown_after_submission`.
    pub fn transport_failure(state: SubmissionState, failure: TransportFailure) -> Self {
        if state.may_have_run() {
            let cause = match failure {
                TransportFailure::Timeout => UnknownCause::Timeout,
                TransportFailure::Cancelled => UnknownCause::Cancelled,
                TransportFailure::Disconnected | TransportFailure::ConnectionFailed => {
                    UnknownCause::Disconnected
                }
            };
            OperationOutcome::UnknownAfterSubmission {
                cause,
                partial_stdout: None,
                partial_stderr: None,
            }
        } else {
            let cause = match failure {
                TransportFailure::Timeout => PreSubmissionFailure::Timeout,
                TransportFailure::Cancelled => PreSubmissionFailure::Cancelled,
                TransportFailure::Disconnected | TransportFailure::ConnectionFailed => {
                    PreSubmissionFailure::ConnectionFailed
                }
            };
            OperationOutcome::FailedBeforeSubmission { cause }
        }
    }

    /// Whether the remote host may have executed the command.
    pub fn may_have_run(&self) -> bool {
        matches!(
            self,
            OperationOutcome::Succeeded { .. } | OperationOutcome::UnknownAfterSubmission { .. }
        )
    }
}

/// Complete result record for one operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct OperationResult {
    pub operation_id: OperationId,
    pub server_id: ServerId,
    pub server_revision: ServerRevision,
    pub tool: ToolId,
    pub submission: SubmissionState,
    pub outcome: OperationOutcome,
    pub approval_id: Option<ApprovalId>,
    pub started_at: Timestamp,
    pub finished_at: Timestamp,
}
