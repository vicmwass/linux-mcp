//! MCP compatibility types and versioned extensions (Spec 00, step 00-08).
//!
//! Two explicitly versioned surfaces:
//!
//! - **`linux-mcp/legacy-1`** — the current stdio server: nine tools, text-only results.
//!   [`LEGACY_TOOLS`] maps each to shared IDs and risk; [`classify_legacy_text`] is the adapter
//!   that recovers typed meaning from the unstructured text. Captured wire fixtures live in
//!   `fixtures/mcp/legacy/` (real output, see its README).
//! - **`linux-mcp/v1`** — proposed remote extensions (not implemented): structured results
//!   ([`ToolResultEnvelope`]) and the prepare/submit/status tools ([`PROPOSED_TOOLS`]).
//!
//! Rules: an unknown schema version is rejected ([`McpSchemaVersion::parse`]), never guessed.
//! Legacy `confirm: true` expresses caller intent only; it is **never** approval evidence
//! ([`LEGACY_CONFIRM_IS_APPROVAL_EVIDENCE`]). Legacy stdio keeps its semantics unchanged; the
//! remote service adds approvals through the separate v1 tools instead of reinterpreting
//! `confirm`.
//!
//! Nothing here claims live MCP transport interoperability.

use serde::{Deserialize, Serialize};

use crate::canonical::IntentDigest;
use crate::public::errors::{InvalidValue, PublicError, ViolationRule};
use crate::public::ids::{
    ApprovalId, OperationId, RequestId, ServerId, Timestamp, impl_json_schema,
};
use crate::public::operations::{
    ApprovalStatus, OperationOutcome, OperationRequest, RiskLevel, SubmissionState, ToolId,
};

pub const LEGACY_SCHEMA_VERSION: &str = "linux-mcp/legacy-1";
pub const EXTENSION_SCHEMA_VERSION: &str = "linux-mcp/v1";

/// `confirm: true` on legacy tools is an agent-supplied boolean, not human approval.
pub const LEGACY_CONFIRM_IS_APPROVAL_EVIDENCE: bool = false;

/// Known schema versions. Anything else is rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum McpSchemaVersion {
    Legacy1,
    V1,
}

impl McpSchemaVersion {
    pub fn parse(value: &str) -> Result<Self, InvalidValue> {
        match value {
            LEGACY_SCHEMA_VERSION => Ok(Self::Legacy1),
            EXTENSION_SCHEMA_VERSION => Ok(Self::V1),
            _ => Err(InvalidValue::new(ViolationRule::NotAllowed)),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Legacy1 => LEGACY_SCHEMA_VERSION,
            Self::V1 => EXTENSION_SCHEMA_VERSION,
        }
    }
}

/// Marker that serializes as `"linux-mcp/v1"` and rejects every other value on input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExtensionVersionV1;

impl Serialize for ExtensionVersionV1 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(EXTENSION_SCHEMA_VERSION)
    }
}

impl<'de> Deserialize<'de> for ExtensionVersionV1 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        if value == EXTENSION_SCHEMA_VERSION {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("unsupported schema_version"))
        }
    }
}

impl_json_schema!(ExtensionVersionV1, "ExtensionVersionV1", {
    "type": "string",
    "const": "linux-mcp/v1"
});

// ---------------------------------------------------------------------------------------
// Legacy tool map
// ---------------------------------------------------------------------------------------

/// One current tool, as defined in `src/mcp/server.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyTool {
    pub name: &'static str,
    /// Properties the captured `inputSchema` lists as required, in schema order.
    pub required: &'static [&'static str],
    /// All properties of the captured `inputSchema`.
    pub properties: &'static [&'static str],
    pub mutating: bool,
    /// Fixed risk, or `None` when it depends on the classified command (`execute_command`).
    pub risk: Option<RiskLevel>,
    /// The tool's own `confirm` flag (legacy expression of intent).
    pub has_confirm: bool,
    /// Whether the tool takes a `server_id` (everything but `list_servers`).
    pub takes_server_id: bool,
}

impl LegacyTool {
    pub fn tool_id(&self) -> ToolId {
        ToolId::new(self.name).expect("legacy tool names are valid tool IDs")
    }
}

/// The nine tools exposed today, verified against the captured `tools/list` fixture.
pub const LEGACY_TOOLS: &[LegacyTool] = &[
    LegacyTool {
        name: "create_user",
        required: &["server_id", "username"],
        properties: &["home", "server_id", "shell", "username"],
        mutating: true,
        risk: Some(RiskLevel::Medium),
        has_confirm: false,
        takes_server_id: true,
    },
    LegacyTool {
        name: "delete_user",
        required: &["server_id", "username", "confirm", "remove_home"],
        properties: &["confirm", "remove_home", "server_id", "username"],
        mutating: true,
        risk: Some(RiskLevel::High),
        has_confirm: true,
        takes_server_id: true,
    },
    LegacyTool {
        name: "disable_user",
        required: &["server_id", "username"],
        properties: &["server_id", "username"],
        mutating: true,
        risk: Some(RiskLevel::Medium),
        has_confirm: false,
        takes_server_id: true,
    },
    LegacyTool {
        name: "enable_user",
        required: &["server_id", "username"],
        properties: &["server_id", "username"],
        mutating: true,
        risk: Some(RiskLevel::Medium),
        has_confirm: false,
        takes_server_id: true,
    },
    LegacyTool {
        name: "execute_command",
        required: &["server_id", "command"],
        properties: &["command", "confirm", "server_id"],
        mutating: true,
        risk: None,
        has_confirm: true,
        takes_server_id: true,
    },
    LegacyTool {
        name: "get_server_capabilities",
        required: &["server_id"],
        properties: &["server_id"],
        mutating: false,
        risk: Some(RiskLevel::Low),
        has_confirm: false,
        takes_server_id: true,
    },
    LegacyTool {
        name: "get_user",
        required: &["server_id", "username"],
        properties: &["server_id", "username"],
        mutating: false,
        risk: Some(RiskLevel::Low),
        has_confirm: false,
        takes_server_id: true,
    },
    LegacyTool {
        name: "list_servers",
        required: &[],
        properties: &[],
        mutating: false,
        risk: Some(RiskLevel::Low),
        has_confirm: false,
        takes_server_id: false,
    },
    LegacyTool {
        name: "list_users",
        required: &["server_id"],
        properties: &["server_id"],
        mutating: false,
        risk: Some(RiskLevel::Low),
        has_confirm: false,
        takes_server_id: true,
    },
];

pub fn legacy_tool(name: &str) -> Option<&'static LegacyTool> {
    LEGACY_TOOLS.iter().find(|t| t.name == name)
}

// ---------------------------------------------------------------------------------------
// Legacy text adapter
// ---------------------------------------------------------------------------------------

/// Typed meaning recovered from a legacy text result.
///
/// Legacy results carry no structure, so this classification is a heuristic adapter keyed on
/// the exact prefixes in `src/mcp/server.rs` and `src/tools/*.rs`. Anything it does not
/// recognize is [`Unrecognized`](Self::Unrecognized) — never assumed to be success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyClassification {
    /// `execute_command` ran and reported this exit code (zero or not).
    ExecCompleted {
        exit_code: u32,
    },
    /// A non-`execute_command` tool reported its documented success text.
    ToolSucceeded,
    /// The tool required its own `confirm` flag (legacy; not approval).
    ConfirmationRequired,
    /// Policy/classifier denied or rejected the command; maps to `refused`.
    Refused,
    /// A failure with no sign the command was submitted (capability, resolve, validation, connect).
    Failed,
    /// Execution failed *and* the text says the remote outcome may be unknown (timeout).
    PossiblyUnknownOutcome,
    /// rmcp rejected the arguments (`isError: true`).
    InvalidArguments,
    Unrecognized,
}

/// Classifies one legacy `tools/call` result. `is_error` is the result's `isError` flag.
pub fn classify_legacy_text(tool: &str, text: &str, is_error: bool) -> LegacyClassification {
    use LegacyClassification as C;
    if is_error {
        return C::InvalidArguments;
    }
    if text == "This operation requires explicit confirmation."
        || text.contains("requires explicit confirmation. Set confirm=true.")
    {
        return C::ConfirmationRequired;
    }
    if text == "This operation is not permitted."
        || text.starts_with("Security policy rejected the command:")
        || text.starts_with("Command rejected:")
    {
        return C::Refused;
    }
    if text.starts_with("Command execution failed:") {
        return if text.contains("remote outcome may be unknown") {
            C::PossiblyUnknownOutcome
        } else {
            C::Failed
        };
    }
    if tool == "execute_command"
        && let Some(rest) = text.strip_prefix("Exit code: ")
        && let Some((code, _)) = rest.split_once(
            "
STDOUT:
",
        )
        && let Ok(exit_code) = code.parse::<u32>()
    {
        return C::ExecCompleted { exit_code };
    }
    const FAILURE_PREFIXES: &[&str] = &[
        "Failed to detect server capabilities:",
        "Failed to list users:",
        "Failed to get user:",
        "Failed to create user:",
        "Failed to disable user:",
        "Failed to enable user:",
        "Failed to delete user:",
    ];
    if FAILURE_PREFIXES.iter().any(|p| text.starts_with(p)) {
        return C::Failed;
    }
    if tool != "execute_command"
        && legacy_tool(tool).is_some_and(|t| looks_like_success(t.name, text))
    {
        return C::ToolSucceeded;
    }
    C::Unrecognized
}

fn looks_like_success(tool: &str, text: &str) -> bool {
    match tool {
        "list_servers" => text == "No Linux servers are configured." || text.starts_with("ID: "),
        "list_users" => text == "No users found." || text.starts_with("Username: "),
        "get_user" => text.starts_with("Username: "),
        "get_server_capabilities" => text.starts_with("Server: "),
        "create_user" => text.starts_with("User '") && text.ends_with("created successfully."),
        "disable_user" => {
            text.starts_with("User '") && text.ends_with("has been disabled successfully.")
        }
        "enable_user" => {
            text.starts_with("User '") && text.ends_with("has been enabled successfully.")
        }
        "delete_user" => {
            text.starts_with("User '") && text.contains("has been permanently deleted")
        }
        _ => false,
    }
}

impl LegacyClassification {
    /// Maps to the shared outcome type where one exists. Success texts need the adapter to
    /// supply output; they are not converted here.
    pub fn refusal(&self) -> Option<crate::public::operations::RefusalReason> {
        use crate::public::operations::RefusalReason as R;
        match self {
            Self::Refused => Some(R::PolicyDenied),
            Self::ConfirmationRequired => Some(R::ConfirmationRequired),
            Self::InvalidArguments => Some(R::InvalidArguments),
            _ => None,
        }
    }

    /// `true` when the remote command might have run, so the result must not be retried.
    pub fn may_have_run(&self) -> bool {
        matches!(
            self,
            Self::ExecCompleted { .. } | Self::ToolSucceeded | Self::PossiblyUnknownOutcome
        )
    }
}

// ---------------------------------------------------------------------------------------
// Proposed v1 extension (not implemented)
// ---------------------------------------------------------------------------------------

/// Proposed additional tools of the remote service. Legacy tools keep their names and shapes.
pub const PROPOSED_TOOLS: &[&str] = &[
    "prepare_operation",
    "submit_operation",
    "get_operation_status",
];

/// `prepare_operation` input: same shape as the management API's operation request.
pub type PrepareOperationParams = OperationRequest;

/// `submit_operation` input. The approval ID only *references* an approval; authority is
/// minted server-side by atomic consume, so a guessed or foreign ID grants nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SubmitOperationParams {
    pub operation_id: OperationId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_id: Option<ApprovalId>,
}

/// `get_operation_status` input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct OperationStatusParams {
    pub operation_id: OperationId,
}

/// Returned instead of executing when human approval is required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ApprovalRequiredResult {
    pub approval_id: ApprovalId,
    pub status: ApprovalStatus,
    pub intent_digest: IntentDigest,
    pub expires_at: Timestamp,
    /// Dashboard-relative path the owner opens to decide, e.g. `/approvals/<id>`. Never carries
    /// credentials or tokens.
    pub dashboard_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToolResultBody {
    /// A completed or refused operation. `submission` shows where it stopped.
    Outcome {
        operation_id: OperationId,
        server_id: ServerId,
        submission: SubmissionState,
        outcome: OperationOutcome,
    },
    ApprovalRequired(ApprovalRequiredResult),
    /// Sanitized error (authentication, validation, not found, unavailable...).
    Error {
        error: PublicError,
    },
}

/// Structured result of a v1 tool call, returned as MCP `structuredContent` alongside a short
/// text rendering. `isError` is `true` only for the `error` kind and for `refused`,
/// `failed_before_submission` and `unknown_after_submission` outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ToolResultEnvelope {
    pub schema_version: ExtensionVersionV1,
    pub request_id: RequestId,
    pub result: ToolResultBody,
}

impl ToolResultEnvelope {
    pub fn new(request_id: RequestId, result: ToolResultBody) -> Self {
        Self {
            schema_version: ExtensionVersionV1,
            request_id,
            result,
        }
    }

    pub fn is_error(&self) -> bool {
        match &self.result {
            ToolResultBody::Error { .. } => true,
            ToolResultBody::ApprovalRequired(_) => false,
            ToolResultBody::Outcome { outcome, .. } => {
                !matches!(outcome, OperationOutcome::Succeeded { .. })
            }
        }
    }
}

/// Initialize-time discovery extension a v1 server advertises under
/// `capabilities.experimental["linux-mcp"]`; clients that do not see it use legacy behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ExtensionDiscovery {
    pub schema_version: ExtensionVersionV1,
    pub tools: Vec<ToolId>,
    pub approval_flow: bool,
    pub structured_results: bool,
}

impl ExtensionDiscovery {
    pub fn v1() -> Self {
        Self {
            schema_version: ExtensionVersionV1,
            tools: PROPOSED_TOOLS
                .iter()
                .map(|t| ToolId::new(*t).unwrap())
                .collect(),
            approval_flow: true,
            structured_results: true,
        }
    }
}
