//! Server inventory DTOs.
//!
//! Field constraints (for Specs 01 and 04–08):
//!
//! | Field | Type | Constraint |
//! |---|---|---|
//! | `display_name` | [`DisplayName`] | 1–100 Unicode scalar values, no control characters, trimmed |
//! | `hostname` | [`Hostname`] | DNS name (labels `[a-z0-9-]`, 1–63, no edge hyphen, total ≤ 253) or IPv4/IPv6 literal without brackets; lowercased |
//! | `port` | [`SshPort`] | 1–65535, default 22 |
//! | `username` | [`SshUsername`] | 1–32, `^[a-z_][a-z0-9_-]*$` (same rule as the current user tools) |
//! | `environment` | [`Environment`] | `development`, `testing`, `staging`, `production` |
//! | `approval_mode` | [`ApprovalMode`] | `medium_and_above` (default) or `high_and_above` |
//!
//! Hostname syntax validation is not an outbound-destination policy: Spec 01 still applies
//! network/port allow-lists and DNS rechecks at connection time.
//!
//! Requests have no owner field and reject unknown fields, so a submitted
//! `owner_user_id` is a validation error rather than silently ignored. Responses carry
//! credential *status* only, never credential content or references to host paths.

use std::fmt;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use super::errors::{InvalidValue, ValidationError, ViolationRule};
use super::ids::{CredentialId, ServerId, ServerRevision, Timestamp, impl_json_schema};
use super::operations::RiskLevel;

macro_rules! validated_string {
    ($(#[$doc:meta])* $name:ident, $validate:path, { $($schema:tt)* }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidValue> {
                $validate(value.into()).map(Self)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = InvalidValue;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl_json_schema!($name, stringify!($name), { $($schema)* });
    };
}

fn validate_display_name(value: String) -> Result<String, InvalidValue> {
    let trimmed = value.trim();
    let count = trimmed.chars().count();
    if count == 0 {
        return Err(InvalidValue::new(ViolationRule::TooShort));
    }
    if count > 100 {
        return Err(InvalidValue::new(ViolationRule::TooLong));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(InvalidValue::new(ViolationRule::InvalidFormat));
    }
    Ok(trimmed.to_owned())
}

fn validate_hostname(value: String) -> Result<String, InvalidValue> {
    let invalid = InvalidValue::new(ViolationRule::InvalidFormat);
    if value.is_empty() {
        return Err(InvalidValue::new(ViolationRule::TooShort));
    }
    if value.len() > 253 {
        return Err(InvalidValue::new(ViolationRule::TooLong));
    }
    if let Ok(ip) = value.parse::<IpAddr>() {
        // Normalized textual form (e.g. compressed IPv6).
        return Ok(ip.to_string());
    }
    let lower = value.to_ascii_lowercase();
    let labels_ok = lower.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    });
    // An all-numeric dotted name that is not a valid IP (e.g. 999.1.1.1) is rejected.
    let all_numeric = lower.bytes().all(|b| b.is_ascii_digit() || b == b'.');
    if !labels_ok || all_numeric {
        return Err(invalid);
    }
    Ok(lower)
}

fn validate_username(value: String) -> Result<String, InvalidValue> {
    if value.is_empty() {
        return Err(InvalidValue::new(ViolationRule::TooShort));
    }
    if value.len() > 32 {
        return Err(InvalidValue::new(ViolationRule::TooLong));
    }
    let first_ok = value
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_lowercase() || b == b'_');
    let rest_ok = value
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
    if !first_ok || !rest_ok {
        return Err(InvalidValue::new(ViolationRule::InvalidFormat));
    }
    Ok(value)
}

validated_string!(
    /// Owner-chosen label. Not unique and not an identifier.
    DisplayName,
    validate_display_name,
    { "type": "string", "minLength": 1, "maxLength": 100 }
);

validated_string!(
    /// Network target syntax: DNS name or IP literal. Not a destination authorization.
    Hostname,
    validate_hostname,
    { "type": "string", "minLength": 1, "maxLength": 253 }
);

validated_string!(
    /// SSH login username on the managed server.
    SshUsername,
    validate_username,
    { "type": "string", "minLength": 1, "maxLength": 32, "pattern": "^[a-z_][a-z0-9_-]*$" }
);

/// SSH port, 1–65535.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct SshPort(u16);

impl SshPort {
    pub const DEFAULT: SshPort = SshPort(22);

    pub fn new(value: u16) -> Result<Self, InvalidValue> {
        if value == 0 {
            Err(InvalidValue::new(ViolationRule::OutOfRange))
        } else {
            Ok(Self(value))
        }
    }

    pub fn get(self) -> u16 {
        self.0
    }
}

impl Default for SshPort {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TryFrom<u16> for SshPort {
    type Error = InvalidValue;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<SshPort> for u16 {
    fn from(value: SshPort) -> Self {
        value.0
    }
}

impl_json_schema!(SshPort, "SshPort", {
    "type": "integer",
    "minimum": 1,
    "maximum": 65535,
    "default": 22
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    Development,
    Testing,
    Staging,
    Production,
}

/// Host-key trust state for a server's (hostname, port).
///
/// `Observed` is never trust: it only means a fingerprint is waiting for independent
/// verification. Only `Verified` permits connections; `Mismatch` blocks until reverified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum HostTrustStatus {
    Unenrolled,
    Observed,
    Verified,
    Mismatch,
}

impl HostTrustStatus {
    pub fn permits_connection(self) -> bool {
        matches!(self, HostTrustStatus::Verified)
    }
}

/// Public host-trust summary. Fingerprints are public key digests, not secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct HostTrustSummary {
    pub status: HostTrustStatus,
    /// `SHA256:<base64>` OpenSSH-style fingerprint of the verified or observed key.
    pub fingerprint: Option<String>,
    pub verified_at: Option<Timestamp>,
}

impl HostTrustSummary {
    pub fn unenrolled() -> Self {
        Self {
            status: HostTrustStatus::Unenrolled,
            fingerprint: None,
            verified_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CredentialKind {
    SshPrivateKey,
    SudoPassword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CredentialState {
    Missing,
    Provisioned,
    /// Sudo without a password (`sudo -n`) is configured; no secret is stored.
    NotRequired,
}

/// Status and reference of one credential slot. Never includes secret material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CredentialStatus {
    pub kind: CredentialKind,
    pub state: CredentialState,
    pub credential_id: Option<CredentialId>,
    pub updated_at: Option<Timestamp>,
}

/// Per-server rule for which operations need human approval (decisions.md D11).
///
/// Set with the server configuration. `MediumAndAbove` is the default and the recommended
/// setting. Neither mode can bypass a policy denial, and `confirm: true` is never approval.
/// Because lowering the threshold weakens a safeguard, changing this setting away from the
/// default requires an interactive browser session (not an MCP or delegated token), so an
/// agent cannot disable approvals for itself. Any change bumps the server revision, which
/// invalidates pending approvals bound to the old setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    /// Medium, high and critical-risk mutations need approval (critical is always denied).
    #[default]
    MediumAndAbove,
    /// Only high-risk mutations need approval; medium-risk operations (for example creating a
    /// user, restarting a service, installing a package) run without it.
    HighAndAbove,
}

impl ApprovalMode {
    /// Whether an operation of `risk` needs human approval under this mode. Low risk never does.
    pub fn requires_approval(self, risk: RiskLevel) -> bool {
        match self {
            ApprovalMode::MediumAndAbove => risk >= RiskLevel::Medium,
            ApprovalMode::HighAndAbove => risk >= RiskLevel::High,
        }
    }

    /// `true` when this is the default (safest) mode.
    pub fn is_default(self) -> bool {
        self == ApprovalMode::MediumAndAbove
    }
}

/// Owned server as returned to its owner (spec 00 `ServerSummary`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ServerSummary {
    pub id: ServerId,
    pub display_name: DisplayName,
    pub hostname: Hostname,
    pub port: SshPort,
    pub username: SshUsername,
    pub environment: Environment,
    pub revision: ServerRevision,
    pub enabled: bool,
    pub approval_mode: ApprovalMode,
    pub host_trust: HostTrustSummary,
    pub credentials: Vec<CredentialStatus>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

fn default_true() -> bool {
    true
}

/// `POST /api/v1/servers`. The owner comes from the authenticated principal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CreateServerRequest {
    pub display_name: DisplayName,
    pub hostname: Hostname,
    #[serde(default)]
    pub port: SshPort,
    pub username: SshUsername,
    pub environment: Environment,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Defaults to `medium_and_above`. A non-default value requires an interactive browser session.
    #[serde(default)]
    pub approval_mode: ApprovalMode,
}

/// `PATCH /api/v1/servers/{id}`. Absent fields are unchanged; at least one change is required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct UpdateServerRequest {
    /// Last revision the caller observed; a stale value yields `revision_conflict`.
    pub expected_revision: ServerRevision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<DisplayName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<Hostname>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<SshPort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<SshUsername>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Changing the approval mode requires an interactive browser session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_mode: Option<ApprovalMode>,
}

impl UpdateServerRequest {
    /// Semantic validation beyond per-field types.
    pub fn validate(&self) -> Result<(), ValidationError> {
        let changes = self.display_name.is_some()
            || self.hostname.is_some()
            || self.port.is_some()
            || self.username.is_some()
            || self.environment.is_some()
            || self.enabled.is_some()
            || self.approval_mode.is_some();
        if changes {
            Ok(())
        } else {
            Err(ValidationError::single("body", ViolationRule::NoChanges))
        }
    }

    /// Whether this update changes the network target or login identity, which must
    /// invalidate pooled connections, capability caches and pending approvals.
    pub fn changes_target(&self) -> bool {
        self.hostname.is_some() || self.port.is_some() || self.username.is_some()
    }

    /// Whether this update changes the approval policy (restricted to interactive sessions).
    pub fn changes_approval_policy(&self) -> bool {
        self.approval_mode.is_some()
    }
}

/// `DELETE /api/v1/servers/{id}?expected_revision=N`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DeleteServerRequest {
    pub expected_revision: ServerRevision,
}
