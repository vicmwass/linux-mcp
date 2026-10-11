//! Encrypted credential-store boundary.

use std::fmt;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use linux_mcp_contracts::{
    backend::credentials::CredentialMaterial,
    public::{
        ids::{CredentialId, ServerId, ServerRevision},
        inventory::CredentialKind,
    },
};
use rand::{RngCore, rngs::OsRng};
use zeroize::Zeroizing;

const ENVELOPE_VERSION: u8 = 1;
const NONCE_LENGTH: usize = 12;
const MAX_SECRET_BYTES: usize = 16 * 1024;

/// The external key service boundary. Implementations must not persist keys in
/// PostgreSQL, the image, or application configuration.
pub trait ExternalKeyProvider: Send + Sync {
    fn load_key(&self, key_id: &str) -> Result<[u8; 32], CredentialCryptoError>;
}

/// Database-safe encrypted credential envelope. It contains no plaintext.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialEnvelope {
    pub format_version: u8,
    pub key_id: String,
    pub nonce: [u8; NONCE_LENGTH],
    pub ciphertext: Vec<u8>,
}

impl fmt::Debug for CredentialEnvelope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialEnvelope")
            .field("format_version", &self.format_version)
            .field("key_id", &self.key_id)
            .field("nonce", &"<redacted>")
            .field("ciphertext_len", &self.ciphertext.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CredentialCryptoError {
    #[error("credential key is unavailable")]
    KeyUnavailable,
    #[error("credential envelope is invalid")]
    InvalidEnvelope,
    #[error("credential authentication failed")]
    AuthenticationFailed,
    #[error("credential is too large")]
    SecretTooLarge,
}

/// Stateless authenticated-encryption adapter for credential envelopes.
pub struct CredentialCipher<K> {
    keys: K,
}

impl<K: ExternalKeyProvider> CredentialCipher<K> {
    pub fn new(keys: K) -> Self {
        Self { keys }
    }

    /// Encrypts a secret with fresh nonce and authenticated target metadata.
    ///
    /// The caller owns the short-lived plaintext buffer. This method never stores
    /// it and returns only an authenticated ciphertext envelope.
    pub fn encrypt(
        &self,
        key_id: &str,
        installation_id: &str,
        owner_id: &str,
        server_id: ServerId,
        credential_id: CredentialId,
        kind: CredentialKind,
        revision: ServerRevision,
        secret: &[u8],
    ) -> Result<CredentialEnvelope, CredentialCryptoError> {
        if secret.len() > MAX_SECRET_BYTES {
            return Err(CredentialCryptoError::SecretTooLarge);
        }
        let key = self.keys.load_key(key_id)?;
        let cipher =
            Aes256Gcm::new_from_slice(&key).map_err(|_| CredentialCryptoError::KeyUnavailable)?;
        let mut nonce = [0u8; NONCE_LENGTH];
        OsRng.fill_bytes(&mut nonce);
        let aad = associated_data(
            installation_id,
            owner_id,
            server_id,
            credential_id,
            kind,
            revision,
        );
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                aes_gcm::aead::Payload {
                    msg: secret,
                    aad: &aad,
                },
            )
            .map_err(|_| CredentialCryptoError::AuthenticationFailed)?;
        Ok(CredentialEnvelope {
            format_version: ENVELOPE_VERSION,
            key_id: key_id.to_owned(),
            nonce,
            ciphertext,
        })
    }

    /// Decrypts only with the exact metadata used during binding.
    pub fn decrypt(
        &self,
        envelope: &CredentialEnvelope,
        installation_id: &str,
        owner_id: &str,
        server_id: ServerId,
        credential_id: CredentialId,
        kind: CredentialKind,
        revision: ServerRevision,
    ) -> Result<CredentialMaterial, CredentialCryptoError> {
        if envelope.format_version != ENVELOPE_VERSION
            || envelope.key_id.is_empty()
            || envelope.ciphertext.is_empty()
        {
            return Err(CredentialCryptoError::InvalidEnvelope);
        }
        let key = self.keys.load_key(&envelope.key_id)?;
        let cipher =
            Aes256Gcm::new_from_slice(&key).map_err(|_| CredentialCryptoError::KeyUnavailable)?;
        let aad = associated_data(
            installation_id,
            owner_id,
            server_id,
            credential_id,
            kind,
            revision,
        );
        let plaintext = cipher
            .decrypt(
                Nonce::from_slice(&envelope.nonce),
                aes_gcm::aead::Payload {
                    msg: &envelope.ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| CredentialCryptoError::AuthenticationFailed)?;
        if plaintext.len() > MAX_SECRET_BYTES {
            return Err(CredentialCryptoError::SecretTooLarge);
        }
        let plaintext = Zeroizing::new(plaintext);
        Ok(CredentialMaterial::new(kind, plaintext.to_vec()))
    }

    /// Re-encrypts an envelope under a new external key without changing its
    /// owner, server, credential binding, or revision.
    pub fn rotate(
        &self,
        envelope: &CredentialEnvelope,
        new_key_id: &str,
        installation_id: &str,
        owner_id: &str,
        server_id: ServerId,
        credential_id: CredentialId,
        kind: CredentialKind,
        revision: ServerRevision,
    ) -> Result<CredentialEnvelope, CredentialCryptoError> {
        if new_key_id.is_empty() {
            return Err(CredentialCryptoError::KeyUnavailable);
        }
        let old_key = self.keys.load_key(&envelope.key_id)?;
        let old_cipher = Aes256Gcm::new_from_slice(&old_key)
            .map_err(|_| CredentialCryptoError::KeyUnavailable)?;
        let aad = associated_data(
            installation_id,
            owner_id,
            server_id,
            credential_id,
            kind,
            revision,
        );
        let plaintext = old_cipher
            .decrypt(
                Nonce::from_slice(&envelope.nonce),
                aes_gcm::aead::Payload {
                    msg: &envelope.ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| CredentialCryptoError::AuthenticationFailed)?;
        let plaintext = Zeroizing::new(plaintext);
        self.encrypt(
            new_key_id,
            installation_id,
            owner_id,
            server_id,
            credential_id,
            kind,
            revision,
            &plaintext,
        )
    }
}

fn associated_data(
    installation_id: &str,
    owner_id: &str,
    server_id: ServerId,
    credential_id: CredentialId,
    kind: CredentialKind,
    revision: ServerRevision,
) -> Vec<u8> {
    format!(
        "linux-mcp/inventory/v1|{installation_id}|{owner_id}|{server_id}|{credential_id}|{kind:?}|{revision}"
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use linux_mcp_contracts::public::ids::{CredentialId, ServerId, ServerRevision};
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct TestKeys(Mutex<HashMap<String, [u8; 32]>>);

    impl ExternalKeyProvider for TestKeys {
        fn load_key(&self, key_id: &str) -> Result<[u8; 32], CredentialCryptoError> {
            self.0
                .lock()
                .expect("test key lock")
                .get(key_id)
                .copied()
                .ok_or(CredentialCryptoError::KeyUnavailable)
        }
    }

    fn ids() -> (ServerId, CredentialId, ServerRevision) {
        (
            ServerId::new_random(),
            CredentialId::new_random(),
            ServerRevision::new(1).expect("valid revision"),
        )
    }

    fn cipher() -> CredentialCipher<TestKeys> {
        let mut keys = HashMap::new();
        keys.insert("key-1".to_owned(), [7u8; 32]);
        CredentialCipher::new(TestKeys(Mutex::new(keys)))
    }

    #[test]
    fn encrypts_and_decrypts_with_bound_metadata() {
        let cipher = cipher();
        let (server, credential, revision) = ids();
        let envelope = cipher
            .encrypt(
                "key-1",
                "installation",
                "owner",
                server,
                credential,
                CredentialKind::SudoPassword,
                revision,
                b"FAKE-SECRET-password",
            )
            .expect("encrypt");
        let material = cipher
            .decrypt(
                &envelope,
                "installation",
                "owner",
                server,
                credential,
                CredentialKind::SudoPassword,
                revision,
            )
            .expect("decrypt");
        assert_eq!(material.kind(), CredentialKind::SudoPassword);
        assert_eq!(material.len(), b"FAKE-SECRET-password".len());
    }

    #[test]
    fn tampering_or_metadata_changes_fail_authentication() {
        let cipher = cipher();
        let (server, credential, revision) = ids();
        let mut envelope = cipher
            .encrypt(
                "key-1",
                "installation",
                "owner",
                server,
                credential,
                CredentialKind::SshPrivateKey,
                revision,
                b"FAKE-SECRET-key",
            )
            .expect("encrypt");
        envelope.ciphertext[0] ^= 1;
        assert!(matches!(
            cipher.decrypt(
                &envelope,
                "installation",
                "owner",
                server,
                credential,
                CredentialKind::SshPrivateKey,
                revision
            ),
            Err(CredentialCryptoError::AuthenticationFailed)
        ));
        assert!(matches!(
            cipher.decrypt(
                &cipher
                    .encrypt(
                        "key-1",
                        "installation",
                        "owner",
                        server,
                        credential,
                        CredentialKind::SshPrivateKey,
                        revision,
                        b"FAKE-SECRET-key",
                    )
                    .expect("encrypt"),
                "installation",
                "other-owner",
                server,
                credential,
                CredentialKind::SshPrivateKey,
                revision
            ),
            Err(CredentialCryptoError::AuthenticationFailed)
        ));
    }

    #[test]
    fn missing_key_does_not_fallback() {
        let cipher = cipher();
        let (server, credential, revision) = ids();
        let result = cipher.encrypt(
            "missing",
            "installation",
            "owner",
            server,
            credential,
            CredentialKind::SudoPassword,
            revision,
            b"FAKE-SECRET",
        );
        assert!(matches!(result, Err(CredentialCryptoError::KeyUnavailable)));
    }

    #[test]
    fn rotation_uses_new_key_id_without_plaintext_fallback() {
        let mut keys = HashMap::new();
        keys.insert("key-1".to_owned(), [7u8; 32]);
        keys.insert("key-2".to_owned(), [8u8; 32]);
        let provider = TestKeys(Mutex::new(keys));
        let cipher = CredentialCipher::new(provider);
        let (server, credential, revision) = ids();
        let envelope = cipher
            .encrypt(
                "key-1",
                "installation",
                "owner",
                server,
                credential,
                CredentialKind::SudoPassword,
                revision,
                b"FAKE-SECRET-password",
            )
            .expect("encrypt");
        let rotated = cipher
            .rotate(
                &envelope,
                "key-2",
                "installation",
                "owner",
                server,
                credential,
                CredentialKind::SudoPassword,
                revision,
            )
            .expect("rotate");
        assert_eq!(rotated.key_id, "key-2");
        let material = cipher
            .decrypt(
                &rotated,
                "installation",
                "owner",
                server,
                credential,
                CredentialKind::SudoPassword,
                revision,
            )
            .expect("decrypt rotated");
        assert_eq!(material.len(), b"FAKE-SECRET-password".len());
    }

    #[test]
    fn material_debug_is_redacted() {
        let material = CredentialMaterial::new(
            CredentialKind::SudoPassword,
            b"FAKE-SECRET-do-not-log".to_vec(),
        );
        let debug = format!("{material:?}");
        assert!(!debug.contains("FAKE-SECRET"));
        assert!(debug.contains("redacted"));
    }
}
