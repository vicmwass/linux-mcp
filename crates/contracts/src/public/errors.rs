//! Sanitized public errors.
//!
//! Errors carry a closed [`ErrorCode`], a fixed message per code, the [`RequestId`]
//! and optional closed [`ErrorDetails`]. Provider, SQL, SSH and secret-store text never
//! enters a [`PublicError`]: there is no constructor that accepts a free-form message.
//!
//! Foreign-owned and absent resources are indistinguishable: [`PublicError::not_found`]
//! takes no resource argument, so callers cannot accidentally encode which case occurred.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::ids::{ApprovalId, RequestId, ServerRevision, impl_json_schema};

/// Closed set of public error codes (snake_case on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unauthenticated,
    Forbidden,
    NotFound,
    Validation,
    RevisionConflict,
    ApprovalRequired,
    Unavailable,
    Timeout,
    OutcomeUnknown,
}

impl ErrorCode {
    pub const ALL: [ErrorCode; 9] = [
        ErrorCode::Unauthenticated,
        ErrorCode::Forbidden,
        ErrorCode::NotFound,
        ErrorCode::Validation,
        ErrorCode::RevisionConflict,
        ErrorCode::ApprovalRequired,
        ErrorCode::Unavailable,
        ErrorCode::Timeout,
        ErrorCode::OutcomeUnknown,
    ];

    /// HTTP status used by the management API (decisions.md D8).
    pub const fn http_status(self) -> u16 {
        match self {
            ErrorCode::Unauthenticated => 401,
            ErrorCode::Forbidden => 403,
            ErrorCode::NotFound => 404,
            ErrorCode::Validation => 422,
            ErrorCode::RevisionConflict => 409,
            ErrorCode::ApprovalRequired => 428,
            ErrorCode::Unavailable => 503,
            ErrorCode::Timeout => 504,
            ErrorCode::OutcomeUnknown => 502,
        }
    }

    /// The only message text a server emits for this code.
    pub const fn message(self) -> &'static str {
        match self {
            ErrorCode::Unauthenticated => "Authentication is required.",
            ErrorCode::Forbidden => "The credential does not grant this action.",
            ErrorCode::NotFound => "The requested resource was not found.",
            ErrorCode::Validation => "The request is invalid.",
            ErrorCode::RevisionConflict => "The resource was changed by another request.",
            ErrorCode::ApprovalRequired => "This operation requires human approval.",
            ErrorCode::Unavailable => "The service is temporarily unavailable.",
            ErrorCode::Timeout => "The operation timed out before it was submitted.",
            ErrorCode::OutcomeUnknown => {
                "The operation was submitted but its outcome is unknown. It will not be retried."
            }
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorCode::Unauthenticated => "unauthenticated",
            ErrorCode::Forbidden => "forbidden",
            ErrorCode::NotFound => "not_found",
            ErrorCode::Validation => "validation",
            ErrorCode::RevisionConflict => "revision_conflict",
            ErrorCode::ApprovalRequired => "approval_required",
            ErrorCode::Unavailable => "unavailable",
            ErrorCode::Timeout => "timeout",
            ErrorCode::OutcomeUnknown => "outcome_unknown",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Validation rule that a value violated. Values themselves are never echoed back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ViolationRule {
    Required,
    TooShort,
    TooLong,
    InvalidFormat,
    OutOfRange,
    UnknownField,
    NotAllowed,
    NoChanges,
}

impl fmt::Display for ViolationRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ViolationRule::Required => "required",
            ViolationRule::TooShort => "too_short",
            ViolationRule::TooLong => "too_long",
            ViolationRule::InvalidFormat => "invalid_format",
            ViolationRule::OutOfRange => "out_of_range",
            ViolationRule::UnknownField => "unknown_field",
            ViolationRule::NotAllowed => "not_allowed",
            ViolationRule::NoChanges => "no_changes",
        };
        f.write_str(s)
    }
}

/// A single invalid value, produced by validated-type parsers that do not know the field name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid value: {rule}")]
pub struct InvalidValue {
    pub rule: ViolationRule,
}

impl InvalidValue {
    pub const fn new(rule: ViolationRule) -> Self {
        Self { rule }
    }
}

/// Dotted field path such as `hostname` or `items[2].port`.
///
/// Limited to `[a-z0-9_.\[\]]`, 1–128 characters, so it can never carry user-supplied values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FieldPath(String);

impl_json_schema!(FieldPath, "FieldPath", {
    "type": "string",
    "pattern": r"^[a-z0-9_.\[\]]{1,128}$"
});

impl FieldPath {
    pub fn new(path: impl Into<String>) -> Result<Self, InvalidValue> {
        let path = path.into();
        if path.is_empty() || path.len() > 128 {
            return Err(InvalidValue::new(ViolationRule::InvalidFormat));
        }
        if !path
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.[]".contains(&b))
        {
            return Err(InvalidValue::new(ViolationRule::InvalidFormat));
        }
        Ok(Self(path))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for FieldPath {
    type Error = InvalidValue;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<FieldPath> for String {
    fn from(value: FieldPath) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FieldViolation {
    pub field: FieldPath,
    pub rule: ViolationRule,
}

impl FieldViolation {
    /// Panics only on a malformed static field path, which is a programming error.
    pub fn new(field: &str, rule: ViolationRule) -> Self {
        Self {
            field: FieldPath::new(field).expect("static field path must be valid"),
            rule,
        }
    }
}

/// Domain validation failure with one or more field violations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("validation failed for {} field(s)", violations.len())]
pub struct ValidationError {
    pub violations: Vec<FieldViolation>,
}

impl ValidationError {
    pub fn single(field: &str, rule: ViolationRule) -> Self {
        Self {
            violations: vec![FieldViolation::new(field, rule)],
        }
    }

    pub fn from_invalid(field: &str, error: InvalidValue) -> Self {
        Self::single(field, error.rule)
    }
}

/// Approval states visible to callers in an `approval_required` error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ApprovalErrorReason {
    /// No approval exists for this exact intent.
    Missing,
    /// An approval exists but is still waiting for a human decision.
    Pending,
    /// The approval expired.
    Expired,
    /// The approval was already consumed by a previous dispatch.
    Consumed,
    /// The approval was rejected by the human approver.
    Rejected,
    /// Arguments, target revision, identity or schema version no longer match the binding.
    BindingMismatch,
}

/// Closed set of structured error details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ErrorDetails {
    FieldViolations {
        violations: Vec<FieldViolation>,
    },
    RevisionConflict {
        current_revision: ServerRevision,
    },
    ApprovalRequired {
        reason: ApprovalErrorReason,
        approval_id: Option<ApprovalId>,
    },
    RetryAfter {
        retry_after_ms: u32,
    },
}

/// Public error envelope used by the management API and MCP structured results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PublicError {
    code: ErrorCode,
    message: String,
    request_id: RequestId,
    details: Option<ErrorDetails>,
}

impl PublicError {
    fn with(code: ErrorCode, request_id: RequestId, details: Option<ErrorDetails>) -> Self {
        Self {
            code,
            message: code.message().to_owned(),
            request_id,
            details,
        }
    }

    pub fn unauthenticated(request_id: RequestId) -> Self {
        Self::with(ErrorCode::Unauthenticated, request_id, None)
    }

    /// Scope/permission failure that does not depend on whether a resource exists.
    pub fn forbidden(request_id: RequestId) -> Self {
        Self::with(ErrorCode::Forbidden, request_id, None)
    }

    /// Absent and foreign-owned resources both use this; it intentionally takes no resource.
    pub fn not_found(request_id: RequestId) -> Self {
        Self::with(ErrorCode::NotFound, request_id, None)
    }

    pub fn validation(request_id: RequestId, error: ValidationError) -> Self {
        Self::with(
            ErrorCode::Validation,
            request_id,
            Some(ErrorDetails::FieldViolations {
                violations: error.violations,
            }),
        )
    }

    pub fn revision_conflict(request_id: RequestId, current_revision: ServerRevision) -> Self {
        Self::with(
            ErrorCode::RevisionConflict,
            request_id,
            Some(ErrorDetails::RevisionConflict { current_revision }),
        )
    }

    pub fn approval_required(
        request_id: RequestId,
        reason: ApprovalErrorReason,
        approval_id: Option<ApprovalId>,
    ) -> Self {
        Self::with(
            ErrorCode::ApprovalRequired,
            request_id,
            Some(ErrorDetails::ApprovalRequired {
                reason,
                approval_id,
            }),
        )
    }

    pub fn unavailable(request_id: RequestId, retry_after_ms: Option<u32>) -> Self {
        Self::with(
            ErrorCode::Unavailable,
            request_id,
            retry_after_ms.map(|retry_after_ms| ErrorDetails::RetryAfter { retry_after_ms }),
        )
    }

    pub fn timeout(request_id: RequestId) -> Self {
        Self::with(ErrorCode::Timeout, request_id, None)
    }

    pub fn outcome_unknown(request_id: RequestId) -> Self {
        Self::with(ErrorCode::OutcomeUnknown, request_id, None)
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn request_id(&self) -> RequestId {
        self.request_id
    }

    pub fn details(&self) -> Option<&ErrorDetails> {
        self.details.as_ref()
    }

    pub fn http_status(&self) -> u16 {
        self.code.http_status()
    }
}

impl fmt::Display for PublicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}): {}", self.code, self.request_id, self.message)
    }
}

impl std::error::Error for PublicError {}
