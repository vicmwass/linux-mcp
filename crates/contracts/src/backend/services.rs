//! Async service interfaces (Spec 00 "Service boundaries"; decisions.md D4, D6, D9).
//!
//! All traits are object safe (`Arc<dyn Inventory>` etc.), `Send + Sync + 'static`, take a
//! context first, and return [`ServiceResult`] (sanitized [`PublicError`]s; never provider,
//! SQL or SSH text). There is no general-purpose secret getter: secrets are reachable only
//! through [`CredentialStore::resolve_for`], which requires an [`AuthorizedTarget`].
//!
//! # Contexts
//! [`CallContext`] carries request ID and deadline and is all an *unauthenticated* entry
//! point (login, token validation) may use. [`RequestContext`] adds the trusted
//! [`Principal`] and is created only from the output of [`Identity`].
//!
//! # Cancellation and deadlines
//! Cancellation is future drop; deadlines are absolute. Before `Audit::record_submitted`
//! returns, dropping or timing out means *nothing was sent*:
//! [`OperationOutcome::FailedBeforeSubmission`]. After it, a lost result is
//! [`OperationOutcome::UnknownAfterSubmission`]; cancellation cannot recall a submitted remote
//! command, and nothing replays it.
//!
//! # Call sequence for an operation (executed by 04 or 08, implemented by 01/03/06)
//! 1. `Identity::validate_mcp_token` / `authenticate_browser` → `Principal` → `RequestContext`.
//! 2. `TargetResolver::resolve(ctx, server_id, scope)` — ownership, enabled and host-trust
//!    checks happen here, **before** any secret access or network use. Foreign == absent.
//! 3. Build an `OperationIntent`; `Executor::evaluate(ctx, &intent, &target)` →
//!    `PolicyEvaluation`.
//! 4. `Denied` → return `OperationOutcome::Refused`. `RequiresApproval` →
//!    `Approvals::request`, return the pending approval; a human later calls
//!    `Approvals::decide`. `Allowed` → `ExecutionAuthority::without_approval`.
//! 5. For approved intents `Approvals::consume(ctx, approval_id, &intent, &target, &policy)`
//!    atomically marks the approval consumed once and returns the `ExecutionAuthority`.
//!    Concurrent consumers get exactly one `Ok`. The consumption is never undone.
//! 6. `Audit::begin_operation` durably records `SubmissionPending`; if audit is unavailable
//!    the mutation is rejected with `unavailable` and nothing is sent.
//! 7. `Executor::submit(ctx, SubmitRequest{..})` re-resolves the target (`TargetResolver`),
//!    rechecks revision and policy (`ExecutionAuthority::recheck`), leases credentials,
//!    calls `Audit::record_submitted` immediately before the exec request, then
//!    `Audit::record_outcome`.
//! 8. Inventory/credential/trust changes publish [`TargetInvalidation`] to every registered
//!    [`InvalidationSink`]; sinks drop pooled connections and capability caches. Missed events
//!    are harmless because every operation rechecks the revision.
//!
//! Recovery on startup calls `Audit::reconcile_unresolved`, which marks any record stuck in
//! `SubmissionPending`/`Submitted` as `unknown_after_submission`. Nothing is replayed.

use std::sync::Arc;

use async_trait::async_trait;

use crate::backend::credentials::{CredentialLease, CredentialPurpose};
use crate::backend::identity::{Principal, Scope, SessionRef};
use crate::backend::operation_authority::{AuthorityError, ExecutionAuthority, PolicyEvaluation};
use crate::backend::target::AuthorizedTarget;
use crate::public::api::{
    ApprovalDecisionRequest, AuditAction, AuditQuery, AuditRecord, CredentialWriteResponse,
    HostKeyObservation, PutCredentialRequest,
};
use crate::public::errors::PublicError;
use crate::public::ids::{
    ApprovalId, AuditRecordId, InstallationId, OperationId, RequestId, ServerId, ServerRevision,
    Timestamp, UserId,
};
use crate::public::inventory::{
    CreateServerRequest, CredentialKind, ServerSummary, UpdateServerRequest,
};
use crate::public::operations::{
    ApprovalSummary, OperationIntent, OperationOutcome, OperationResult, SubmissionState,
};
use crate::public::pagination::{Page, PageRequest};

/// Result type of every service method.
pub type ServiceResult<T> = Result<T, PublicError>;

/// Context available before authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallContext {
    pub request_id: RequestId,
    /// Absolute deadline; implementations check it before each external step.
    pub deadline: Timestamp,
}

/// Context for authenticated calls: the principal always comes from [`Identity`].
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub call: CallContext,
    pub principal: Principal,
}

impl RequestContext {
    pub fn new(call: CallContext, principal: Principal) -> Self {
        Self { call, principal }
    }

    pub fn request_id(&self) -> RequestId {
        self.call.request_id
    }

    pub fn deadline(&self) -> Timestamp {
        self.call.deadline
    }

    pub fn is_past_deadline(&self, now: Timestamp) -> bool {
        now >= self.call.deadline
    }
}

// ---------------------------------------------------------------------------------------
// Inventory (owner: Spec 01)
// ---------------------------------------------------------------------------------------

/// Owner-scoped inventory. Every method constrains by `ctx.principal` ownership and
/// installation; foreign and absent IDs both yield `not_found`.
#[async_trait]
pub trait Inventory: Send + Sync + 'static {
    async fn list_owned(
        &self,
        ctx: &RequestContext,
        page: PageRequest,
    ) -> ServiceResult<Page<ServerSummary>>;

    async fn create(
        &self,
        ctx: &RequestContext,
        request: CreateServerRequest,
    ) -> ServiceResult<ServerSummary>;

    async fn get_owned(&self, ctx: &RequestContext, id: ServerId) -> ServiceResult<ServerSummary>;

    /// Fails with `revision_conflict` if `request.expected_revision` is stale. A change that
    /// affects the target increments the revision and publishes a [`TargetInvalidation`].
    async fn update(
        &self,
        ctx: &RequestContext,
        id: ServerId,
        request: UpdateServerRequest,
    ) -> ServiceResult<ServerSummary>;

    /// Disables future resolutions immediately; secret cleanup may be deferred.
    async fn delete(
        &self,
        ctx: &RequestContext,
        id: ServerId,
        expected_revision: ServerRevision,
    ) -> ServiceResult<()>;

    /// Host-key observation (no trust change) and verified enrollment (Spec 01).
    async fn observe_host_key(
        &self,
        ctx: &RequestContext,
        id: ServerId,
    ) -> ServiceResult<HostKeyObservation>;

    async fn enroll_host_key(
        &self,
        ctx: &RequestContext,
        id: ServerId,
        expected_revision: ServerRevision,
        fingerprint: String,
    ) -> ServiceResult<ServerSummary>;

    async fn revoke_host_key(
        &self,
        ctx: &RequestContext,
        id: ServerId,
        expected_revision: ServerRevision,
    ) -> ServiceResult<ServerSummary>;
}

// ---------------------------------------------------------------------------------------
// TargetResolver (owner: Spec 01)
// ---------------------------------------------------------------------------------------

/// Loads the **current** record and authorizes it for the principal and scope.
///
/// Must complete ownership, enabled and host-trust checks before returning; callers perform no
/// secret access or network connection for a server that did not resolve.
#[async_trait]
pub trait TargetResolver: Send + Sync + 'static {
    async fn resolve(
        &self,
        ctx: &RequestContext,
        server_id: ServerId,
        scope: Scope,
    ) -> ServiceResult<AuthorizedTarget>;
}

// ---------------------------------------------------------------------------------------
// CredentialStore (owner: Spec 01)
// ---------------------------------------------------------------------------------------

#[async_trait]
pub trait CredentialStore: Send + Sync + 'static {
    /// Provision or rotate one credential slot (write-only input). Bumps the server revision
    /// and publishes a [`TargetInvalidation`].
    async fn put(
        &self,
        ctx: &RequestContext,
        server_id: ServerId,
        request: PutCredentialRequest,
    ) -> ServiceResult<CredentialWriteResponse>;

    async fn delete(
        &self,
        ctx: &RequestContext,
        server_id: ServerId,
        kind: CredentialKind,
        expected_revision: ServerRevision,
    ) -> ServiceResult<CredentialWriteResponse>;

    /// The only path to secret material: a short-lived lease for an authorized target.
    /// Rejects references that are not part of `target` and kinds that mismatch `purpose`.
    async fn resolve_for(
        &self,
        ctx: &RequestContext,
        target: &AuthorizedTarget,
        purpose: CredentialPurpose,
    ) -> ServiceResult<CredentialLease>;
}

// ---------------------------------------------------------------------------------------
// Identity (owner: Spec 02)
// ---------------------------------------------------------------------------------------

/// Unauthenticated inputs. Bearer values are never logged or placed in errors.
#[derive(Clone, PartialEq, Eq)]
pub struct BearerToken(String);

impl BearerToken {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for BearerToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BearerToken(<redacted>)")
    }
}

/// Browser request material: session cookie and, for unsafe methods, the CSRF header value.
#[derive(Clone, PartialEq, Eq)]
pub struct BrowserCredentials {
    pub session_cookie: BearerToken,
    pub csrf_token: Option<BearerToken>,
    pub unsafe_method: bool,
}

impl std::fmt::Debug for BrowserCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BrowserCredentials(<redacted>)")
    }
}

/// What to revoke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevocationTarget {
    Session(SessionRef),
    /// All sessions and tokens of a user in one installation.
    User {
        installation_id: InstallationId,
        user_id: UserId,
    },
}

#[async_trait]
pub trait Identity: Send + Sync + 'static {
    /// Validates a dashboard session (and CSRF for unsafe methods). Produces a
    /// `BrowserSession` principal.
    async fn authenticate_browser(
        &self,
        ctx: &CallContext,
        credentials: BrowserCredentials,
    ) -> ServiceResult<Principal>;

    /// Validates issuer, audience/resource, signature, lifetime and scopes of an MCP access
    /// token for this installation. Tokens from other installations fail.
    async fn validate_mcp_token(
        &self,
        ctx: &CallContext,
        token: BearerToken,
    ) -> ServiceResult<Principal>;

    /// Revocation takes effect for new operations within the configured bound (60 s target);
    /// already-submitted operations may complete.
    async fn revoke(&self, ctx: &RequestContext, target: RevocationTarget) -> ServiceResult<()>;

    /// Revalidates a previously issued principal (long-lived sessions, reconnects).
    async fn revalidate(&self, ctx: &RequestContext) -> ServiceResult<()>;
}

// ---------------------------------------------------------------------------------------
// Executor (owner: Spec 03)
// ---------------------------------------------------------------------------------------

/// Read-only facts about a target, from capability detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetInspection {
    pub server_id: ServerId,
    pub server_revision: ServerRevision,
    pub distribution: String,
    pub version: String,
    pub kernel: String,
    pub architecture: String,
    pub init_system: String,
    pub package_manager: String,
    pub commands: Vec<String>,
    pub inspected_at: Timestamp,
}

/// Why cached state for a target must be dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationReason {
    InventoryChanged,
    CredentialChanged,
    HostTrustChanged,
    Disabled,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetInvalidation {
    pub installation_id: InstallationId,
    pub owner: UserId,
    pub server_id: ServerId,
    /// Revision after the change; cached state for lower revisions must not be reused.
    pub new_revision: ServerRevision,
    pub reason: InvalidationReason,
}

/// Consumer of invalidation events (connection pools, capability caches).
#[async_trait]
pub trait InvalidationSink: Send + Sync + 'static {
    async fn invalidate(&self, event: TargetInvalidation);
}

/// Everything `Executor::submit` needs. `authority` is consumed: it cannot be reused.
#[derive(Debug)]
pub struct SubmitRequest {
    pub intent: OperationIntent,
    pub authority: ExecutionAuthority,
    pub audit: AuditTicket,
}

#[async_trait]
pub trait Executor: Send + Sync + 'static {
    /// Capability inspection (read-only). Requires an authorized target.
    async fn inspect(
        &self,
        ctx: &RequestContext,
        target: &AuthorizedTarget,
    ) -> ServiceResult<TargetInspection>;

    /// Preflight policy evaluation bound to the intent digest and target revision.
    async fn evaluate(
        &self,
        ctx: &RequestContext,
        intent: &OperationIntent,
        target: &AuthorizedTarget,
    ) -> ServiceResult<PolicyEvaluation>;

    /// Submits exactly one authorized operation. Re-resolves the target, rechecks revision and
    /// policy, calls `Audit::record_submitted` immediately before the exec request, and
    /// `Audit::record_outcome` afterwards. Never retries after submission.
    ///
    /// Refusals and failures are `Ok(OperationResult)` with the typed outcome; `Err` is only
    /// for failures that occur before any state exists (for example `unauthenticated`).
    async fn submit(
        &self,
        ctx: &RequestContext,
        request: SubmitRequest,
    ) -> ServiceResult<OperationResult>;

    /// Drops pooled connections and caches for the target; running operations are unaffected.
    async fn invalidate_target(&self, event: TargetInvalidation);
}

// ---------------------------------------------------------------------------------------
// Approvals (owner: Spec 06)
// ---------------------------------------------------------------------------------------

#[async_trait]
pub trait Approvals: Send + Sync + 'static {
    /// Stores the immutable intent and a pending approval. Callable with `approvals:request`.
    async fn request(
        &self,
        ctx: &RequestContext,
        intent: OperationIntent,
        policy: &PolicyEvaluation,
    ) -> ServiceResult<ApprovalSummary>;

    /// Human decision: interactive browser session of the owner with `approvals:decide`.
    /// `request.intent_digest` must match the stored intent.
    async fn decide(
        &self,
        ctx: &RequestContext,
        id: ApprovalId,
        request: ApprovalDecisionRequest,
    ) -> ServiceResult<ApprovalSummary>;

    async fn get(&self, ctx: &RequestContext, id: ApprovalId) -> ServiceResult<ApprovalSummary>;

    async fn list(
        &self,
        ctx: &RequestContext,
        status: Option<crate::public::operations::ApprovalStatus>,
        page: PageRequest,
    ) -> ServiceResult<Page<ApprovalSummary>>;

    /// Atomically consumes an approved, unexpired, matching approval at most once and returns
    /// the single-use authority. A consumed approval is never reset, even after an unknown
    /// outcome. Errors are `approval_required` with a reason (missing, pending, expired,
    /// consumed, rejected, binding mismatch).
    async fn consume(
        &self,
        ctx: &RequestContext,
        id: ApprovalId,
        intent: &OperationIntent,
        target: &AuthorizedTarget,
        policy: &PolicyEvaluation,
    ) -> ServiceResult<ExecutionAuthority>;
}

impl From<AuthorityError> for crate::public::operations::RefusalReason {
    fn from(error: AuthorityError) -> Self {
        error.refusal_reason()
    }
}

// ---------------------------------------------------------------------------------------
// Audit (owner: Spec 06)
// ---------------------------------------------------------------------------------------

/// Handle for one durable operation record, returned by `Audit::begin_operation`.
#[derive(Debug, PartialEq, Eq)]
pub struct AuditTicket {
    record_id: AuditRecordId,
    operation_id: OperationId,
}

impl AuditTicket {
    /// For audit adapters only.
    pub fn new(record_id: AuditRecordId, operation_id: OperationId) -> Self {
        Self {
            record_id,
            operation_id,
        }
    }

    pub fn record_id(&self) -> AuditRecordId {
        self.record_id
    }

    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }
}

/// Counts for startup reconciliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReconcileReport {
    pub marked_unknown: u64,
}

#[async_trait]
pub trait Audit: Send + Sync + 'static {
    /// Durably records intent with `SubmissionPending`. If the record cannot be made durable
    /// this returns `unavailable` and the caller must not submit a mutation.
    async fn begin_operation(
        &self,
        ctx: &RequestContext,
        intent: &OperationIntent,
        authority: &ExecutionAuthority,
    ) -> ServiceResult<AuditTicket>;

    /// Records that the exec request is being sent (state `Submitted`). Called by the executor
    /// immediately before submission; after this returns, a lost result is `unknown`.
    async fn record_submitted(
        &self,
        ctx: &RequestContext,
        ticket: &AuditTicket,
    ) -> ServiceResult<()>;

    /// Records the final outcome. Output text is not stored by default.
    async fn record_outcome(
        &self,
        ctx: &RequestContext,
        ticket: AuditTicket,
        state: SubmissionState,
        outcome: &OperationOutcome,
    ) -> ServiceResult<()>;

    /// Records refusals and authorization failures without foreign server metadata.
    async fn record_event(
        &self,
        ctx: &RequestContext,
        action: AuditAction,
        server_id: Option<ServerId>,
    ) -> ServiceResult<()>;

    async fn list_owned(
        &self,
        ctx: &RequestContext,
        query: AuditQuery,
    ) -> ServiceResult<Page<AuditRecord>>;

    /// Startup recovery: unresolved dispatches become `unknown_after_submission`; nothing is
    /// replayed.
    async fn reconcile_unresolved(&self, ctx: &CallContext) -> ServiceResult<ReconcileReport>;
}

/// Bundle of service handles a composed runtime passes to protocol adapters. Holds trait
/// objects only, so adapters stay independent of implementations.
#[derive(Clone)]
pub struct Services {
    pub inventory: Arc<dyn Inventory>,
    pub targets: Arc<dyn TargetResolver>,
    pub credentials: Arc<dyn CredentialStore>,
    pub identity: Arc<dyn Identity>,
    pub executor: Arc<dyn Executor>,
    pub approvals: Arc<dyn Approvals>,
    pub audit: Arc<dyn Audit>,
}
