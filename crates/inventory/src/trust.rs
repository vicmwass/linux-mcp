//! Host-key observation and independently verified host trust.

use linux_mcp_contracts::public::ids::{ServerId, ServerRevision, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKeyObservation {
    pub owner_user_id: UserId,
    pub server_id: ServerId,
    pub hostname: String,
    pub port: u16,
    pub fingerprint: String,
    pub revision: ServerRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedHostKey {
    pub observation: HostKeyObservation,
    pub verified_fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TrustError {
    #[error("host trust observation is stale")]
    StaleObservation,
    #[error("host fingerprint does not match the observation")]
    FingerprintMismatch,
    #[error("host trust identity does not match the target")]
    TargetMismatch,
    #[error("host key is not verified")]
    Unverified,
}

pub fn enroll(
    observation: HostKeyObservation,
    current_revision: ServerRevision,
    presented_fingerprint: &str,
) -> Result<VerifiedHostKey, TrustError> {
    if observation.revision != current_revision {
        return Err(TrustError::StaleObservation);
    }
    if observation.fingerprint != presented_fingerprint {
        return Err(TrustError::FingerprintMismatch);
    }
    Ok(VerifiedHostKey {
        verified_fingerprint: presented_fingerprint.to_owned(),
        observation,
    })
}

pub fn verify_connection(
    trusted: &VerifiedHostKey,
    owner_user_id: UserId,
    server_id: ServerId,
    hostname: &str,
    port: u16,
    presented_fingerprint: &str,
) -> Result<(), TrustError> {
    let o = &trusted.observation;
    if o.owner_user_id != owner_user_id
        || o.server_id != server_id
        || o.hostname != hostname
        || o.port != port
    {
        return Err(TrustError::TargetMismatch);
    }
    if trusted.verified_fingerprint != presented_fingerprint {
        return Err(TrustError::Unverified);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (UserId, ServerId) {
        (UserId::new_random(), ServerId::new_random())
    }

    #[test]
    fn enrollment_requires_current_observation_and_exact_fingerprint() {
        let (owner, server) = ids();
        let observation = HostKeyObservation {
            owner_user_id: owner,
            server_id: server,
            hostname: "host.example".into(),
            port: 22,
            fingerprint: "SHA256:one".into(),
            revision: ServerRevision::INITIAL,
        };
        assert_eq!(
            enroll(
                observation.clone(),
                ServerRevision::new(2).unwrap(),
                "SHA256:one"
            ),
            Err(TrustError::StaleObservation)
        );
        assert_eq!(
            enroll(observation, ServerRevision::INITIAL, "SHA256:two"),
            Err(TrustError::FingerprintMismatch)
        );
    }

    #[test]
    fn trust_is_scoped_to_owner_server_host_and_port() {
        let (owner, server) = ids();
        let o = HostKeyObservation {
            owner_user_id: owner,
            server_id: server,
            hostname: "host.example".into(),
            port: 22,
            fingerprint: "SHA256:one".into(),
            revision: ServerRevision::INITIAL,
        };
        let t = enroll(o, ServerRevision::INITIAL, "SHA256:one").unwrap();
        assert!(verify_connection(&t, owner, server, "host.example", 22, "SHA256:one").is_ok());
        assert_eq!(
            verify_connection(&t, owner, server, "host.example", 2222, "SHA256:one"),
            Err(TrustError::TargetMismatch)
        );
        assert_eq!(
            verify_connection(&t, owner, server, "host.example", 22, "SHA256:two"),
            Err(TrustError::Unverified)
        );
    }
}
