# Spec 01 secret and host-trust boundaries

## Encryption envelope

Credential plaintext is accepted only through the write-only management contract.
The adapter immediately converts it to owned backend material and encrypts it
before persistence. The envelope uses AES-256-GCM through RustCrypto `aes-gcm`.

The stored envelope contains:

```text
format_version | key_id | nonce | ciphertext_with_tag
```

Authenticated associated data binds the envelope to
`installation_id`, `owner_user_id`, `server_id`, credential kind, credential
revision, and format version. Moving ciphertext to another owner/server or
changing its metadata therefore fails authentication.

The wrapping/master key is loaded from an external key provider identified by
`key_id`; it is never stored in PostgreSQL, the image, committed configuration,
logs, diagnostics, or API responses. Key-provider errors fail the operation and
never trigger plaintext storage or a weaker cipher.

Each encryption uses a fresh random 96-bit nonce. Decryption rejects unknown
format versions, unavailable key IDs, invalid nonce lengths, authentication
failures, disabled credentials, and revision/owner mismatches.

## Rotation and recovery

Credential provisioning and rotation lock the server row, require its current
revision, write a new encrypted version, disable the previous active version, bump
the server revision, and append one invalidation event in the same transaction.
Key rotation re-encrypts using a new `key_id` and does not expose plaintext to
callers.

The provider must retain old key IDs until all envelopes using them have been
rewrapped or deliberately retired. A failed rewrap leaves the old version
readable and does not publish a new active version. Startup checks configured
key IDs before enabling credential resolution.

Secret cleanup is deferred after server deletion or credential replacement until
no legitimate lease can still be active. Lease duration is bounded by the frozen
contract (at most 60 seconds); cleanup may then remove disabled ciphertext.
Already-submitted remote commands are not recalled.

## Runtime secret boundary

Only `CredentialStore::resolve_for(&AuthorizedTarget, purpose)` can create a
`CredentialLease`. The resolver must complete owner, installation, enabled-state,
host-trust, scope, and revision checks before decryption. There is no
`get_secret(credential_id)` operation.

Leases are short-lived, non-serializable, and redacted in debug output. The
material is zeroized when its owned buffer is dropped. This does not erase copies
made by SSH/crypto libraries, allocators, swap, core dumps, or debuggers, so the
adapter minimizes copies and access duration.

## Host trust

Observation connects only far enough to obtain the presented host-key fingerprint;
it does not authenticate or execute a command and does not establish trust. The
fingerprint is returned for independent verification.

Enrollment requires the most recent observation, the current server revision, and
an exact fingerprint match. It stores `verified` trust for the owner/server/
hostname/port tuple and bumps the server revision. A changed presented key becomes
`mismatch`, publishes invalidation, and blocks all normal connections until
explicit re-enrollment.

Trust is never shared by hostname alone. Port is part of the key, and a verified
key for one owner's target cannot authorize another owner's target.
