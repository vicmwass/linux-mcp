# Spec 01 inventory design

This directory records the storage, secret, trust, lifecycle, and destination-policy
decisions for Spec 01. It is an implementation boundary for `crates/inventory`; it
does not authorize SSH execution or identity-provider integration.

## Contract inputs

The implementation consumes the frozen Spec 00 contracts:

- `Inventory` owns server CRUD, host-key observation/enrollment, revision checks, and
  owner-scoped responses.
- `TargetResolver` returns a fresh `AuthorizedTarget` only after installation,
  ownership, enabled-state, host-trust, and scope checks.
- `CredentialStore` accepts write-only inputs and returns a short-lived lease only
  for an authorized target.
- `ServerRevision` is incremented for every target-affecting mutation.

The owner and installation are always taken from the trusted `Principal`. No request
body, path parameter, hostname, credential reference, or filesystem path selects an
owner or bypasses authorization.

## Design documents

- [schema.md](schema.md) — PostgreSQL tables, constraints, indexes, and transaction
  boundaries.
- [secrets-and-trust.md](secrets-and-trust.md) — authenticated encryption,
  external-key handling, credential lifecycle, host trust, and retention.
- [destination-policy.md](destination-policy.md) — outbound network and DNS policy
  evaluated again at connection time.
- [acceptance-matrix.md](acceptance-matrix.md) — requirement-to-service/table/test
  mapping and security acceptance cases.

## Dependency choices

The inventory crate will use SQLx for typed asynchronous PostgreSQL access. SQLx's
runtime feature is selected by the crate rather than pulled into the contracts crate.
The primary documentation describes SQLx as an async SQL toolkit and requires an
explicit runtime feature for async APIs:
<https://docs.rs/sqlx/latest/sqlx/>.

Credential envelopes use RustCrypto `aes-gcm` with AES-256-GCM and authenticated
associated data. The primary documentation specifies AEAD encryption/decryption and
documents the implementation's security considerations:
<https://docs.rs/aes-gcm/latest/aes_gcm/>.

The external key-encryption/decryption boundary is an inventory configuration
trait. The database and container image never contain the wrapping/master key.
No plaintext fallback is permitted if that provider is unavailable.
