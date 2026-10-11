//! Shared contracts for the multi-user linux-mcp service (Spec 00).
//!
//! Feature map (see `specs/contracts/decisions.md`, D1):
//! - `public` (default): browser-safe wire DTOs, IDs, errors and canonicalization.
//! - `schema` (default): JSON Schema derivation for wire types.
//! - `backend`: trusted identity, credential handles, operation authority and service traits.
//! - `test-support`: deterministic in-memory adapters and scenarios. Never enable in production.
//!
//! This crate depends on no implementation crate; implementations depend on it.
//!
//! # Pinned versions
//! [`CONTRACT_VERSION`], [`MANAGEMENT_API_VERSION`], [`MCP_EXTENSION_VERSION`],
//! [`MCP_LEGACY_VERSION`] and [`CANONICALIZATION_VERSION`] are asserted by `tests/schema_compat.rs`
//! against the committed schemas and fixtures. A breaking wire change requires bumping the
//! relevant version and a consumer migration plan.

#![forbid(unsafe_code)]

/// Semantic version of the shared contract. A breaking change requires a major bump.
pub const CONTRACT_VERSION: &str = "1.0.0";

/// Management API route prefix version (`/api/v1`).
pub const MANAGEMENT_API_VERSION: &str = "v1";

/// Proposed remote MCP extension schema version.
pub const MCP_EXTENSION_VERSION: &str = "linux-mcp/v1";

/// Schema version label of the current (legacy) stdio tool surface.
pub const MCP_LEGACY_VERSION: &str = "linux-mcp/legacy-1";

#[cfg(feature = "public")]
pub mod canonical;

#[cfg(feature = "public")]
pub mod public;

#[cfg(feature = "backend")]
pub mod backend;

#[cfg(feature = "test-support")]
pub mod test_support;

#[cfg(feature = "public")]
pub use canonical::CANONICALIZATION_VERSION;

/// Curated re-exports of the browser-safe contract types.
///
/// Backend items are deliberately **not** re-exported here: reach them under
/// [`backend`] so their use is explicit and cannot leak into public builds.
#[cfg(feature = "public")]
pub mod prelude {
    pub use crate::canonical::{CANONICALIZATION_VERSION, IntentDigest};
    pub use crate::public::api::{ErrorEnvelope, ROUTES, RouteSpec};
    pub use crate::public::errors::{ErrorCode, PublicError};
    pub use crate::public::ids::{
        ApprovalId, AuditRecordId, CredentialId, HostTrustId, InstallationId, OperationId,
        RequestId, ServerId, ServerRevision, Timestamp, UserId,
    };
    pub use crate::public::inventory::{
        CreateServerRequest, Environment, HostTrustStatus, ServerSummary, UpdateServerRequest,
    };
    pub use crate::public::operations::{
        ApprovalStatus, ApprovalSummary, OperationIntent, OperationOutcome, OperationRequest,
        OperationResult, RefusalReason, RiskLevel, SubmissionState,
    };
    pub use crate::public::pagination::{Page, PageRequest};
}
