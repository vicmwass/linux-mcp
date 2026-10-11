# Spec 01 — Inventory, secrets and trust implementation

Source: [01](../01-inventory-and-secrets.md). Registry: `01-inventory.steps.json`. Follow [shared implementation rules](IMPLEMENTATION-RULES.md) and [coordination](README.md). Status: planned; not implemented.

Own `crates/inventory/{src,migrations,tests}`, `specs/design/01/` and step evidence. Standard checks: `cargo fmt --all -- --check`, `cargo test -p linux-mcp-inventory`, root regression tests at integration, and PostgreSQL migration/integration tests against a disposable database. Initial adapters use contract mocks; real identity integration is explicit below.

## 01-01 — Map storage and credential boundaries

Dependencies: `00-contracts/00-12`. Resources: `inventory-design`.

1. Read frozen contracts and identify exact CRUD, resolver, secret and trust interfaces.
2. Design owner/installation constraints, revision transactions, encrypted envelope/key IDs, deletion/in-flight semantics and outbound policy.
3. Write schema, retention, rotation and threat-boundary decisions in `specs/design/01/`; select maintained crypto dependencies using primary documentation during implementation.

Acceptance: every 01 requirement maps to a table/service/check; no arbitrary secret path or foreign credential-reference API; missing critical contract changes are resolved through the contract owner before completion.

## 01-02 — Bootstrap crate and migration ownership

Dependencies: `01-01`. Resources: `workspace-manifests`, `migration-registry`, `inventory-bootstrap`.

1. Add the crate, SQLx setup, typed configuration and namespaced migrations for servers, credentials, trust and invalidation events.
2. Register migration order without editing identity/governance tables; user subjects may be opaque IDs until 02 integrates.
3. Add temporary PostgreSQL test setup and export placeholders for separately owned modules.

Acceptance: fresh migrations and restart/reapply pass; crate builds; root still builds. No migration silently destroys user data.

## 01-03 — Implement owner-scoped inventory repository

Dependencies: `01-02`. Resources: `inventory-repository`.

1. Implement CRUD, stable pagination and expected-revision updates in `src/repository/`.
2. Enforce owner/installation constraints in every lookup and transaction; absent/foreign IDs share public errors.
3. Add concurrent-write, duplicate-hostname, pagination-isolation and restart persistence tests.

Acceptance: real PostgreSQL tests prove Alice/Bob isolation and stale-write conflicts; no submitted owner field controls access. Publish repository adapter examples.

## 01-04 — Implement encrypted credential store

Dependencies: `01-02`. Resources: `inventory-credentials`.

1. Implement write-only provisioning, authenticated encryption, external key loading and redacted secret handles in `src/credentials/`.
2. Implement credential/key rotation with version references and bounded decrypted lifetime; design cleanup for in-flight handles.
3. Test missing/wrong key, tampered ciphertext, foreign references, redaction and rotation recovery.

Acceptance: database-only export cannot decrypt secrets; logs/debug/responses expose no plaintext; key material is absent from image/config examples; crypto failure never falls back to plaintext.

## 01-05 — Implement host trust and destination policy

Dependencies: `01-02`. Resources: `inventory-trust-policy`.

1. Implement observed versus independently verified host-key enrollment scoped to owner/host/port in `src/trust/`.
2. Implement deployment-configured network/port policy, DNS-result validation and connection-time policy evidence in `src/destinations/`.
3. Test key changes, unknown trust, IPv4/IPv6, alternate DNS addresses and blocked infrastructure destinations.

Acceptance: unverified/changed keys fail closed; self-hosted private destinations require configured allowance; policy applies again to the actual address used by execution.

## 01-06 — Compose authorized resolution and lifecycle events

Dependencies: `01-03`, `01-04`, `01-05`. Resources: `inventory-resolver`, `inventory-exports`.

1. Compose TargetResolver/CredentialStore under trusted Principal and current revision in `src/resolver/`.
2. Commit inventory/trust/credential revision changes and invalidation publication atomically, with missed-event revision fallback.
3. Implement delete/disable semantics and safe secret cleanup; document already-submitted command behavior.

Acceptance: foreign IDs fail before secret resolution; rotation/deletion affects subsequent resolutions immediately; transaction fault tests cannot publish an uncommitted target revision.

## 01-07 — Expose management handlers and mock demo

Dependencies: `01-06`. Resources: `inventory-http`, `shared-routing`.

1. Add contract-conforming CRUD/credential/trust handlers with pluggable authenticated Principal extraction.
2. Validate body sizes, revision preconditions, errors and write-only secret forms; ship a clearly test-only mock identity demo.
3. Exercise HTTP schema fixtures and concurrency tests; ensure no handler executes Linux commands.

Acceptance: management API contract suite passes; production configuration cannot enable mock authentication; credentials never echo in errors/responses.

## 01-08 — Validate real identity and publish handoff

Dependencies: `01-07`, `02-identity/02-08`. Resources: `inventory-integration`.

1. Run two-user real-token management tests with identity adapter and PostgreSQL, including credential/trust guesses and revocation.
2. Restart without rebuilding and verify inventory/rotation persistence and revision changes.
3. Run standard checks and publish `specs/handoffs/01.md`, deployment requirements and actual architecture behavior.

Acceptance: all Spec 01 criteria pass with real identity/storage adapters; SSH cache invalidation is verified later at 04/07, not claimed proven by repository tests.

## Scope addition (2026-10-11): per-server approval mode

Store `approval_mode` per server (default `medium_and_above`; migration default for existing rows); expose it on create/update/summary; return it in every `ResolvedTarget`; restrict create-with-non-default and update to interactive browser sessions (MCP/delegated tokens get `forbidden`); a change bumps the revision and publishes an invalidation. Tests: default, toggle, agent token refused, stale approval invalidated. Decision record: [contracts decisions D11](../contracts/decisions.md).
