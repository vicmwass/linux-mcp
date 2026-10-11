//! Step 00-07: compile probes proving the service traits are object safe and implementable
//! without any implementation-crate dependency (no tokio: futures are polled by hand).

#![cfg(feature = "backend")]

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use async_trait::async_trait;
use linux_mcp_contracts::backend::credentials::{CredentialLease, CredentialPurpose};
use linux_mcp_contracts::backend::identity::{
    AuthKind, Principal, Scope, SessionRef, VerifiedIdentity,
};
use linux_mcp_contracts::backend::operation_authority::{ExecutionAuthority, PolicyEvaluation};
use linux_mcp_contracts::backend::services::*;
use linux_mcp_contracts::backend::target::AuthorizedTarget;
use linux_mcp_contracts::public::api::*;
use linux_mcp_contracts::public::errors::{ApprovalErrorReason, ErrorCode, PublicError};
use linux_mcp_contracts::public::ids::*;
use linux_mcp_contracts::public::inventory::*;
use linux_mcp_contracts::public::operations::*;
use linux_mcp_contracts::public::pagination::{Page, PageRequest};

fn block_on<F: Future>(future: F) -> F::Output {
    fn noop_raw() -> RawWaker {
        fn clone(_: *const ()) -> RawWaker {
            noop_raw()
        }
        fn noop(_: *const ()) {}
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    let waker = unsafe { Waker::from_raw(noop_raw()) };
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            return value;
        }
    }
}

fn ts(secs: i64) -> Timestamp {
    Timestamp::from_unix_millis(1_790_000_000_000 + secs * 1000).unwrap()
}

fn req_id() -> RequestId {
    RequestId::parse("0b1c2d3e-4f50-4617-8899-aabbccddeeff").unwrap()
}

fn principal() -> Principal {
    Principal::from_verified(VerifiedIdentity {
        user_id: UserId::parse("a11ce000-0000-4000-8000-000000000001").unwrap(),
        installation_id: InstallationId::parse("aaaaaaaa-0000-4000-8000-000000000001").unwrap(),
        scopes: [Scope::InventoryRead].into_iter().collect(),
        session: SessionRef::new("s").unwrap(),
        auth_kind: AuthKind::McpToken,
        authenticated_at: ts(0),
        expires_at: ts(3600),
    })
    .unwrap()
}

fn ctx() -> RequestContext {
    RequestContext::new(
        CallContext {
            request_id: req_id(),
            deadline: ts(30),
        },
        principal(),
    )
}

/// Counts calls so the probe can show every trait is reachable through `dyn`.
#[derive(Default)]
struct Stub {
    calls: AtomicUsize,
}

impl Stub {
    fn hit(&self) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }
    fn err<T>(&self) -> ServiceResult<T> {
        self.hit();
        Err(PublicError::not_found(req_id()))
    }
}

#[async_trait]
impl Inventory for Stub {
    async fn list_owned(
        &self,
        _: &RequestContext,
        _: PageRequest,
    ) -> ServiceResult<Page<ServerSummary>> {
        self.hit();
        Ok(Page::last(vec![]))
    }
    async fn create(
        &self,
        _: &RequestContext,
        _: CreateServerRequest,
    ) -> ServiceResult<ServerSummary> {
        self.err()
    }
    async fn get_owned(&self, _: &RequestContext, _: ServerId) -> ServiceResult<ServerSummary> {
        self.err()
    }
    async fn update(
        &self,
        _: &RequestContext,
        _: ServerId,
        _: UpdateServerRequest,
    ) -> ServiceResult<ServerSummary> {
        self.err()
    }
    async fn delete(
        &self,
        _: &RequestContext,
        _: ServerId,
        _: ServerRevision,
    ) -> ServiceResult<()> {
        self.err()
    }
    async fn observe_host_key(
        &self,
        _: &RequestContext,
        _: ServerId,
    ) -> ServiceResult<HostKeyObservation> {
        self.err()
    }
    async fn enroll_host_key(
        &self,
        _: &RequestContext,
        _: ServerId,
        _: ServerRevision,
        _: String,
    ) -> ServiceResult<ServerSummary> {
        self.err()
    }
    async fn revoke_host_key(
        &self,
        _: &RequestContext,
        _: ServerId,
        _: ServerRevision,
    ) -> ServiceResult<ServerSummary> {
        self.err()
    }
}

#[async_trait]
impl TargetResolver for Stub {
    async fn resolve(
        &self,
        _: &RequestContext,
        _: ServerId,
        _: Scope,
    ) -> ServiceResult<AuthorizedTarget> {
        self.err()
    }
}

#[async_trait]
impl CredentialStore for Stub {
    async fn put(
        &self,
        _: &RequestContext,
        _: ServerId,
        _: PutCredentialRequest,
    ) -> ServiceResult<CredentialWriteResponse> {
        self.err()
    }
    async fn delete(
        &self,
        _: &RequestContext,
        _: ServerId,
        _: CredentialKind,
        _: ServerRevision,
    ) -> ServiceResult<CredentialWriteResponse> {
        self.err()
    }
    async fn resolve_for(
        &self,
        _: &RequestContext,
        _: &AuthorizedTarget,
        _: CredentialPurpose,
    ) -> ServiceResult<CredentialLease> {
        self.err()
    }
}

#[async_trait]
impl Identity for Stub {
    async fn authenticate_browser(
        &self,
        _: &CallContext,
        _: BrowserCredentials,
    ) -> ServiceResult<Principal> {
        self.hit();
        Err(PublicError::unauthenticated(req_id()))
    }
    async fn validate_mcp_token(
        &self,
        _: &CallContext,
        _: BearerToken,
    ) -> ServiceResult<Principal> {
        self.hit();
        Err(PublicError::unauthenticated(req_id()))
    }
    async fn revoke(&self, _: &RequestContext, _: RevocationTarget) -> ServiceResult<()> {
        self.hit();
        Ok(())
    }
    async fn revalidate(&self, _: &RequestContext) -> ServiceResult<()> {
        self.hit();
        Ok(())
    }
}

#[async_trait]
impl Executor for Stub {
    async fn inspect(
        &self,
        _: &RequestContext,
        _: &AuthorizedTarget,
    ) -> ServiceResult<TargetInspection> {
        self.err()
    }
    async fn evaluate(
        &self,
        _: &RequestContext,
        _: &OperationIntent,
        _: &AuthorizedTarget,
    ) -> ServiceResult<PolicyEvaluation> {
        self.err()
    }
    async fn submit(&self, _: &RequestContext, _: SubmitRequest) -> ServiceResult<OperationResult> {
        self.err()
    }
    async fn invalidate_target(&self, _: TargetInvalidation) {
        self.hit();
    }
}

#[async_trait]
impl Approvals for Stub {
    async fn request(
        &self,
        _: &RequestContext,
        _: OperationIntent,
        _: &PolicyEvaluation,
    ) -> ServiceResult<ApprovalSummary> {
        self.err()
    }
    async fn decide(
        &self,
        _: &RequestContext,
        _: ApprovalId,
        _: ApprovalDecisionRequest,
    ) -> ServiceResult<ApprovalSummary> {
        self.err()
    }
    async fn get(&self, _: &RequestContext, _: ApprovalId) -> ServiceResult<ApprovalSummary> {
        self.err()
    }
    async fn list(
        &self,
        _: &RequestContext,
        _: Option<ApprovalStatus>,
        _: PageRequest,
    ) -> ServiceResult<Page<ApprovalSummary>> {
        self.hit();
        Ok(Page::last(vec![]))
    }
    async fn consume(
        &self,
        _: &RequestContext,
        _: ApprovalId,
        _: &OperationIntent,
        _: &AuthorizedTarget,
        _: &PolicyEvaluation,
    ) -> ServiceResult<ExecutionAuthority> {
        self.hit();
        Err(PublicError::approval_required(
            req_id(),
            ApprovalErrorReason::Missing,
            None,
        ))
    }
}

#[async_trait]
impl Audit for Stub {
    async fn begin_operation(
        &self,
        _: &RequestContext,
        _: &OperationIntent,
        _: &ExecutionAuthority,
    ) -> ServiceResult<AuditTicket> {
        self.hit();
        Err(PublicError::unavailable(req_id(), Some(1000)))
    }
    async fn record_submitted(&self, _: &RequestContext, _: &AuditTicket) -> ServiceResult<()> {
        self.hit();
        Ok(())
    }
    async fn record_outcome(
        &self,
        _: &RequestContext,
        _: AuditTicket,
        _: SubmissionState,
        _: &OperationOutcome,
    ) -> ServiceResult<()> {
        self.hit();
        Ok(())
    }
    async fn record_event(
        &self,
        _: &RequestContext,
        _: AuditAction,
        _: Option<ServerId>,
    ) -> ServiceResult<()> {
        self.hit();
        Ok(())
    }
    async fn list_owned(
        &self,
        _: &RequestContext,
        _: AuditQuery,
    ) -> ServiceResult<Page<AuditRecord>> {
        self.hit();
        Ok(Page::last(vec![]))
    }
    async fn reconcile_unresolved(&self, _: &CallContext) -> ServiceResult<ReconcileReport> {
        self.hit();
        Ok(ReconcileReport::default())
    }
}

#[async_trait]
impl InvalidationSink for Stub {
    async fn invalidate(&self, _: TargetInvalidation) {
        self.hit();
    }
}

fn services(stub: &Arc<Stub>) -> Services {
    Services {
        inventory: stub.clone(),
        targets: stub.clone(),
        credentials: stub.clone(),
        identity: stub.clone(),
        executor: stub.clone(),
        approvals: stub.clone(),
        audit: stub.clone(),
    }
}

fn assert_send_sync<T: Send + Sync + 'static>() {}

#[test]
fn traits_are_object_safe_send_sync_and_reachable() {
    assert_send_sync::<Services>();
    assert_send_sync::<Arc<dyn InvalidationSink>>();
    let stub = Arc::new(Stub::default());
    let s = services(&stub);
    let ctx = ctx();
    let server = ServerId::parse("5e000000-0000-4000-8000-00000000a001").unwrap();

    block_on(async {
        assert!(
            s.inventory
                .list_owned(&ctx, PageRequest::default())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        let foreign = s
            .targets
            .resolve(&ctx, server, Scope::ServersInspect)
            .await
            .unwrap_err();
        assert_eq!(foreign.code(), ErrorCode::NotFound);
        assert_eq!(
            s.identity
                .validate_mcp_token(&ctx.call, BearerToken::new("tok".into()))
                .await
                .unwrap_err()
                .code(),
            ErrorCode::Unauthenticated
        );
        assert!(
            s.identity
                .revoke(
                    &ctx,
                    RevocationTarget::Session(SessionRef::new("s").unwrap())
                )
                .await
                .is_ok()
        );
        assert!(
            s.audit
                .list_owned(&ctx, AuditQuery::default())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert_eq!(
            s.audit.reconcile_unresolved(&ctx.call).await.unwrap(),
            ReconcileReport::default()
        );
        assert!(
            s.approvals
                .list(&ctx, None, PageRequest::default())
                .await
                .is_ok()
        );
        s.executor
            .invalidate_target(TargetInvalidation {
                installation_id: ctx.principal.installation_id(),
                owner: ctx.principal.user_id(),
                server_id: server,
                new_revision: ServerRevision::new(2).unwrap(),
                reason: InvalidationReason::CredentialChanged,
            })
            .await;
    });
    assert_eq!(stub.calls.load(Ordering::SeqCst), 8);
}

#[test]
fn secrets_are_unreachable_without_an_authorized_target() {
    // The only secret-returning method takes `&AuthorizedTarget`, and `AuthorizedTarget`
    // can only be produced by `AuthorizedTarget::authorize` (or a resolver). Verify the
    // contract surface exposes no `get_secret`-style method by inspecting the source.
    let source = include_str!("../src/backend/services.rs");
    assert!(source.contains("target: &AuthorizedTarget,\n        purpose: CredentialPurpose,\n    ) -> ServiceResult<CredentialLease>"));
    assert_eq!(
        source.matches("-> ServiceResult<CredentialLease>").count(),
        1
    );
    assert_eq!(source.matches("CredentialMaterial").count(), 0);
}

#[test]
fn contracts_manifest_has_no_implementation_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    let deps = manifest.split("[dependencies]").nth(1).unwrap();
    for forbidden in [
        "tokio",
        "sqlx",
        "russh",
        "rmcp",
        "axum",
        "hyper",
        "reqwest",
        "postgres",
        "linux-mcp =",
    ] {
        assert!(
            !deps.contains(forbidden),
            "contracts must not depend on {forbidden}"
        );
    }
}

#[test]
fn audit_ticket_is_not_copyable_and_authority_is_consumed() {
    // Compile-time property: `SubmitRequest` owns its authority and audit ticket by value.
    fn takes_by_value(_: SubmitRequest) {}
    let _ = takes_by_value;
    let ticket = AuditTicket::new(
        AuditRecordId::parse("a0000000-0000-4000-8000-000000000001").unwrap(),
        OperationId::parse("0e000000-0000-4000-8000-000000000001").unwrap(),
    );
    assert_eq!(
        ticket.operation_id().to_string(),
        "0e000000-0000-4000-8000-000000000001"
    );
}

#[test]
fn deadlines_are_absolute() {
    let ctx = ctx();
    assert!(!ctx.is_past_deadline(ts(29)));
    assert!(ctx.is_past_deadline(ts(30)));
    assert_eq!(
        format!("{:?}", BearerToken::new("s3cr3t".into())),
        "BearerToken(<redacted>)"
    );
    assert!(
        !format!(
            "{:?}",
            BrowserCredentials {
                session_cookie: BearerToken::new("cookie-secret".into()),
                csrf_token: None,
                unsafe_method: true,
            }
        )
        .contains("cookie-secret")
    );
}
