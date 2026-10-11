//! Credential references, leases and redacted secret material (decisions.md D7).
//!
//! - [`CredentialRef`] is an owner- and server-scoped reference: an ID, never a filesystem
//!   path or secret. There is deliberately no constructor taking a path.
//! - [`CredentialMaterial`] holds plaintext. It is not `Clone`, not `Serialize`, prints
//!   `<redacted>` in `Debug`, and zeroizes its buffer on drop.
//! - [`CredentialLease`] can only be issued for an [`AuthorizedTarget`] whose resolved
//!   credentials include the reference, and refuses access after its short lifetime.
//!
//! Memory clearing covers only the buffer owned by `CredentialMaterial`. It cannot clear
//! copies made by SSH/crypto libraries, the allocator after reallocation, swap, core dumps
//! or a debugger. Adapters must minimize copies and the time plaintext exists.
//!
//! ```compile_fail
//! use linux_mcp_contracts::backend::credentials::CredentialMaterial;
//! use linux_mcp_contracts::public::inventory::CredentialKind;
//! let m = CredentialMaterial::new(CredentialKind::SudoPassword, b"x".to_vec());
//! let _copy = m.clone();
//! ```
//!
//! ```compile_fail
//! use linux_mcp_contracts::backend::credentials::CredentialMaterial;
//! use linux_mcp_contracts::public::inventory::CredentialKind;
//! let m = CredentialMaterial::new(CredentialKind::SudoPassword, b"x".to_vec());
//! let _json = serde_json::to_string(&m);
//! ```

use std::fmt;

use zeroize::Zeroizing;

use super::target::AuthorizedTarget;
use crate::public::ids::{CredentialId, ServerId, ServerRevision, Timestamp, UserId};
use crate::public::inventory::CredentialKind;

/// Maximum lease lifetime (Q3: Spec 03 may lower it).
pub const MAX_LEASE_MILLIS: u64 = 60_000;

/// Owner- and server-scoped credential reference with binding revision evidence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CredentialRef {
    pub id: CredentialId,
    pub kind: CredentialKind,
    pub owner: UserId,
    pub server_id: ServerId,
    /// Server revision at which this credential was bound; rotation bumps the server revision.
    pub bound_at_revision: ServerRevision,
    pub enabled: bool,
}

/// Why a secret is being resolved; recorded by audit and checked against the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CredentialPurpose {
    SshAuthentication,
    SudoElevation,
}

impl CredentialPurpose {
    pub fn kind(self) -> CredentialKind {
        match self {
            CredentialPurpose::SshAuthentication => CredentialKind::SshPrivateKey,
            CredentialPurpose::SudoElevation => CredentialKind::SudoPassword,
        }
    }
}

/// Plaintext secret. See module docs for guarantees and limits.
pub struct CredentialMaterial {
    kind: CredentialKind,
    bytes: Zeroizing<Vec<u8>>,
}

impl CredentialMaterial {
    /// For credential-store adapters after decryption. Takes ownership to avoid copies.
    pub fn new(kind: CredentialKind, bytes: Vec<u8>) -> Self {
        Self {
            kind,
            bytes: Zeroizing::new(bytes),
        }
    }

    pub fn kind(&self) -> CredentialKind {
        self.kind
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl fmt::Debug for CredentialMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CredentialMaterial({:?}, <redacted>)", self.kind)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CredentialError {
    /// The reference is not part of the authorized target (foreign, stale or unrelated).
    #[error("credential is not bound to the authorized target")]
    NotBound,
    #[error("credential is disabled")]
    Disabled,
    #[error("credential kind does not match its purpose")]
    KindMismatch,
    #[error("lease lifetime must be 1..={MAX_LEASE_MILLIS} ms")]
    InvalidLifetime,
    #[error("credential lease has expired")]
    Expired,
}

/// Short-lived access to one credential for one authorized target.
pub struct CredentialLease {
    credential_id: CredentialId,
    owner: UserId,
    server_id: ServerId,
    revision: ServerRevision,
    purpose: CredentialPurpose,
    expires_at: Timestamp,
    material: CredentialMaterial,
}

impl CredentialLease {
    /// Issues a lease after checking the reference belongs to the authorized target.
    pub fn issue(
        target: &AuthorizedTarget,
        credential: &CredentialRef,
        purpose: CredentialPurpose,
        material: CredentialMaterial,
        issued_at: Timestamp,
        ttl_millis: u64,
    ) -> Result<Self, CredentialError> {
        let resolved = target.resolved();
        let bound = resolved.credentials.iter().any(|c| c == credential)
            && credential.owner == resolved.owner
            && credential.server_id == resolved.server_id
            && credential.bound_at_revision <= resolved.revision;
        if !bound {
            return Err(CredentialError::NotBound);
        }
        if !credential.enabled {
            return Err(CredentialError::Disabled);
        }
        if credential.kind != purpose.kind() || material.kind() != credential.kind {
            return Err(CredentialError::KindMismatch);
        }
        if ttl_millis == 0 || ttl_millis > MAX_LEASE_MILLIS {
            return Err(CredentialError::InvalidLifetime);
        }
        let expires_at = issued_at
            .checked_add_millis(ttl_millis)
            .ok_or(CredentialError::InvalidLifetime)?;
        Ok(Self {
            credential_id: credential.id,
            owner: resolved.owner,
            server_id: resolved.server_id,
            revision: resolved.revision,
            purpose,
            expires_at,
            material,
        })
    }

    /// Scoped access to the plaintext. The closure must not retain copies longer than needed.
    pub fn with_secret<R>(
        &self,
        now: Timestamp,
        f: impl FnOnce(&[u8]) -> R,
    ) -> Result<R, CredentialError> {
        if now >= self.expires_at {
            return Err(CredentialError::Expired);
        }
        Ok(f(&self.material.bytes))
    }

    pub fn credential_id(&self) -> CredentialId {
        self.credential_id
    }

    pub fn owner(&self) -> UserId {
        self.owner
    }

    pub fn server_id(&self) -> ServerId {
        self.server_id
    }

    pub fn revision(&self) -> ServerRevision {
        self.revision
    }

    pub fn purpose(&self) -> CredentialPurpose {
        self.purpose
    }

    pub fn expires_at(&self) -> Timestamp {
        self.expires_at
    }
}

impl fmt::Debug for CredentialLease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialLease")
            .field("credential_id", &self.credential_id)
            .field("server_id", &self.server_id)
            .field("purpose", &self.purpose)
            .field("expires_at", &self.expires_at)
            .field("material", &self.material)
            .finish()
    }
}
