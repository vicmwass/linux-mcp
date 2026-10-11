//! Policy evaluation, human approval evidence and executable authority (decisions.md D9).
//!
//! - [`PolicyEvaluation`]: the executor's preflight verdict for one intent digest and revision.
//! - [`ApprovalEvidence`]: a human decision. Only an interactive browser session of the
//!   intent's owner holding `approvals:decide` can record it; MCP tokens, delegated tokens,
//!   mutation scope and `confirm: true` cannot.
//! - [`ExecutionAuthority`]: the only thing `Executor::submit` accepts. It is not `Clone`, not
//!   serializable and has private fields, so a public boolean or an arbitrary approval ID
//!   cannot manufacture it:
//!
//! ```compile_fail
//! fn forge(a: &linux_mcp_contracts::backend::operation_authority::ExecutionAuthority) {
//!     let _copy: linux_mcp_contracts::backend::operation_authority::ExecutionAuthority = a.clone();
//! }
//! ```
//!
//! ```compile_fail
//! fn leak(a: &linux_mcp_contracts::backend::operation_authority::ExecutionAuthority) {
//!     let _ = serde_json::to_string(a);
//! }
//! ```
//!
//! Approval never overrides a policy denial; [`ExecutionAuthority::recheck`] re-evaluates
//! the current policy verdict and target revision immediately before submission.

use crate::backend::identity::{AuthKind, Principal, Scope};
use crate::backend::target::AuthorizedTarget;
use crate::canonical::IntentDigest;
use crate::public::ids::{ApprovalId, OperationId, ServerRevision, Timestamp, UserId};
use crate::public::operations::{OperationIntent, PolicyVerdict, RefusalReason};

/// Preflight (or submit-time) policy result bound to an intent digest and target revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEvaluation {
    pub intent_digest: IntentDigest,
    pub server_revision: ServerRevision,
    pub verdict: PolicyVerdict,
    /// Identifier of the policy rules version that produced the verdict.
    pub policy_version: String,
    pub evaluated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthorityError {
    #[error("policy denied the operation: {0:?}")]
    PolicyDenied(RefusalReason),
    #[error("operation requires human approval")]
    ApprovalRequired,
    #[error("approval does not match the intent binding")]
    BindingMismatch,
    #[error("intent or approval has expired")]
    Expired,
    #[error("target revision changed since the intent was created")]
    RevisionChanged { current: ServerRevision },
    #[error("approver is not allowed to approve this intent")]
    ApproverNotAllowed,
    #[error("unsupported canonicalization version")]
    UnsupportedCanonicalization,
}

impl AuthorityError {
    pub fn refusal_reason(&self) -> RefusalReason {
        match self {
            AuthorityError::PolicyDenied(reason) => *reason,
            AuthorityError::ApprovalRequired => RefusalReason::ApprovalRequired,
            AuthorityError::BindingMismatch
            | AuthorityError::Expired
            | AuthorityError::ApproverNotAllowed
            | AuthorityError::UnsupportedCanonicalization => RefusalReason::ApprovalInvalid,
            AuthorityError::RevisionChanged { .. } => RefusalReason::RevisionChanged,
        }
    }
}

/// Recorded human decision approving one intent digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalEvidence {
    approval_id: ApprovalId,
    intent_digest: IntentDigest,
    approver: UserId,
    decided_at: Timestamp,
    expires_at: Timestamp,
}

impl ApprovalEvidence {
    /// Records an approval decision. First release: the owner approves their own intents
    /// through an authenticated, CSRF-protected browser session.
    pub fn record(
        approval_id: ApprovalId,
        approver: &Principal,
        intent: &OperationIntent,
        decided_at: Timestamp,
    ) -> Result<Self, AuthorityError> {
        let interactive = matches!(approver.auth_kind(), AuthKind::BrowserSession);
        let allowed = interactive
            && approver.user_id() == intent.owner()
            && approver.installation_id() == intent.installation_id()
            && approver.has_scope(Scope::ApprovalsDecide)
            && !approver.is_expired_at(decided_at);
        if !allowed {
            return Err(AuthorityError::ApproverNotAllowed);
        }
        if intent.check_version().is_err() {
            return Err(AuthorityError::UnsupportedCanonicalization);
        }
        if intent.is_expired_at(decided_at) {
            return Err(AuthorityError::Expired);
        }
        Ok(Self {
            approval_id,
            intent_digest: intent.digest(),
            approver: approver.user_id(),
            decided_at,
            expires_at: intent.expires_at(),
        })
    }

    /// Rehydrates evidence from durable storage (governance adapter only).
    pub fn from_storage(
        approval_id: ApprovalId,
        intent_digest: IntentDigest,
        approver: UserId,
        decided_at: Timestamp,
        expires_at: Timestamp,
    ) -> Self {
        Self {
            approval_id,
            intent_digest,
            approver,
            decided_at,
            expires_at,
        }
    }

    pub fn approval_id(&self) -> ApprovalId {
        self.approval_id
    }
    pub fn intent_digest(&self) -> &IntentDigest {
        &self.intent_digest
    }
    pub fn approver(&self) -> UserId {
        self.approver
    }
    pub fn decided_at(&self) -> Timestamp {
        self.decided_at
    }
    pub fn expires_at(&self) -> Timestamp {
        self.expires_at
    }
}

/// Single-use authority to submit exactly one intent to one authorized target revision.
#[derive(Debug)]
pub struct ExecutionAuthority {
    operation_id: OperationId,
    intent_digest: IntentDigest,
    owner: UserId,
    server_revision: ServerRevision,
    approval_id: Option<ApprovalId>,
    expires_at: Timestamp,
}

fn check_binding(
    intent: &OperationIntent,
    target: &AuthorizedTarget,
    policy: &PolicyEvaluation,
    now: Timestamp,
) -> Result<IntentDigest, AuthorityError> {
    intent
        .check_version()
        .map_err(|_| AuthorityError::UnsupportedCanonicalization)?;
    let digest = intent.digest();
    if intent.is_expired_at(now) {
        return Err(AuthorityError::Expired);
    }
    if target.owner() != intent.owner()
        || target.resolved().installation_id != intent.installation_id()
        || target.server_id() != intent.server_id()
        || policy.intent_digest != digest
    {
        return Err(AuthorityError::BindingMismatch);
    }
    if target.revision() != intent.server_revision() || policy.server_revision != target.revision()
    {
        return Err(AuthorityError::RevisionChanged {
            current: target.revision(),
        });
    }
    if let PolicyVerdict::Denied { reason } = policy.verdict {
        return Err(AuthorityError::PolicyDenied(reason));
    }
    Ok(digest)
}

impl ExecutionAuthority {
    /// For intents whose policy verdict is `allowed` (no human approval needed).
    pub fn without_approval(
        intent: &OperationIntent,
        target: &AuthorizedTarget,
        policy: &PolicyEvaluation,
        now: Timestamp,
    ) -> Result<Self, AuthorityError> {
        let digest = check_binding(intent, target, policy, now)?;
        if policy.verdict != PolicyVerdict::Allowed {
            return Err(AuthorityError::ApprovalRequired);
        }
        Ok(Self {
            operation_id: intent.operation_id(),
            intent_digest: digest,
            owner: intent.owner(),
            server_revision: intent.server_revision(),
            approval_id: None,
            expires_at: intent.expires_at(),
        })
    }

    /// Called by `Approvals::consume` **after** atomically marking the approval consumed.
    pub fn from_consumed_approval(
        evidence: ApprovalEvidence,
        intent: &OperationIntent,
        target: &AuthorizedTarget,
        policy: &PolicyEvaluation,
        now: Timestamp,
    ) -> Result<Self, AuthorityError> {
        let digest = check_binding(intent, target, policy, now)?;
        if evidence.intent_digest != digest || evidence.approver != intent.owner() {
            return Err(AuthorityError::BindingMismatch);
        }
        if now >= evidence.expires_at {
            return Err(AuthorityError::Expired);
        }
        Ok(Self {
            operation_id: intent.operation_id(),
            intent_digest: digest,
            owner: intent.owner(),
            server_revision: intent.server_revision(),
            approval_id: Some(evidence.approval_id),
            expires_at: intent.expires_at(),
        })
    }

    /// Submit-time recheck against the current target revision and a fresh policy verdict.
    pub fn recheck(
        &self,
        intent: &OperationIntent,
        target: &AuthorizedTarget,
        current_policy: &PolicyEvaluation,
        now: Timestamp,
    ) -> Result<(), AuthorityError> {
        let digest = check_binding(intent, target, current_policy, now)?;
        if digest != self.intent_digest
            || intent.operation_id() != self.operation_id
            || target.owner() != self.owner
        {
            return Err(AuthorityError::BindingMismatch);
        }
        if target.revision() != self.server_revision {
            return Err(AuthorityError::RevisionChanged {
                current: target.revision(),
            });
        }
        if now >= self.expires_at {
            return Err(AuthorityError::Expired);
        }
        Ok(())
    }

    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }
    pub fn intent_digest(&self) -> &IntentDigest {
        &self.intent_digest
    }
    pub fn owner(&self) -> UserId {
        self.owner
    }
    pub fn server_revision(&self) -> ServerRevision {
        self.server_revision
    }
    pub fn approval_id(&self) -> Option<ApprovalId> {
        self.approval_id
    }
}
