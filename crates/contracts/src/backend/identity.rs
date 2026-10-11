//! Trusted `Principal` and the identity boundary (decisions.md D2).
//!
//! A [`Principal`] is the only accepted evidence of who is calling. It is built in exactly
//! one place: [`Principal::from_verified`], taking a [`VerifiedIdentity`] that an identity
//! adapter (Spec 02) produces **after** validating issuer, audience/resource, signature,
//! lifetime and scopes, or that the legacy stdio adapter produces for its single local user.
//! Any other call site outside identity crates and `test_support` is a review defect.
//!
//! `Principal` has private fields and implements neither `Deserialize` nor `Default`, so it
//! cannot be produced from tool arguments, URL parameters or request bodies:
//!
//! ```compile_fail
//! use linux_mcp_contracts::backend::identity::Principal;
//! let _p: Principal = serde_json::from_str("{}").unwrap();
//! ```
//!
//! ```compile_fail
//! use linux_mcp_contracts::backend::identity::Principal;
//! let _p = Principal::default();
//! ```
//!
//! ```compile_fail
//! use linux_mcp_contracts::backend::identity::VerifiedIdentity;
//! let _v: VerifiedIdentity = serde_json::from_str("{}").unwrap();
//! ```
//!
//! Server IDs, MCP session IDs and URLs never identify a caller.

use std::collections::BTreeSet;
use std::fmt;

use crate::public::errors::{InvalidValue, ViolationRule};
use crate::public::ids::{InstallationId, Timestamp, UserId};

/// Authorization scopes. All remain subject to per-server ownership and execution policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scope {
    InventoryRead,
    InventoryWrite,
    CredentialsWrite,
    ServersInspect,
    ServersMutate,
    ApprovalsRequest,
    ApprovalsDecide,
    AuditRead,
}

impl Scope {
    pub const ALL: [Scope; 8] = [
        Scope::InventoryRead,
        Scope::InventoryWrite,
        Scope::CredentialsWrite,
        Scope::ServersInspect,
        Scope::ServersMutate,
        Scope::ApprovalsRequest,
        Scope::ApprovalsDecide,
        Scope::AuditRead,
    ];

    /// OAuth scope string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Scope::InventoryRead => "inventory:read",
            Scope::InventoryWrite => "inventory:write",
            Scope::CredentialsWrite => "credentials:write",
            Scope::ServersInspect => "servers:inspect",
            Scope::ServersMutate => "servers:mutate",
            Scope::ApprovalsRequest => "approvals:request",
            Scope::ApprovalsDecide => "approvals:decide",
            Scope::AuditRead => "audit:read",
        }
    }

    /// Parses one OAuth scope string. Unknown scopes are rejected, not ignored.
    pub fn parse(value: &str) -> Result<Self, InvalidValue> {
        Self::ALL
            .into_iter()
            .find(|scope| scope.as_str() == value)
            .ok_or(InvalidValue::new(ViolationRule::InvalidFormat))
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How the caller authenticated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthKind {
    /// Dashboard browser session (cookie). Never authorizes an external agent by itself.
    BrowserSession,
    /// OAuth access token presented to the remote MCP endpoint.
    McpToken,
    /// Explicitly delegated token used by the dashboard backend on behalf of a user.
    DelegatedToken,
    /// Single-user legacy stdio adapter; never used by the remote service.
    LegacyLocal,
}

/// Opaque session or token *identifier* (session ID, `jti`). Never the bearer value itself.
///
/// 1–128 visible ASCII characters. Used for revocation and binding MCP sessions to subjects.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SessionRef(String);

impl SessionRef {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidValue> {
        let value = value.into();
        if value.is_empty() || value.len() > 128 {
            return Err(InvalidValue::new(ViolationRule::OutOfRange));
        }
        if !value.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(InvalidValue::new(ViolationRule::InvalidFormat));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SessionRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SessionRef({})", self.0)
    }
}

/// Claims an identity adapter has verified. Constructing one asserts that verification
/// happened; only identity adapters and the legacy adapter may do so.
#[derive(Debug, Clone)]
pub struct VerifiedIdentity {
    pub user_id: UserId,
    pub installation_id: InstallationId,
    pub scopes: BTreeSet<Scope>,
    pub session: SessionRef,
    pub auth_kind: AuthKind,
    pub authenticated_at: Timestamp,
    pub expires_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    #[error("identity expires before it was authenticated")]
    InvalidLifetime,
    #[error("principal has expired")]
    Expired,
    #[error("principal lacks scope {0}")]
    MissingScope(Scope),
    #[error("principal belongs to another installation")]
    WrongInstallation,
}

/// Authenticated caller for one request. See the module docs for the construction rule.
#[derive(Clone, PartialEq, Eq)]
pub struct Principal {
    user_id: UserId,
    installation_id: InstallationId,
    scopes: BTreeSet<Scope>,
    session: SessionRef,
    auth_kind: AuthKind,
    authenticated_at: Timestamp,
    expires_at: Timestamp,
}

impl Principal {
    /// The single trusted-adapter construction boundary.
    pub fn from_verified(identity: VerifiedIdentity) -> Result<Self, IdentityError> {
        if identity.expires_at <= identity.authenticated_at {
            return Err(IdentityError::InvalidLifetime);
        }
        Ok(Self {
            user_id: identity.user_id,
            installation_id: identity.installation_id,
            scopes: identity.scopes,
            session: identity.session,
            auth_kind: identity.auth_kind,
            authenticated_at: identity.authenticated_at,
            expires_at: identity.expires_at,
        })
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn installation_id(&self) -> InstallationId {
        self.installation_id
    }

    pub fn scopes(&self) -> &BTreeSet<Scope> {
        &self.scopes
    }

    pub fn has_scope(&self, scope: Scope) -> bool {
        self.scopes.contains(&scope)
    }

    pub fn session(&self) -> &SessionRef {
        &self.session
    }

    pub fn auth_kind(&self) -> AuthKind {
        self.auth_kind
    }

    pub fn authenticated_at(&self) -> Timestamp {
        self.authenticated_at
    }

    pub fn expires_at(&self) -> Timestamp {
        self.expires_at
    }

    pub fn is_expired_at(&self, now: Timestamp) -> bool {
        now >= self.expires_at
    }

    /// Checks lifetime, installation and scope together; used at every authorization point.
    pub fn authorize(
        &self,
        installation_id: InstallationId,
        scope: Scope,
        now: Timestamp,
    ) -> Result<(), IdentityError> {
        if self.is_expired_at(now) {
            return Err(IdentityError::Expired);
        }
        if self.installation_id != installation_id {
            return Err(IdentityError::WrongInstallation);
        }
        if !self.has_scope(scope) {
            return Err(IdentityError::MissingScope(scope));
        }
        Ok(())
    }
}

impl fmt::Debug for Principal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Principal")
            .field("user_id", &self.user_id)
            .field("installation_id", &self.installation_id)
            .field("scopes", &self.scopes)
            .field("auth_kind", &self.auth_kind)
            .field("session", &self.session)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}
