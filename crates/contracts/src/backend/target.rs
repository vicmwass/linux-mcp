//! Resolved and authorized targets.
//!
//! A [`ServerId`] alone is never an authorized target. The sequence is:
//!
//! 1. `TargetResolver` (Spec 01) loads the **current** record as a [`ResolvedTarget`]
//!    (owner, revision, enabled state, network target, host trust, credential references).
//! 2. [`AuthorizedTarget::authorize`] checks principal lifetime, installation, scope,
//!    ownership, enabled state and verified host trust. Ownership failure maps to the same
//!    `not_found` as an absent server.
//! 3. Credential leases and execution accept only `&AuthorizedTarget`; submission rechecks
//!    the current revision with [`AuthorizedTarget::ensure_current`].
//!
//! `AuthorizedTarget` has private fields and is neither `Clone` nor serializable, so it is
//! re-derived for every operation rather than cached or passed through clients:
//!
//! ```compile_fail
//! fn copy(t: &linux_mcp_contracts::backend::target::AuthorizedTarget) {
//!     let _again: linux_mcp_contracts::backend::target::AuthorizedTarget = t.clone();
//! }
//! ```

use crate::backend::credentials::CredentialRef;
use crate::backend::identity::{IdentityError, Principal, Scope};
use crate::public::errors::{ErrorCode, PublicError, ValidationError, ViolationRule};
use crate::public::ids::{
    HostTrustId, InstallationId, RequestId, ServerId, ServerRevision, Timestamp, UserId,
};
use crate::public::inventory::{ApprovalMode, HostTrustStatus, Hostname, SshPort, SshUsername};

/// Where and as whom to connect. Server-side only.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkTarget {
    pub hostname: Hostname,
    pub port: SshPort,
    pub username: SshUsername,
}

/// Owner-scoped host-trust reference for (hostname, port).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostTrustRef {
    pub id: HostTrustId,
    pub status: HostTrustStatus,
    /// `SHA256:<base64>` fingerprint the SSH layer must match exactly.
    pub fingerprint: String,
    pub hostname: Hostname,
    pub port: SshPort,
}

/// Current inventory record for one server, as loaded by a resolver. Not yet authorized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub owner: UserId,
    pub installation_id: InstallationId,
    pub server_id: ServerId,
    pub revision: ServerRevision,
    pub enabled: bool,
    /// Which operations need human approval on this server (current value, resolved fresh).
    pub approval_mode: ApprovalMode,
    pub network: NetworkTarget,
    pub host_trust: Option<HostTrustRef>,
    pub credentials: Vec<CredentialRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TargetError {
    /// Absent **or** owned by someone else; callers cannot tell which.
    #[error("server not found")]
    NotFound,
    #[error(transparent)]
    Identity(#[from] IdentityError),
    #[error("server is disabled")]
    Disabled,
    #[error("host key is not verified for this target")]
    HostTrustNotVerified,
    #[error("target changed: current revision {current}")]
    RevisionChanged { current: ServerRevision },
}

impl TargetError {
    /// Sanitized public mapping. Foreign ownership and wrong installation are `not_found`.
    pub fn to_public(&self, request_id: RequestId) -> PublicError {
        match self {
            TargetError::NotFound | TargetError::Identity(IdentityError::WrongInstallation) => {
                PublicError::not_found(request_id)
            }
            TargetError::Identity(IdentityError::Expired)
            | TargetError::Identity(IdentityError::InvalidLifetime) => {
                PublicError::unauthenticated(request_id)
            }
            TargetError::Identity(IdentityError::MissingScope(_)) => {
                PublicError::forbidden(request_id)
            }
            TargetError::Disabled => PublicError::validation(
                request_id,
                ValidationError::single("server.enabled", ViolationRule::NotAllowed),
            ),
            TargetError::HostTrustNotVerified => PublicError::validation(
                request_id,
                ValidationError::single("server.host_trust", ViolationRule::NotAllowed),
            ),
            TargetError::RevisionChanged { current } => {
                PublicError::revision_conflict(request_id, *current)
            }
        }
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            TargetError::NotFound | TargetError::Identity(IdentityError::WrongInstallation) => {
                ErrorCode::NotFound
            }
            TargetError::Identity(IdentityError::Expired)
            | TargetError::Identity(IdentityError::InvalidLifetime) => ErrorCode::Unauthenticated,
            TargetError::Identity(IdentityError::MissingScope(_)) => ErrorCode::Forbidden,
            TargetError::Disabled | TargetError::HostTrustNotVerified => ErrorCode::Validation,
            TargetError::RevisionChanged { .. } => ErrorCode::RevisionConflict,
        }
    }
}

/// Evidence that a principal may act on the current revision of a server.
#[derive(Debug)]
pub struct AuthorizedTarget {
    principal_user: UserId,
    scope: Scope,
    authorized_at: Timestamp,
    resolved: ResolvedTarget,
}

impl AuthorizedTarget {
    /// Authorizes a freshly resolved record. Checks run in an order that never reveals
    /// whether a foreign server exists: identity/installation, then ownership, then state.
    pub fn authorize(
        principal: &Principal,
        resolved: ResolvedTarget,
        scope: Scope,
        now: Timestamp,
    ) -> Result<Self, TargetError> {
        principal.authorize(principal.installation_id(), scope, now)?;
        if resolved.installation_id != principal.installation_id()
            || resolved.owner != principal.user_id()
        {
            return Err(TargetError::NotFound);
        }
        if !resolved.enabled {
            return Err(TargetError::Disabled);
        }
        let trusted = resolved.host_trust.as_ref().is_some_and(|trust| {
            trust.status.permits_connection()
                && trust.hostname == resolved.network.hostname
                && trust.port == resolved.network.port
        });
        if !trusted {
            return Err(TargetError::HostTrustNotVerified);
        }
        Ok(Self {
            principal_user: principal.user_id(),
            scope,
            authorized_at: now,
            resolved,
        })
    }

    /// Revision recheck immediately before submission (or secret resolution).
    pub fn ensure_current(&self, current_revision: ServerRevision) -> Result<(), TargetError> {
        if current_revision == self.resolved.revision {
            Ok(())
        } else {
            Err(TargetError::RevisionChanged {
                current: current_revision,
            })
        }
    }

    pub fn resolved(&self) -> &ResolvedTarget {
        &self.resolved
    }

    pub fn owner(&self) -> UserId {
        self.principal_user
    }

    pub fn server_id(&self) -> ServerId {
        self.resolved.server_id
    }

    pub fn revision(&self) -> ServerRevision {
        self.resolved.revision
    }

    /// Approval rule of the target at resolution time; the executor's policy uses it.
    pub fn approval_mode(&self) -> ApprovalMode {
        self.resolved.approval_mode
    }

    pub fn scope(&self) -> Scope {
        self.scope
    }

    pub fn authorized_at(&self) -> Timestamp {
        self.authorized_at
    }

    /// Isolation key for connection pools and capability caches (Spec 03):
    /// owner, server, effective SSH username and revision.
    pub fn cache_key(&self) -> TargetCacheKey {
        TargetCacheKey {
            owner: self.resolved.owner,
            server_id: self.resolved.server_id,
            username: self.resolved.network.username.clone(),
            revision: self.resolved.revision,
        }
    }
}

/// Cache/pool key; two owners with identical hostnames never share an entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetCacheKey {
    pub owner: UserId,
    pub server_id: ServerId,
    pub username: SshUsername,
    pub revision: ServerRevision,
}
