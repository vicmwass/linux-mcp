//! In-memory implementations of every service trait (`World`).
//!
//! Deterministic: IDs come from a counter, time from a controlled [`Clock`], and network
//! behavior from per-server [`ExecScript`]s. State sits behind one `std::sync::Mutex`, so
//! approval consumption and revision updates are atomic across threads. No external service,
//! network, filesystem or real secret is touched; secrets are fake markers.
//!
//! **This simulates the contract, not production guarantees.** It proves consumers handle the
//! contract's outcomes; it does not prove database transactionality, OAuth validation or SSH
//! behavior. It is a test double, never an authentication bypass: tokens are plain map entries
//! this module mints, and the module is compiled only with the `test-support` feature.
//!
//! All futures complete without waiting, so [`block_on`] can drive them without a runtime.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::future::Future;
use std::pin::pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Wake, Waker};

use async_trait::async_trait;
use serde_json::Value;

use crate::backend::credentials::{
    CredentialLease, CredentialMaterial, CredentialPurpose, CredentialRef,
};
use crate::backend::identity::{
    AuthKind, IdentityError, Principal, Scope, SessionRef, VerifiedIdentity,
};
use crate::backend::operation_authority::{
    ApprovalEvidence, AuthorityError, ExecutionAuthority, PolicyEvaluation,
};
use crate::backend::services::*;
use crate::backend::target::{
    AuthorizedTarget, HostTrustRef, NetworkTarget, ResolvedTarget, TargetCacheKey, TargetError,
};
use crate::canonical::IntentDigest;
use crate::public::api::{
    ApprovalDecision, ApprovalDecisionRequest, AuditAction, AuditOutcomeStatus, AuditQuery,
    AuditRecord, CredentialInput, CredentialWriteAction, CredentialWriteResponse,
    HostKeyObservation, PutCredentialRequest,
};
use crate::public::errors::{ApprovalErrorReason, PublicError, ValidationError, ViolationRule};
use crate::public::ids::*;
use crate::public::inventory::*;
use crate::public::operations::*;
use crate::public::pagination::{Page, PageCursor, PageRequest};

/// Drives a future that never pends (all `World` futures) without an async runtime.
pub fn block_on<F: Future>(future: F) -> F::Output {
    struct Noop;
    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            return value;
        }
    }
}

/// Controlled clock shared by a world and its tests.
#[derive(Debug, Clone)]
pub struct Clock(Arc<AtomicI64>);

impl Clock {
    pub const START_MILLIS: i64 = 1_791_000_000_000;

    pub fn new() -> Self {
        Self(Arc::new(AtomicI64::new(Self::START_MILLIS)))
    }

    pub fn now(&self) -> Timestamp {
        Timestamp::from_unix_millis(self.0.load(Ordering::SeqCst)).expect("clock in range")
    }

    pub fn advance_millis(&self, millis: i64) {
        self.0.fetch_add(millis, Ordering::SeqCst);
    }

    pub fn advance_secs(&self, secs: i64) {
        self.advance_millis(secs * 1000);
    }

    fn reset(&self) {
        self.0.store(Self::START_MILLIS, Ordering::SeqCst);
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

/// Scripted network/executor behavior for one server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecScript {
    /// The remote command runs and reports this exit code and output.
    Succeed {
        exit_code: u32,
        stdout: String,
        stderr: String,
    },
    /// Time out while connecting: nothing is submitted.
    TimeoutBeforeSubmit,
    /// Time out after the exec request was sent.
    TimeoutAfterSubmit,
    /// The connection drops after the exec request was sent.
    DisconnectAfterSubmit,
}

impl ExecScript {
    pub fn ok() -> Self {
        ExecScript::Succeed {
            exit_code: 0,
            stdout: "ok\n".into(),
            stderr: String::new(),
        }
    }
}

/// One simulated submission to a (fake) remote host. Proves what would have run and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecAttempt {
    pub sequence: u64,
    pub owner: UserId,
    pub server_id: ServerId,
    pub server_revision: ServerRevision,
    pub operation_id: OperationId,
    pub tool: ToolId,
    pub hostname: Hostname,
    pub port: SshPort,
    pub ssh_username: SshUsername,
    pub host_fingerprint: String,
    pub credential_id: CredentialId,
    pub cache_key: TargetCacheKey,
}

/// Fields for seeding a server directly (bypasses the Inventory API, for scenarios).
#[derive(Debug, Clone)]
pub struct SeedServer {
    pub id: ServerId,
    pub owner: UserId,
    pub display_name: &'static str,
    pub hostname: &'static str,
    pub port: u16,
    pub username: &'static str,
    pub environment: Environment,
    pub revision: u64,
    pub enabled: bool,
    pub approval_mode: ApprovalMode,
    /// Fingerprint the fake host presents.
    pub host_fingerprint: String,
    /// Whether that fingerprint is already verified for this server.
    pub verified: bool,
}

struct CredRec {
    cref: CredentialRef,
    secret: Vec<u8>,
    updated_at: Timestamp,
}

struct TrustRec {
    id: HostTrustId,
    status: HostTrustStatus,
    fingerprint: Option<String>,
    verified_at: Option<Timestamp>,
}

struct ServerRec {
    id: ServerId,
    owner: UserId,
    installation: InstallationId,
    display_name: DisplayName,
    hostname: Hostname,
    port: SshPort,
    username: SshUsername,
    environment: Environment,
    revision: ServerRevision,
    enabled: bool,
    approval_mode: ApprovalMode,
    created_at: Timestamp,
    updated_at: Timestamp,
    seq: u64,
    trust: TrustRec,
    creds: Vec<CredRec>,
    sudo_passwordless: bool,
    actual_fingerprint: String,
    observed: Option<(String, ServerRevision)>,
}

struct TokenRec {
    user: UserId,
    installation: InstallationId,
    scopes: BTreeSet<Scope>,
    session: SessionRef,
    kind: AuthKind,
    expires_at: Timestamp,
}

struct ApprovalRec {
    owner: UserId,
    intent: OperationIntent,
    digest: IntentDigest,
    status: ApprovalStatus,
    requested_at: Timestamp,
    decided_at: Option<Timestamp>,
    seq: u64,
}

struct AuditRec {
    id: AuditRecordId,
    owner: UserId,
    request_id: RequestId,
    action: AuditAction,
    operation_id: Option<OperationId>,
    server_id: Option<ServerId>,
    revision: Option<ServerRevision>,
    tool: Option<ToolId>,
    args: Option<Value>,
    approval_id: Option<ApprovalId>,
    submission: Option<SubmissionState>,
    outcome: Option<AuditOutcomeStatus>,
    refusal: Option<RefusalReason>,
    exit_code: Option<u32>,
    at: Timestamp,
}

#[derive(Default)]
struct State {
    counter: u64,
    servers: HashMap<ServerId, ServerRec>,
    tokens: HashMap<String, TokenRec>,
    revoked_sessions: HashSet<String>,
    revoked_users: HashSet<UserId>,
    approvals: HashMap<ApprovalId, ApprovalRec>,
    audit: Vec<AuditRec>,
    scripts: HashMap<ServerId, ExecScript>,
    attempts: Vec<ExecAttempt>,
    invalidations: Vec<TargetInvalidation>,
    pool: HashSet<TargetCacheKey>,
    audit_unavailable: bool,
}

struct Inner {
    installation: InstallationId,
    clock: Clock,
    state: Mutex<State>,
}

/// Deterministic in-memory world implementing all service traits. Cheap to clone.
#[derive(Clone)]
pub struct World {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("installation", &self.inner.installation)
            .finish_non_exhaustive()
    }
}

fn id_str(prefix: u32, n: u64) -> String {
    format!("{prefix:08x}-0000-4000-8000-{n:012x}")
}

impl World {
    pub fn new(installation: InstallationId) -> Self {
        Self {
            inner: Arc::new(Inner {
                installation,
                clock: Clock::new(),
                state: Mutex::new(State::default()),
            }),
        }
    }

    pub fn installation(&self) -> InstallationId {
        self.inner.installation
    }

    pub fn clock(&self) -> &Clock {
        &self.inner.clock
    }

    pub fn now(&self) -> Timestamp {
        self.inner.clock.now()
    }

    /// Clears all state and resets the clock and ID counter. Seed again afterwards.
    pub fn reset(&self) {
        *self.lock() = State::default();
        self.inner.clock.reset();
    }

    /// Bundle of trait objects over this world.
    pub fn services(&self) -> Services {
        Services {
            inventory: Arc::new(self.clone()),
            targets: Arc::new(self.clone()),
            credentials: Arc::new(self.clone()),
            identity: Arc::new(self.clone()),
            executor: Arc::new(self.clone()),
            approvals: Arc::new(self.clone()),
            audit: Arc::new(self.clone()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.inner.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn next_id<T>(
        &self,
        prefix: u32,
        parse: fn(&str) -> Result<T, crate::public::errors::InvalidValue>,
    ) -> T {
        let mut st = self.lock();
        st.counter += 1;
        parse(&id_str(prefix, st.counter)).expect("generated id is valid")
    }

    /// Next deterministic operation ID.
    pub fn next_operation_id(&self) -> OperationId {
        self.next_id(0x0e00_0000, OperationId::parse)
    }

    // ----- seeding and controls ---------------------------------------------------------

    /// Inserts a server directly. A verified server gets trust for `host_fingerprint`.
    pub fn seed_server(&self, seed: SeedServer) -> ServerId {
        let now = self.now();
        let trust_id = self.next_id(0x7000_0000, HostTrustId::parse);
        let mut st = self.lock();
        st.counter += 1;
        let seq = st.counter;
        let rec = ServerRec {
            id: seed.id,
            owner: seed.owner,
            installation: self.inner.installation,
            display_name: DisplayName::new(seed.display_name).expect("seed name"),
            hostname: Hostname::new(seed.hostname).expect("seed hostname"),
            port: SshPort::new(seed.port).expect("seed port"),
            username: SshUsername::new(seed.username).expect("seed username"),
            environment: seed.environment,
            revision: ServerRevision::new(seed.revision).expect("seed revision"),
            enabled: seed.enabled,
            approval_mode: seed.approval_mode,
            created_at: now,
            updated_at: now,
            seq,
            trust: TrustRec {
                id: trust_id,
                status: if seed.verified {
                    HostTrustStatus::Verified
                } else {
                    HostTrustStatus::Unenrolled
                },
                fingerprint: seed.verified.then(|| seed.host_fingerprint.clone()),
                verified_at: seed.verified.then_some(now),
            },
            creds: vec![],
            sudo_passwordless: false,
            actual_fingerprint: seed.host_fingerprint,
            observed: None,
        };
        st.servers.insert(seed.id, rec);
        seed.id
    }

    /// Adds a credential with a fake secret marker. Returns its reference.
    pub fn seed_credential(
        &self,
        server: ServerId,
        kind: CredentialKind,
        fake_secret: &str,
    ) -> CredentialRef {
        assert!(
            fake_secret.starts_with("FAKE-SECRET"),
            "fixtures use fake secret markers only"
        );
        let id = self.next_id(0xc000_0000, CredentialId::parse);
        let now = self.now();
        let mut st = self.lock();
        let rec = st.servers.get_mut(&server).expect("seeded server exists");
        let cref = CredentialRef {
            id,
            kind,
            owner: rec.owner,
            server_id: server,
            bound_at_revision: rec.revision,
            enabled: true,
        };
        rec.creds.retain(|c| c.cref.kind != kind);
        rec.creds.push(CredRec {
            cref: cref.clone(),
            secret: fake_secret.as_bytes().to_vec(),
            updated_at: now,
        });
        cref
    }

    /// Mints a fake bearer/session value for a user. Returns the string to present.
    pub fn issue_token(
        &self,
        user: UserId,
        scopes: &[Scope],
        kind: AuthKind,
        ttl_secs: i64,
    ) -> String {
        let n = {
            let mut st = self.lock();
            st.counter += 1;
            st.counter
        };
        let prefix = match kind {
            AuthKind::BrowserSession => "FAKE-SESSION",
            _ => "FAKE-TOKEN",
        };
        // Installation-qualified so two worlds never mint the same value.
        let tag = &self.inner.installation.to_string()[28..];
        let value = format!("{prefix}-{tag}-{n}");
        let expires_at = self
            .now()
            .checked_add_millis((ttl_secs * 1000) as u64)
            .expect("ttl in range");
        let rec = TokenRec {
            user,
            installation: self.inner.installation,
            scopes: scopes.iter().copied().collect(),
            session: SessionRef::new(format!("sess-{n}")).expect("session ref"),
            kind,
            expires_at,
        };
        self.lock().tokens.insert(value.clone(), rec);
        value
    }

    /// Expected CSRF value for a browser session value (test helper).
    pub fn csrf_for(&self, session_value: &str) -> Option<String> {
        self.lock()
            .tokens
            .get(session_value)
            .map(|t| format!("FAKE-CSRF-{}", t.session.as_str()))
    }

    /// Scripts the fake network/executor for one server.
    pub fn script(&self, server: ServerId, script: ExecScript) {
        self.lock().scripts.insert(server, script);
    }

    /// Changes the host key the fake host presents (simulates a changed/rotated host key).
    pub fn set_presented_host_key(&self, server: ServerId, fingerprint: &str) {
        if let Some(rec) = self.lock().servers.get_mut(&server) {
            rec.actual_fingerprint = fingerprint.to_owned();
        }
    }

    /// Makes durable audit unavailable (mutations must then be rejected).
    pub fn set_audit_unavailable(&self, unavailable: bool) {
        self.lock().audit_unavailable = unavailable;
    }

    /// Simulates a crash: leaves records pending, nothing else changes.
    pub fn attempts(&self) -> Vec<ExecAttempt> {
        self.lock().attempts.clone()
    }

    pub fn invalidations(&self) -> Vec<TargetInvalidation> {
        self.lock().invalidations.clone()
    }

    /// Connection-pool keys the fake executor currently holds.
    pub fn pool_keys(&self) -> Vec<TargetCacheKey> {
        self.lock().pool.iter().cloned().collect()
    }

    pub fn server_revision(&self, server: ServerId) -> Option<ServerRevision> {
        self.lock().servers.get(&server).map(|s| s.revision)
    }

    pub fn audit_actions(&self) -> Vec<AuditAction> {
        self.lock().audit.iter().map(|r| r.action).collect()
    }

    /// `true` if any stored audit record would expose a secret marker (should be never).
    pub fn audit_contains_secret_marker(&self) -> bool {
        self.lock()
            .audit
            .iter()
            .any(|r| format!("{:?}", r.args).contains("FAKE-SECRET"))
    }

    // ----- authentication of calls ------------------------------------------------------

    fn rid(ctx: &RequestContext) -> RequestId {
        ctx.request_id()
    }

    fn check_call(&self, call: &CallContext) -> Result<(), PublicError> {
        if self.now() >= call.deadline {
            Err(PublicError::timeout(call.request_id))
        } else {
            Ok(())
        }
    }

    fn check_principal(&self, st: &State, ctx: &RequestContext) -> Result<(), PublicError> {
        let rid = Self::rid(ctx);
        let p = &ctx.principal;
        if p.is_expired_at(self.now())
            || p.installation_id() != self.inner.installation
            || st.revoked_sessions.contains(p.session().as_str())
            || st.revoked_users.contains(&p.user_id())
        {
            return Err(PublicError::unauthenticated(rid));
        }
        Ok(())
    }

    /// Deadline, revocation, expiry, installation and scope checks for every call.
    fn auth(&self, ctx: &RequestContext, scope: Scope) -> Result<(), PublicError> {
        self.check_call(&ctx.call)?;
        {
            let st = self.lock();
            self.check_principal(&st, ctx)?;
        }
        ctx.principal
            .authorize(self.inner.installation, scope, self.now())
            .map_err(|e| match e {
                IdentityError::MissingScope(_) => PublicError::forbidden(Self::rid(ctx)),
                _ => PublicError::unauthenticated(Self::rid(ctx)),
            })
    }

    fn owned<'a>(
        st: &'a mut State,
        ctx: &RequestContext,
        id: ServerId,
    ) -> Result<&'a mut ServerRec, PublicError> {
        let me = ctx.principal.user_id();
        let install = ctx.principal.installation_id();
        match st.servers.get_mut(&id) {
            Some(rec) if rec.owner == me && rec.installation == install => Ok(rec),
            _ => Err(PublicError::not_found(Self::rid(ctx))),
        }
    }

    fn summary(rec: &ServerRec) -> ServerSummary {
        let cred = |kind: CredentialKind| {
            let found = rec.creds.iter().find(|c| c.cref.kind == kind);
            let state = match (found, kind, rec.sudo_passwordless) {
                (Some(_), _, _) => CredentialState::Provisioned,
                (None, CredentialKind::SudoPassword, true) => CredentialState::NotRequired,
                _ => CredentialState::Missing,
            };
            CredentialStatus {
                kind,
                state,
                credential_id: found.map(|c| c.cref.id),
                updated_at: found.map(|c| c.updated_at),
            }
        };
        ServerSummary {
            id: rec.id,
            display_name: rec.display_name.clone(),
            hostname: rec.hostname.clone(),
            port: rec.port,
            username: rec.username.clone(),
            environment: rec.environment,
            revision: rec.revision,
            enabled: rec.enabled,
            approval_mode: rec.approval_mode,
            host_trust: HostTrustSummary {
                status: rec.trust.status,
                fingerprint: rec.trust.fingerprint.clone(),
                verified_at: rec.trust.verified_at,
            },
            credentials: vec![
                cred(CredentialKind::SshPrivateKey),
                cred(CredentialKind::SudoPassword),
            ],
            created_at: rec.created_at,
            updated_at: rec.updated_at,
        }
    }

    fn bump(&self, st: &mut State, id: ServerId, reason: InvalidationReason) -> ServerRevision {
        let now = self.now();
        let rec = st.servers.get_mut(&id).expect("server exists");
        rec.revision = rec.revision.next().expect("revision bound");
        rec.updated_at = now;
        let event = TargetInvalidation {
            installation_id: rec.installation,
            owner: rec.owner,
            server_id: id,
            new_revision: rec.revision,
            reason,
        };
        let revision = rec.revision;
        Self::publish(st, event);
        revision
    }

    /// Records the event and evicts pooled connections for older revisions of that server.
    fn publish(st: &mut State, event: TargetInvalidation) {
        st.pool
            .retain(|k| !(k.server_id == event.server_id && k.revision < event.new_revision));
        if matches!(
            event.reason,
            InvalidationReason::Deleted | InvalidationReason::Disabled
        ) {
            st.pool.retain(|k| k.server_id != event.server_id);
        }
        st.invalidations.push(event);
    }

    fn paginate<T>(
        &self,
        ctx: &RequestContext,
        items: Vec<T>,
        page: &PageRequest,
    ) -> Result<Page<T>, PublicError> {
        let start = match &page.cursor {
            None => 0usize,
            Some(c) => c
                .as_str()
                .strip_prefix('c')
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| *n <= items.len())
                .ok_or_else(|| {
                    PublicError::validation(
                        Self::rid(ctx),
                        ValidationError::single("cursor", ViolationRule::InvalidFormat),
                    )
                })?,
        };
        let limit = page.limit.get() as usize;
        let end = (start + limit).min(items.len());
        let next = (end < items.len()).then(|| PageCursor::new(format!("c{end}")).expect("cursor"));
        Ok(Page {
            items: items.into_iter().skip(start).take(limit).collect(),
            next_cursor: next,
        })
    }

    // ----- policy ------------------------------------------------------------------------

    /// Mock policy: catastrophic deletes and critical risk are denied; the server's
    /// `ApprovalMode` decides which remaining risk levels need approval; the rest is allowed.
    fn policy_for(&self, intent: &OperationIntent, target: &AuthorizedTarget) -> PolicyEvaluation {
        let command = intent
            .arguments()
            .value()
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("");
        let verdict = if command.contains("rm -rf /") || intent.risk() == RiskLevel::Critical {
            PolicyVerdict::Denied {
                reason: RefusalReason::PolicyDenied,
            }
        } else if target.approval_mode().requires_approval(intent.risk()) {
            PolicyVerdict::RequiresApproval
        } else {
            PolicyVerdict::Allowed
        };
        PolicyEvaluation {
            intent_digest: intent.digest(),
            server_revision: target.revision(),
            verdict,
            policy_version: "mock-policy-v1".into(),
            evaluated_at: self.now(),
        }
    }

    /// Changing the approval policy needs an interactive browser session, so an agent token
    /// can never turn approvals off for itself.
    fn require_interactive_for_approval_mode(
        &self,
        ctx: &RequestContext,
        restricted: bool,
    ) -> Result<(), PublicError> {
        if restricted && ctx.principal.auth_kind() != AuthKind::BrowserSession {
            return Err(PublicError::forbidden(Self::rid(ctx)));
        }
        Ok(())
    }

    fn scope_for(risk: RiskLevel) -> Scope {
        if risk >= RiskLevel::Medium {
            Scope::ServersMutate
        } else {
            Scope::ServersInspect
        }
    }

    // ----- sync cores of trait methods ----------------------------------------------------

    fn resolve_inner(
        &self,
        ctx: &RequestContext,
        id: ServerId,
        scope: Scope,
    ) -> Result<AuthorizedTarget, PublicError> {
        self.auth(ctx, scope)?;
        let st = self.lock();
        let rec = st
            .servers
            .get(&id)
            .ok_or_else(|| PublicError::not_found(Self::rid(ctx)))?;
        let resolved = ResolvedTarget {
            owner: rec.owner,
            installation_id: rec.installation,
            server_id: rec.id,
            revision: rec.revision,
            enabled: rec.enabled,
            approval_mode: rec.approval_mode,
            network: NetworkTarget {
                hostname: rec.hostname.clone(),
                port: rec.port,
                username: rec.username.clone(),
            },
            host_trust: (rec.trust.status != HostTrustStatus::Unenrolled).then(|| HostTrustRef {
                id: rec.trust.id,
                status: rec.trust.status,
                fingerprint: rec.trust.fingerprint.clone().unwrap_or_default(),
                hostname: rec.hostname.clone(),
                port: rec.port,
            }),
            credentials: rec.creds.iter().map(|c| c.cref.clone()).collect(),
        };
        AuthorizedTarget::authorize(&ctx.principal, resolved, scope, self.now())
            .map_err(|e: TargetError| e.to_public(Self::rid(ctx)))
    }

    fn lease_inner(
        &self,
        ctx: &RequestContext,
        target: &AuthorizedTarget,
        purpose: CredentialPurpose,
    ) -> Result<CredentialLease, PublicError> {
        self.auth(ctx, Scope::ServersInspect)
            .or_else(|_| self.auth(ctx, Scope::ServersMutate))?;
        let st = self.lock();
        let rec = st
            .servers
            .get(&target.server_id())
            .filter(|r| r.owner == ctx.principal.user_id())
            .ok_or_else(|| PublicError::not_found(Self::rid(ctx)))?;
        target
            .ensure_current(rec.revision)
            .map_err(|e| e.to_public(Self::rid(ctx)))?;
        let cred = rec
            .creds
            .iter()
            .find(|c| c.cref.kind == purpose.kind())
            .ok_or_else(|| PublicError::unavailable(Self::rid(ctx), None))?;
        let material = CredentialMaterial::new(cred.cref.kind, cred.secret.clone());
        CredentialLease::issue(target, &cred.cref, purpose, material, self.now(), 30_000)
            .map_err(|_| PublicError::unavailable(Self::rid(ctx), None))
    }

    fn audit_push(
        &self,
        st: &mut State,
        ctx: &RequestContext,
        action: AuditAction,
    ) -> AuditRecordId {
        st.counter += 1;
        let id = AuditRecordId::parse(&id_str(0xa000_0000, st.counter)).expect("audit id");
        st.audit.push(AuditRec {
            id,
            owner: ctx.principal.user_id(),
            request_id: ctx.request_id(),
            action,
            operation_id: None,
            server_id: None,
            revision: None,
            tool: None,
            args: None,
            approval_id: None,
            submission: None,
            outcome: None,
            refusal: None,
            exit_code: None,
            at: self.now(),
        });
        id
    }

    fn audit_status(
        outcome: &OperationOutcome,
    ) -> (AuditOutcomeStatus, Option<RefusalReason>, Option<u32>) {
        match outcome {
            OperationOutcome::Succeeded { exit_status, .. } => (
                AuditOutcomeStatus::Succeeded,
                None,
                match exit_status {
                    ExitStatus::Code { code } => Some(*code),
                    ExitStatus::Signal { .. } => None,
                },
            ),
            OperationOutcome::Refused { reason, .. } => {
                (AuditOutcomeStatus::Refused, Some(*reason), None)
            }
            OperationOutcome::FailedBeforeSubmission { .. } => {
                (AuditOutcomeStatus::FailedBeforeSubmission, None, None)
            }
            OperationOutcome::UnknownAfterSubmission { .. } => {
                (AuditOutcomeStatus::UnknownAfterSubmission, None, None)
            }
        }
    }

    fn submit_inner(
        &self,
        ctx: &RequestContext,
        request: SubmitRequest,
    ) -> ServiceResult<OperationResult> {
        let SubmitRequest {
            intent,
            authority,
            audit,
        } = request;
        self.auth(ctx, Self::scope_for(intent.risk()))?;
        let started_at = self.now();
        let result = |submission: SubmissionState, outcome: OperationOutcome| OperationResult {
            operation_id: intent.operation_id(),
            server_id: intent.server_id(),
            server_revision: intent.server_revision(),
            tool: intent.tool().clone(),
            submission,
            outcome,
            approval_id: authority.approval_id(),
            started_at,
            finished_at: self.now(),
        };
        let finish = |submission: SubmissionState,
                      outcome: OperationOutcome,
                      ticket: AuditTicket|
         -> ServiceResult<OperationResult> {
            self.audit_outcome_inner(ctx, ticket, submission, &outcome)?;
            Ok(result(submission, outcome))
        };

        // Re-resolve the current target: ownership, enabled, trust and revision are rechecked.
        let target =
            match self.resolve_inner(ctx, intent.server_id(), Self::scope_for(intent.risk())) {
                Ok(t) => t,
                Err(_) => {
                    return finish(
                        SubmissionState::NotSubmitted,
                        OperationOutcome::refused(RefusalReason::TargetUnavailable, None),
                        audit,
                    );
                }
            };
        let policy = self.policy_for(&intent, &target);
        if let Err(error) = authority.recheck(&intent, &target, &policy, self.now()) {
            return finish(
                SubmissionState::NotSubmitted,
                OperationOutcome::refused(error.refusal_reason(), None),
                audit,
            );
        }
        let lease = match self.lease_inner(ctx, &target, CredentialPurpose::SshAuthentication) {
            Ok(l) => l,
            Err(_) => {
                return finish(
                    SubmissionState::NotSubmitted,
                    OperationOutcome::FailedBeforeSubmission {
                        cause: PreSubmissionFailure::CredentialUnavailable,
                    },
                    audit,
                );
            }
        };

        let (script, presented, trusted, attempt_base) = {
            let st = self.lock();
            let rec = st
                .servers
                .get(&intent.server_id())
                .expect("resolved server exists");
            (
                st.scripts
                    .get(&rec.id)
                    .cloned()
                    .unwrap_or_else(ExecScript::ok),
                rec.actual_fingerprint.clone(),
                rec.trust.fingerprint.clone(),
                (rec.hostname.clone(), rec.port, rec.username.clone()),
            )
        };
        if trusted.as_deref() != Some(presented.as_str()) {
            return finish(
                SubmissionState::NotSubmitted,
                OperationOutcome::FailedBeforeSubmission {
                    cause: PreSubmissionFailure::HostKeyRejected,
                },
                audit,
            );
        }
        if script == ExecScript::TimeoutBeforeSubmit {
            return finish(
                SubmissionState::NotSubmitted,
                OperationOutcome::transport_failure(
                    SubmissionState::NotSubmitted,
                    TransportFailure::Timeout,
                ),
                audit,
            );
        }

        // Submission boundary: durable record first, then the (simulated) exec request.
        if self.audit_submitted_inner(ctx, &audit).is_err() {
            return finish(
                SubmissionState::NotSubmitted,
                OperationOutcome::FailedBeforeSubmission {
                    cause: PreSubmissionFailure::AuditUnavailable,
                },
                audit,
            );
        }
        lease
            .with_secret(self.now(), |secret| debug_assert!(!secret.is_empty()))
            .map_err(|_| PublicError::unavailable(Self::rid(ctx), None))?;
        {
            let mut st = self.lock();
            st.pool.insert(target.cache_key());
            let sequence = st.attempts.len() as u64 + 1;
            st.attempts.push(ExecAttempt {
                sequence,
                owner: ctx.principal.user_id(),
                server_id: intent.server_id(),
                server_revision: target.revision(),
                operation_id: intent.operation_id(),
                tool: intent.tool().clone(),
                hostname: attempt_base.0,
                port: attempt_base.1,
                ssh_username: attempt_base.2,
                host_fingerprint: presented,
                credential_id: lease.credential_id(),
                cache_key: target.cache_key(),
            });
        }
        match script {
            ExecScript::Succeed {
                exit_code,
                stdout,
                stderr,
            } => finish(
                SubmissionState::Completed,
                OperationOutcome::Succeeded {
                    exit_status: ExitStatus::Code { code: exit_code },
                    stdout: BoundedOutput::from_bytes(
                        stdout.as_bytes(),
                        DEFAULT_OUTPUT_LIMIT_BYTES,
                    ),
                    stderr: BoundedOutput::from_bytes(
                        stderr.as_bytes(),
                        DEFAULT_OUTPUT_LIMIT_BYTES,
                    ),
                    verification: Verification::NotAttempted,
                },
                audit,
            ),
            ExecScript::TimeoutAfterSubmit => finish(
                SubmissionState::Submitted,
                OperationOutcome::transport_failure(
                    SubmissionState::Submitted,
                    TransportFailure::Timeout,
                ),
                audit,
            ),
            ExecScript::DisconnectAfterSubmit | ExecScript::TimeoutBeforeSubmit => finish(
                SubmissionState::Submitted,
                OperationOutcome::transport_failure(
                    SubmissionState::Submitted,
                    TransportFailure::Disconnected,
                ),
                audit,
            ),
        }
    }

    fn audit_submitted_inner(
        &self,
        ctx: &RequestContext,
        ticket: &AuditTicket,
    ) -> ServiceResult<()> {
        let mut st = self.lock();
        if st.audit_unavailable {
            return Err(PublicError::unavailable(Self::rid(ctx), Some(1000)));
        }
        let rec = st
            .audit
            .iter_mut()
            .find(|r| r.id == ticket.record_id() && r.owner == ctx.principal.user_id())
            .ok_or_else(|| PublicError::not_found(Self::rid(ctx)))?;
        rec.submission = Some(SubmissionState::Submitted);
        rec.action = AuditAction::OperationSubmitted;
        rec.at = self.now();
        Ok(())
    }

    fn audit_outcome_inner(
        &self,
        ctx: &RequestContext,
        ticket: AuditTicket,
        state: SubmissionState,
        outcome: &OperationOutcome,
    ) -> ServiceResult<()> {
        let mut st = self.lock();
        let (status, refusal, exit_code) = Self::audit_status(outcome);
        let action = match status {
            AuditOutcomeStatus::Refused => AuditAction::OperationRefused,
            AuditOutcomeStatus::UnknownAfterSubmission => AuditAction::OperationOutcomeUnknown,
            _ => AuditAction::OperationCompleted,
        };
        let at = self.now();
        let rec = st
            .audit
            .iter_mut()
            .find(|r| r.id == ticket.record_id() && r.owner == ctx.principal.user_id())
            .ok_or_else(|| PublicError::not_found(Self::rid(ctx)))?;
        rec.submission = Some(state);
        rec.outcome = Some(status);
        rec.refusal = refusal;
        rec.exit_code = exit_code;
        rec.action = action;
        rec.at = at;
        Ok(())
    }

    fn approval_summary(&self, rec: &ApprovalRec, id: ApprovalId) -> ApprovalSummary {
        ApprovalSummary {
            approval_id: id,
            status: rec.status,
            intent: rec.intent.clone(),
            intent_digest: rec.digest.clone(),
            requested_at: rec.requested_at,
            decided_at: rec.decided_at,
            expires_at: rec.intent.expires_at(),
        }
    }

    /// Lazily expires pending/approved approvals past their expiry.
    fn expire(&self, st: &mut State) {
        let now = self.now();
        for rec in st.approvals.values_mut() {
            if matches!(
                rec.status,
                ApprovalStatus::Pending | ApprovalStatus::Approved
            ) && now >= rec.intent.expires_at()
            {
                rec.status = ApprovalStatus::Expired;
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------------------

#[async_trait]
impl Inventory for World {
    async fn list_owned(
        &self,
        ctx: &RequestContext,
        page: PageRequest,
    ) -> ServiceResult<Page<ServerSummary>> {
        self.auth(ctx, Scope::InventoryRead)?;
        let st = self.lock();
        let mut mine: Vec<&ServerRec> = st
            .servers
            .values()
            .filter(|r| {
                r.owner == ctx.principal.user_id()
                    && r.installation == ctx.principal.installation_id()
            })
            .collect();
        mine.sort_by_key(|r| r.seq);
        let items = mine.into_iter().map(Self::summary).collect();
        self.paginate(ctx, items, &page)
    }

    async fn create(
        &self,
        ctx: &RequestContext,
        request: CreateServerRequest,
    ) -> ServiceResult<ServerSummary> {
        self.auth(ctx, Scope::InventoryWrite)?;
        self.require_interactive_for_approval_mode(ctx, !request.approval_mode.is_default())?;
        let id = self.next_id(0x5e00_0000, ServerId::parse);
        let trust_id = self.next_id(0x7000_0000, HostTrustId::parse);
        let now = self.now();
        let mut st = self.lock();
        st.counter += 1;
        let seq = st.counter;
        let rec = ServerRec {
            id,
            owner: ctx.principal.user_id(),
            installation: ctx.principal.installation_id(),
            display_name: request.display_name,
            hostname: request.hostname,
            port: request.port,
            username: request.username,
            environment: request.environment,
            revision: ServerRevision::INITIAL,
            enabled: request.enabled,
            approval_mode: request.approval_mode,
            created_at: now,
            updated_at: now,
            seq,
            trust: TrustRec {
                id: trust_id,
                status: HostTrustStatus::Unenrolled,
                fingerprint: None,
                verified_at: None,
            },
            creds: vec![],
            sudo_passwordless: false,
            actual_fingerprint: format!("SHA256:{:0<43}", format!("FakeHostKey{seq}")),
            observed: None,
        };
        let summary = Self::summary(&rec);
        st.servers.insert(id, rec);
        Ok(summary)
    }

    async fn get_owned(&self, ctx: &RequestContext, id: ServerId) -> ServiceResult<ServerSummary> {
        self.auth(ctx, Scope::InventoryRead)?;
        let mut st = self.lock();
        Ok(Self::summary(Self::owned(&mut st, ctx, id)?))
    }

    async fn update(
        &self,
        ctx: &RequestContext,
        id: ServerId,
        request: UpdateServerRequest,
    ) -> ServiceResult<ServerSummary> {
        self.auth(ctx, Scope::InventoryWrite)?;
        request
            .validate()
            .map_err(|e| PublicError::validation(Self::rid(ctx), e))?;
        self.require_interactive_for_approval_mode(ctx, request.changes_approval_policy())?;
        let mut st = self.lock();
        let rec = Self::owned(&mut st, ctx, id)?;
        if rec.revision != request.expected_revision {
            return Err(PublicError::revision_conflict(Self::rid(ctx), rec.revision));
        }
        let changes_target = request.changes_target();
        let disabled = request.enabled == Some(false) && rec.enabled;
        if let Some(v) = request.display_name {
            rec.display_name = v;
        }
        if let Some(v) = request.hostname {
            rec.hostname = v;
        }
        if let Some(v) = request.port {
            rec.port = v;
        }
        if let Some(v) = request.username {
            rec.username = v;
        }
        if let Some(v) = request.environment {
            rec.environment = v;
        }
        if let Some(v) = request.enabled {
            rec.enabled = v;
        }
        if let Some(v) = request.approval_mode {
            rec.approval_mode = v;
        }
        if changes_target {
            // New network target: previous host trust does not apply.
            rec.trust = TrustRec {
                id: rec.trust.id,
                status: HostTrustStatus::Unenrolled,
                fingerprint: None,
                verified_at: None,
            };
            rec.observed = None;
        }
        let reason = if disabled {
            InvalidationReason::Disabled
        } else if changes_target {
            InvalidationReason::HostTrustChanged
        } else {
            InvalidationReason::InventoryChanged
        };
        self.bump(&mut st, id, reason);
        Ok(Self::summary(st.servers.get(&id).expect("exists")))
    }

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: ServerId,
        expected_revision: ServerRevision,
    ) -> ServiceResult<()> {
        self.auth(ctx, Scope::InventoryWrite)?;
        let mut st = self.lock();
        let rec = Self::owned(&mut st, ctx, id)?;
        if rec.revision != expected_revision {
            return Err(PublicError::revision_conflict(Self::rid(ctx), rec.revision));
        }
        let revision = rec.revision.next().expect("revision bound");
        let event = TargetInvalidation {
            installation_id: rec.installation,
            owner: rec.owner,
            server_id: id,
            new_revision: revision,
            reason: InvalidationReason::Deleted,
        };
        st.servers.remove(&id);
        Self::publish(&mut st, event);
        Ok(())
    }

    async fn observe_host_key(
        &self,
        ctx: &RequestContext,
        id: ServerId,
    ) -> ServiceResult<HostKeyObservation> {
        self.auth(ctx, Scope::InventoryWrite)?;
        let mut st = self.lock();
        let rec = Self::owned(&mut st, ctx, id)?;
        let presented = rec.actual_fingerprint.clone();
        let differs = rec.trust.status == HostTrustStatus::Verified
            && rec.trust.fingerprint.as_deref() != Some(presented.as_str());
        let observed_at = self.now();
        let hostname = rec.hostname.clone();
        let port = rec.port;
        if differs {
            // A changed key blocks connections until explicitly reverified.
            rec.trust.status = HostTrustStatus::Mismatch;
            let revision = self.bump(&mut st, id, InvalidationReason::HostTrustChanged);
            st.servers.get_mut(&id).expect("exists").observed = Some((presented.clone(), revision));
            return Ok(HostKeyObservation {
                server_id: id,
                server_revision: revision,
                hostname,
                port,
                key_type: "ssh-ed25519".into(),
                fingerprint: presented,
                observed_at,
                differs_from_verified: true,
            });
        }
        if rec.trust.status == HostTrustStatus::Unenrolled
            || rec.trust.status == HostTrustStatus::Observed
        {
            // Observation is NOT trust.
            rec.trust.status = HostTrustStatus::Observed;
            rec.trust.fingerprint = Some(presented.clone());
        }
        rec.observed = Some((presented.clone(), rec.revision));
        let revision = rec.revision;
        Ok(HostKeyObservation {
            server_id: id,
            server_revision: revision,
            hostname,
            port,
            key_type: "ssh-ed25519".into(),
            fingerprint: presented,
            observed_at,
            differs_from_verified: false,
        })
    }

    async fn enroll_host_key(
        &self,
        ctx: &RequestContext,
        id: ServerId,
        expected_revision: ServerRevision,
        fingerprint: String,
    ) -> ServiceResult<ServerSummary> {
        self.auth(ctx, Scope::InventoryWrite)?;
        let mut st = self.lock();
        let rec = Self::owned(&mut st, ctx, id)?;
        if rec.revision != expected_revision {
            return Err(PublicError::revision_conflict(Self::rid(ctx), rec.revision));
        }
        // Enrollment requires the user-verified fingerprint to equal the latest observation.
        let observed = rec.observed.clone();
        if observed != Some((fingerprint.clone(), rec.revision)) {
            return Err(PublicError::validation(
                Self::rid(ctx),
                ValidationError::single("fingerprint", ViolationRule::NotAllowed),
            ));
        }
        rec.trust.status = HostTrustStatus::Verified;
        rec.trust.fingerprint = Some(fingerprint);
        rec.trust.verified_at = Some(self.now());
        rec.observed = None;
        self.bump(&mut st, id, InvalidationReason::HostTrustChanged);
        Ok(Self::summary(st.servers.get(&id).expect("exists")))
    }

    async fn revoke_host_key(
        &self,
        ctx: &RequestContext,
        id: ServerId,
        expected_revision: ServerRevision,
    ) -> ServiceResult<ServerSummary> {
        self.auth(ctx, Scope::InventoryWrite)?;
        let mut st = self.lock();
        let rec = Self::owned(&mut st, ctx, id)?;
        if rec.revision != expected_revision {
            return Err(PublicError::revision_conflict(Self::rid(ctx), rec.revision));
        }
        rec.trust = TrustRec {
            id: rec.trust.id,
            status: HostTrustStatus::Unenrolled,
            fingerprint: None,
            verified_at: None,
        };
        rec.observed = None;
        self.bump(&mut st, id, InvalidationReason::HostTrustChanged);
        Ok(Self::summary(st.servers.get(&id).expect("exists")))
    }
}

#[async_trait]
impl TargetResolver for World {
    async fn resolve(
        &self,
        ctx: &RequestContext,
        server_id: ServerId,
        scope: Scope,
    ) -> ServiceResult<AuthorizedTarget> {
        self.resolve_inner(ctx, server_id, scope)
    }
}

#[async_trait]
impl CredentialStore for World {
    async fn put(
        &self,
        ctx: &RequestContext,
        server_id: ServerId,
        request: PutCredentialRequest,
    ) -> ServiceResult<CredentialWriteResponse> {
        self.auth(ctx, Scope::CredentialsWrite)?;
        let now = self.now();
        let new_id = self.next_id(0xc000_0000, CredentialId::parse);
        let mut st = self.lock();
        let rec = Self::owned(&mut st, ctx, server_id)?;
        if rec.revision != request.expected_revision {
            return Err(PublicError::revision_conflict(Self::rid(ctx), rec.revision));
        }
        let kind = request.credential.kind();
        let existed = rec.creds.iter().any(|c| c.cref.kind == kind)
            || (kind == CredentialKind::SudoPassword && rec.sudo_passwordless);
        let secret_text = match &request.credential {
            // `WriteOnlySecret` is Serialize by design (clients send it); the mock reads it that way.
            CredentialInput::SshPrivateKey { private_key } => {
                Some(serde_json::to_value(private_key).expect("serializes"))
            }
            CredentialInput::SudoPassword { password } => {
                Some(serde_json::to_value(password).expect("serializes"))
            }
            CredentialInput::SudoPasswordless => None,
        };
        rec.creds.retain(|c| c.cref.kind != kind);
        let mut action = if existed {
            CredentialWriteAction::Rotated
        } else {
            CredentialWriteAction::Provisioned
        };
        let new_revision = rec.revision.next().expect("revision bound");
        let owner = rec.owner;
        match secret_text {
            Some(Value::String(text)) => {
                rec.sudo_passwordless =
                    rec.sudo_passwordless && kind != CredentialKind::SudoPassword;
                rec.creds.push(CredRec {
                    cref: CredentialRef {
                        id: new_id,
                        kind,
                        owner,
                        server_id,
                        bound_at_revision: new_revision,
                        enabled: true,
                    },
                    secret: text.into_bytes(),
                    updated_at: now,
                });
            }
            _ => {
                rec.sudo_passwordless = true;
                action = CredentialWriteAction::Removed;
            }
        }
        self.bump(&mut st, server_id, InvalidationReason::CredentialChanged);
        let rec = st.servers.get(&server_id).expect("exists");
        let status = Self::summary(rec)
            .credentials
            .into_iter()
            .find(|c| c.kind == kind)
            .expect("slot");
        Ok(CredentialWriteResponse {
            server_id,
            server_revision: rec.revision,
            action,
            credential: status,
        })
    }

    async fn delete(
        &self,
        ctx: &RequestContext,
        server_id: ServerId,
        kind: CredentialKind,
        expected_revision: ServerRevision,
    ) -> ServiceResult<CredentialWriteResponse> {
        self.auth(ctx, Scope::CredentialsWrite)?;
        let mut st = self.lock();
        let rec = Self::owned(&mut st, ctx, server_id)?;
        if rec.revision != expected_revision {
            return Err(PublicError::revision_conflict(Self::rid(ctx), rec.revision));
        }
        rec.creds.retain(|c| c.cref.kind != kind);
        if kind == CredentialKind::SudoPassword {
            rec.sudo_passwordless = false;
        }
        self.bump(&mut st, server_id, InvalidationReason::CredentialChanged);
        let rec = st.servers.get(&server_id).expect("exists");
        let status = Self::summary(rec)
            .credentials
            .into_iter()
            .find(|c| c.kind == kind)
            .expect("slot");
        Ok(CredentialWriteResponse {
            server_id,
            server_revision: rec.revision,
            action: CredentialWriteAction::Removed,
            credential: status,
        })
    }

    async fn resolve_for(
        &self,
        ctx: &RequestContext,
        target: &AuthorizedTarget,
        purpose: CredentialPurpose,
    ) -> ServiceResult<CredentialLease> {
        self.lease_inner(ctx, target, purpose)
    }
}

#[async_trait]
impl Identity for World {
    async fn authenticate_browser(
        &self,
        ctx: &CallContext,
        credentials: BrowserCredentials,
    ) -> ServiceResult<Principal> {
        self.check_call(ctx)?;
        let fail = || PublicError::unauthenticated(ctx.request_id);
        let st = self.lock();
        let rec = st
            .tokens
            .get(credentials.session_cookie.expose())
            .ok_or_else(fail)?;
        if rec.kind != AuthKind::BrowserSession || rec.installation != self.inner.installation {
            return Err(fail());
        }
        if credentials.unsafe_method {
            let expected = format!("FAKE-CSRF-{}", rec.session.as_str());
            if credentials.csrf_token.as_ref().map(|t| t.expose()) != Some(expected.as_str()) {
                return Err(fail());
            }
        }
        self.principal_from(&st, rec, ctx.request_id)
    }

    async fn validate_mcp_token(
        &self,
        ctx: &CallContext,
        token: BearerToken,
    ) -> ServiceResult<Principal> {
        self.check_call(ctx)?;
        let fail = || PublicError::unauthenticated(ctx.request_id);
        let st = self.lock();
        let rec = st.tokens.get(token.expose()).ok_or_else(fail)?;
        // Browser sessions never authorize an external agent.
        if !matches!(rec.kind, AuthKind::McpToken | AuthKind::DelegatedToken)
            || rec.installation != self.inner.installation
        {
            return Err(fail());
        }
        self.principal_from(&st, rec, ctx.request_id)
    }

    async fn revoke(&self, ctx: &RequestContext, target: RevocationTarget) -> ServiceResult<()> {
        self.check_call(&ctx.call)?;
        let mut st = self.lock();
        self.check_principal(&st, ctx)?;
        match target {
            RevocationTarget::Session(session) => {
                // Own sessions only.
                let owned = st
                    .tokens
                    .values()
                    .any(|t| t.session == session && t.user == ctx.principal.user_id());
                if !owned {
                    return Err(PublicError::not_found(Self::rid(ctx)));
                }
                st.revoked_sessions.insert(session.as_str().to_owned());
            }
            RevocationTarget::User {
                installation_id,
                user_id,
            } => {
                if installation_id != self.inner.installation || user_id != ctx.principal.user_id()
                {
                    return Err(PublicError::not_found(Self::rid(ctx)));
                }
                st.revoked_users.insert(user_id);
            }
        }
        Ok(())
    }

    async fn revalidate(&self, ctx: &RequestContext) -> ServiceResult<()> {
        self.check_call(&ctx.call)?;
        let st = self.lock();
        self.check_principal(&st, ctx)
    }
}

impl World {
    fn principal_from(
        &self,
        st: &State,
        rec: &TokenRec,
        request_id: RequestId,
    ) -> ServiceResult<Principal> {
        let fail = || PublicError::unauthenticated(request_id);
        if self.now() >= rec.expires_at
            || st.revoked_sessions.contains(rec.session.as_str())
            || st.revoked_users.contains(&rec.user)
        {
            return Err(fail());
        }
        Principal::from_verified(VerifiedIdentity {
            user_id: rec.user,
            installation_id: rec.installation,
            scopes: rec.scopes.clone(),
            session: rec.session.clone(),
            auth_kind: rec.kind,
            authenticated_at: self.now(),
            expires_at: rec.expires_at,
        })
        .map_err(|_| fail())
    }
}

#[async_trait]
impl Executor for World {
    async fn inspect(
        &self,
        ctx: &RequestContext,
        target: &AuthorizedTarget,
    ) -> ServiceResult<TargetInspection> {
        self.auth(ctx, Scope::ServersInspect)?;
        let st = self.lock();
        let rec = st
            .servers
            .get(&target.server_id())
            .filter(|r| r.owner == ctx.principal.user_id())
            .ok_or_else(|| PublicError::not_found(Self::rid(ctx)))?;
        target
            .ensure_current(rec.revision)
            .map_err(|e| e.to_public(Self::rid(ctx)))?;
        Ok(TargetInspection {
            server_id: rec.id,
            server_revision: rec.revision,
            distribution: "FakeLinux".into(),
            version: "1.0".into(),
            kernel: "0.0.0-fake".into(),
            architecture: "x86_64".into(),
            init_system: "Systemd".into(),
            package_manager: "Apt".into(),
            commands: vec!["id".into(), "ls".into(), "useradd".into()],
            inspected_at: self.now(),
        })
    }

    async fn evaluate(
        &self,
        ctx: &RequestContext,
        intent: &OperationIntent,
        target: &AuthorizedTarget,
    ) -> ServiceResult<PolicyEvaluation> {
        self.auth(ctx, Self::scope_for(intent.risk()))?;
        if intent.owner() != ctx.principal.user_id() || intent.server_id() != target.server_id() {
            return Err(PublicError::not_found(Self::rid(ctx)));
        }
        Ok(self.policy_for(intent, target))
    }

    async fn submit(
        &self,
        ctx: &RequestContext,
        request: SubmitRequest,
    ) -> ServiceResult<OperationResult> {
        self.submit_inner(ctx, request)
    }

    async fn invalidate_target(&self, event: TargetInvalidation) {
        Self::publish(&mut self.lock(), event);
    }
}

#[async_trait]
impl InvalidationSink for World {
    async fn invalidate(&self, event: TargetInvalidation) {
        Self::publish(&mut self.lock(), event);
    }
}

#[async_trait]
impl Approvals for World {
    async fn request(
        &self,
        ctx: &RequestContext,
        intent: OperationIntent,
        policy: &PolicyEvaluation,
    ) -> ServiceResult<ApprovalSummary> {
        self.auth(ctx, Scope::ApprovalsRequest)?;
        let rid = Self::rid(ctx);
        if intent.owner() != ctx.principal.user_id()
            || intent.installation_id() != ctx.principal.installation_id()
        {
            return Err(PublicError::not_found(rid));
        }
        if policy.verdict != PolicyVerdict::RequiresApproval
            || policy.intent_digest != intent.digest()
        {
            return Err(PublicError::validation(
                rid,
                ValidationError::single("operation", ViolationRule::NotAllowed),
            ));
        }
        let id = self.next_id(0xa990_0000, ApprovalId::parse);
        let mut st = self.lock();
        st.counter += 1;
        let seq = st.counter;
        let rec = ApprovalRec {
            owner: intent.owner(),
            digest: intent.digest(),
            intent,
            status: ApprovalStatus::Pending,
            requested_at: self.now(),
            decided_at: None,
            seq,
        };
        let summary = self.approval_summary(&rec, id);
        st.approvals.insert(id, rec);
        let audit_id = self.audit_push(&mut st, ctx, AuditAction::ApprovalRequested);
        if let Some(r) = st.audit.iter_mut().find(|r| r.id == audit_id) {
            r.approval_id = Some(id);
            r.operation_id = Some(summary.intent.operation_id());
            r.server_id = Some(summary.intent.server_id());
            r.tool = Some(summary.intent.tool().clone());
        }
        Ok(summary)
    }

    async fn decide(
        &self,
        ctx: &RequestContext,
        id: ApprovalId,
        request: ApprovalDecisionRequest,
    ) -> ServiceResult<ApprovalSummary> {
        self.auth(ctx, Scope::ApprovalsDecide)?;
        let rid = Self::rid(ctx);
        let now = self.now();
        let mut st = self.lock();
        self.expire(&mut st);
        let rec = st
            .approvals
            .get_mut(&id)
            .filter(|r| r.owner == ctx.principal.user_id())
            .ok_or_else(|| PublicError::not_found(rid))?;
        if rec.status != ApprovalStatus::Pending {
            return Err(PublicError::approval_required(
                rid,
                match rec.status {
                    ApprovalStatus::Consumed => ApprovalErrorReason::Consumed,
                    ApprovalStatus::Expired => ApprovalErrorReason::Expired,
                    ApprovalStatus::Rejected => ApprovalErrorReason::Rejected,
                    _ => ApprovalErrorReason::BindingMismatch,
                },
                Some(id),
            ));
        }
        if request.intent_digest != rec.digest {
            return Err(PublicError::validation(
                rid,
                ValidationError::single("intent_digest", ViolationRule::NotAllowed),
            ));
        }
        match request.decision {
            ApprovalDecision::Reject => rec.status = ApprovalStatus::Rejected,
            ApprovalDecision::Approve => {
                // Only an interactive browser session of the owner can approve.
                ApprovalEvidence::record(id, &ctx.principal, &rec.intent, now).map_err(
                    |e| match e {
                        AuthorityError::Expired => PublicError::approval_required(
                            rid,
                            ApprovalErrorReason::Expired,
                            Some(id),
                        ),
                        _ => PublicError::forbidden(rid),
                    },
                )?;
                rec.status = ApprovalStatus::Approved;
            }
        }
        rec.decided_at = Some(now);
        let rec = st.approvals.get(&id).expect("exists");
        let summary = self.approval_summary(rec, id);
        self.audit_push(&mut st, ctx, AuditAction::ApprovalDecided);
        Ok(summary)
    }

    async fn get(&self, ctx: &RequestContext, id: ApprovalId) -> ServiceResult<ApprovalSummary> {
        self.auth(ctx, Scope::ApprovalsRequest)
            .or_else(|_| self.auth(ctx, Scope::ApprovalsDecide))?;
        let mut st = self.lock();
        self.expire(&mut st);
        let rec = st
            .approvals
            .get(&id)
            .filter(|r| r.owner == ctx.principal.user_id())
            .ok_or_else(|| PublicError::not_found(Self::rid(ctx)))?;
        Ok(self.approval_summary(rec, id))
    }

    async fn list(
        &self,
        ctx: &RequestContext,
        status: Option<ApprovalStatus>,
        page: PageRequest,
    ) -> ServiceResult<Page<ApprovalSummary>> {
        self.auth(ctx, Scope::ApprovalsRequest)
            .or_else(|_| self.auth(ctx, Scope::ApprovalsDecide))?;
        let mut st = self.lock();
        self.expire(&mut st);
        let mut mine: Vec<(&ApprovalId, &ApprovalRec)> = st
            .approvals
            .iter()
            .filter(|(_, r)| {
                r.owner == ctx.principal.user_id() && status.is_none_or(|s| r.status == s)
            })
            .collect();
        mine.sort_by_key(|(_, r)| r.seq);
        let items = mine
            .into_iter()
            .map(|(id, r)| self.approval_summary(r, *id))
            .collect();
        self.paginate(ctx, items, &page)
    }

    async fn consume(
        &self,
        ctx: &RequestContext,
        id: ApprovalId,
        intent: &OperationIntent,
        target: &AuthorizedTarget,
        policy: &PolicyEvaluation,
    ) -> ServiceResult<ExecutionAuthority> {
        self.auth(ctx, Scope::ServersMutate)?;
        let rid = Self::rid(ctx);
        let now = self.now();
        let reject =
            |reason: ApprovalErrorReason| PublicError::approval_required(rid, reason, Some(id));
        // One lock for check-and-mark: concurrent consumers get exactly one success.
        let mut st = self.lock();
        let rec = st
            .approvals
            .get_mut(&id)
            .filter(|r| r.owner == ctx.principal.user_id())
            .ok_or_else(|| PublicError::not_found(rid))?;
        match rec.status {
            ApprovalStatus::Approved => {}
            ApprovalStatus::Pending => return Err(reject(ApprovalErrorReason::Pending)),
            ApprovalStatus::Rejected => return Err(reject(ApprovalErrorReason::Rejected)),
            ApprovalStatus::Expired => return Err(reject(ApprovalErrorReason::Expired)),
            ApprovalStatus::Consumed => return Err(reject(ApprovalErrorReason::Consumed)),
            ApprovalStatus::Invalidated => {
                return Err(reject(ApprovalErrorReason::BindingMismatch));
            }
        }
        if now >= rec.intent.expires_at() {
            rec.status = ApprovalStatus::Expired;
            return Err(reject(ApprovalErrorReason::Expired));
        }
        if rec.digest != intent.digest() || rec.intent.operation_id() != intent.operation_id() {
            return Err(reject(ApprovalErrorReason::BindingMismatch));
        }
        let evidence = ApprovalEvidence::from_storage(
            id,
            rec.digest.clone(),
            rec.owner,
            rec.decided_at.unwrap_or(now),
            rec.intent.expires_at(),
        );
        match ExecutionAuthority::from_consumed_approval(evidence, intent, target, policy, now) {
            Ok(authority) => {
                // Consumed before dispatch and never reset, even after an unknown outcome.
                rec.status = ApprovalStatus::Consumed;
                self.audit_push(&mut st, ctx, AuditAction::ApprovalConsumed);
                Ok(authority)
            }
            Err(AuthorityError::RevisionChanged { .. }) => {
                rec.status = ApprovalStatus::Invalidated;
                Err(reject(ApprovalErrorReason::BindingMismatch))
            }
            Err(AuthorityError::Expired) => {
                rec.status = ApprovalStatus::Expired;
                Err(reject(ApprovalErrorReason::Expired))
            }
            Err(AuthorityError::PolicyDenied(_)) => Err(PublicError::forbidden(rid)),
            Err(_) => Err(reject(ApprovalErrorReason::BindingMismatch)),
        }
    }
}

#[async_trait]
impl Audit for World {
    async fn begin_operation(
        &self,
        ctx: &RequestContext,
        intent: &OperationIntent,
        authority: &ExecutionAuthority,
    ) -> ServiceResult<AuditTicket> {
        self.check_call(&ctx.call)?;
        let rid = Self::rid(ctx);
        {
            let st = self.lock();
            self.check_principal(&st, ctx)?;
            if st.audit_unavailable {
                return Err(PublicError::unavailable(rid, Some(1000)));
            }
        }
        if authority.operation_id() != intent.operation_id()
            || intent.owner() != ctx.principal.user_id()
        {
            return Err(PublicError::not_found(rid));
        }
        let mut st = self.lock();
        let id = self.audit_push(&mut st, ctx, AuditAction::OperationSubmissionPending);
        let rec = st
            .audit
            .iter_mut()
            .find(|r| r.id == id)
            .expect("just pushed");
        rec.operation_id = Some(intent.operation_id());
        rec.server_id = Some(intent.server_id());
        rec.revision = Some(intent.server_revision());
        rec.tool = Some(intent.tool().clone());
        rec.args = Some(intent.arguments().value().clone());
        rec.approval_id = authority.approval_id();
        rec.submission = Some(SubmissionState::SubmissionPending);
        Ok(AuditTicket::new(id, intent.operation_id()))
    }

    async fn record_submitted(
        &self,
        ctx: &RequestContext,
        ticket: &AuditTicket,
    ) -> ServiceResult<()> {
        self.audit_submitted_inner(ctx, ticket)
    }

    async fn record_outcome(
        &self,
        ctx: &RequestContext,
        ticket: AuditTicket,
        state: SubmissionState,
        outcome: &OperationOutcome,
    ) -> ServiceResult<()> {
        self.audit_outcome_inner(ctx, ticket, state, outcome)
    }

    async fn record_event(
        &self,
        ctx: &RequestContext,
        action: AuditAction,
        _server_id: Option<ServerId>,
    ) -> ServiceResult<()> {
        self.check_call(&ctx.call)?;
        let mut st = self.lock();
        // The caller-supplied server ID is deliberately dropped: events never carry foreign
        // server metadata, and the owner-scoped record needs only the action.
        self.audit_push(&mut st, ctx, action);
        Ok(())
    }

    async fn list_owned(
        &self,
        ctx: &RequestContext,
        query: AuditQuery,
    ) -> ServiceResult<Page<AuditRecord>> {
        self.auth(ctx, Scope::AuditRead)?;
        let st = self.lock();
        let items: Vec<AuditRecord> = st
            .audit
            .iter()
            .filter(|r| r.owner == ctx.principal.user_id())
            .filter(|r| query.server_id.is_none_or(|s| r.server_id == Some(s)))
            .filter(|r| query.operation_id.is_none_or(|o| r.operation_id == Some(o)))
            .filter(|r| {
                query.since.is_none_or(|s| r.at >= s) && query.until.is_none_or(|u| r.at <= u)
            })
            .map(|r| AuditRecord {
                id: r.id,
                request_id: r.request_id,
                action: r.action,
                operation_id: r.operation_id,
                server_id: r.server_id,
                server_revision: r.revision,
                tool: r.tool.clone(),
                redacted_arguments: r.args.clone(),
                approval_id: r.approval_id,
                submission: r.submission,
                outcome: r.outcome,
                refusal_reason: r.refusal,
                exit_code: r.exit_code,
                recorded_at: r.at,
            })
            .collect();
        self.paginate(
            ctx,
            items,
            &PageRequest {
                cursor: query.cursor,
                limit: query.limit,
            },
        )
    }

    async fn reconcile_unresolved(&self, ctx: &CallContext) -> ServiceResult<ReconcileReport> {
        self.check_call(ctx)?;
        let mut st = self.lock();
        let at = self.now();
        let mut marked = 0;
        for rec in st.audit.iter_mut() {
            let unresolved = matches!(
                rec.submission,
                Some(SubmissionState::SubmissionPending | SubmissionState::Submitted)
            ) && rec.outcome.is_none();
            if unresolved {
                // Never assume success or replay: the dispatch may or may not have happened.
                rec.outcome = Some(AuditOutcomeStatus::UnknownAfterSubmission);
                rec.action = AuditAction::OperationOutcomeUnknown;
                rec.at = at;
                marked += 1;
            }
        }
        Ok(ReconcileReport {
            marked_unknown: marked,
        })
    }
}
