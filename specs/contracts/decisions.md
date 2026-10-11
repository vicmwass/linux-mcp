# Spec 00 contract decisions

Status: recorded by step 00-01 on 2026-10-11. Binding for steps 00-02 … 00-12 unless changed through the contracts owner (`contracts-design` resource). Current behavior is documented in [current-surface.md](current-surface.md); file ownership in [ownership.md](ownership.md).

Each decision lists the choice and the reason. "Open question" entries carry a recommended choice; none of them blocks implementation of the contract crate.

## D1. Crate and feature layout

- New workspace member `crates/contracts`, package name **`linux-mcp-contracts`**, library name `linux_mcp_contracts`. The root `linux-mcp` package and `src/main.rs` stay as they are; the root manifest gains a `[workspace]` table only. The root package does **not** depend on the contracts crate in Spec 00 (no runtime change).
- Features:
  - `public` (default): IDs, revisions, inventory DTOs, pagination, public errors, operation intent/outcome DTOs, management API types, MCP fixture types, canonicalization. Must build for `wasm32-unknown-unknown`. No secret, identity-construction or service-trait code.
  - `backend` = `public` + trusted `Principal`, resolved/authorized targets, credential handles, operation authority, async service traits.
  - `test-support` = `backend` + in-memory adapters, scenario seeds and mock principal construction. Opt-in only; production binaries must not enable it.
  - `schema`: JSON Schema derivation (`schemars`). Default features are `["public", "schema"]` so wire schemas are always checkable; browser builds may drop it with `default-features = false, features = ["public"]`.
- Module tree: `src/public/{ids,inventory,errors,pagination,operations,api,mcp}.rs`, `src/backend/{identity,target,credentials,operation_authority,services}.rs`, `src/test_support/…`, `src/canonical.rs` (public, owned with operations). `lib.rs` re-exports modules, never glob re-exports backend items into the public namespace.
- Dependencies (all already in `Cargo.lock`, so offline builds work): `serde` 1, `serde_json` 1, `thiserror` 2, `uuid` 1.26 (`serde`; `v4` only under `backend`), `chrono` 0.4.45 (`serde`, `std`, no `clock` in public), `schemars` 1.2 (optional, `schema`), `sha2` 0.10 (canonical digest), `async-trait` 0.1 and `zeroize` 1.9 (`backend`). No tokio, SQL, HTTP, rmcp or russh dependency in contracts.

## D2. Identity trust boundary

- `Principal` lives in `backend::identity`. It implements neither `Deserialize` nor `Default`, its fields are private, and it is constructed only through `TrustedIdentityAdapter`-gated constructors: `Principal::from_verified(VerifiedIdentity)`, where `VerifiedIdentity` is a backend type that identity adapters (Spec 02) produce after validating issuer, audience, signature, lifetime and scopes. The constructor is named and documented as the single trusted-adapter boundary; reviewers treat any other call site outside identity crates and `test_support` as a defect.
- `Principal` carries: `UserId`, `InstallationId`, `Vec<Scope>` (closed enum: `inventory:read`, `inventory:write`, `credentials:write`, `servers:inspect`, `servers:mutate`, `approvals:request`, `approvals:decide`, `audit:read`), `SessionRef` (opaque session/token identifier, with `AuthKind::{BrowserSession, McpToken, DelegatedToken, LegacyLocal}`), `authenticated_at` and `expires_at`.
- Identity never comes from tool arguments, URL parameters, server IDs or MCP session IDs. Public request DTOs have no owner field; adding `owner_user_id` to a request type is rejected by tests (`deny_unknown_fields` on all request DTOs).
- Browser/public builds do not compile `backend`, so they cannot name `Principal`, credentials or service traits.

## D3. IDs, revisions and time

- IDs (`UserId`, `ServerId`, `CredentialId`, `OperationId`, `ApprovalId`, `RequestId`, `InstallationId`, `HostTrustId`, `AuditRecordId`) are distinct newtypes over `uuid::Uuid`. Wire form: lowercase hyphenated UUID string. Parsing accepts only that canonical form (rejects braces, URNs, uppercase, nil UUID). No `From<Uuid>` across types; conversion between ID types does not exist.
- `ServerRevision(u64)`: starts at 1, wire form JSON integer, bounded to `1..=2^53-1` for JavaScript safety; `next()` is checked and returns an error on overflow. Revisions are per server, monotonically increasing on every change to inventory, credential binding or host trust.
- Timestamps: `chrono::DateTime<Utc>` serialized as RFC 3339 with `Z`, millisecond precision (`Timestamp` newtype truncates on construction so round trips are exact). Durations on the wire are integer milliseconds.
- Legacy stdio server keys (free-form strings) are not `ServerId`s. **Open question Q1** — recommended: the legacy adapter (Spec 03) keeps string keys and maps them to `ServerId` with UUIDv5 over a per-installation namespace; contracts expose `LegacyServerKey(String)` only inside MCP compatibility fixtures.

## D4. Async trait objects

- Service traits use `#[async_trait]` (`Send` futures) and are object safe; consumers hold `Arc<dyn Inventory>` etc. Native `async fn` in traits is not dyn-compatible on the current toolchain (Rust 1.95). Every method takes `&RequestContext` first.
- Traits require `Send + Sync + 'static`. Errors are typed (`ServiceError`), never `anyhow`.

## D5. Schema library and wire versioning

- `schemars` 1.x (JSON Schema 2020-12), the version rmcp 3.2 already uses, so MCP tool schemas and management schemas come from one generator.
- Generated schemas are committed under `schemas/management/` and `schemas/mcp/`; a test regenerates and compares them (set `LINUX_MCP_UPDATE_SCHEMAS=1` to rewrite). Fixtures live under `fixtures/` and round-trip through the Rust types.
- Versions: `CONTRACT_VERSION = "1.0.0"` (semver; breaking change = major), management API path `/api/v1`, MCP extension schema `linux-mcp/v1`, canonicalization `lmcp-canon-v1`. Unknown major versions are rejected, never guessed.

## D6. Cancellation, deadlines and submission boundary

- `RequestContext` carries `RequestId`, an absolute `deadline: Timestamp`, and (backend) the `Principal`. Implementations check the deadline before each external step.
- Cancellation is future drop (runtime-agnostic, no tokio in contracts). Contract guarantee: if a future is dropped or the deadline expires **before** the executor records `SubmissionState::Submitted`, the outcome is `FailedBeforeSubmission` and nothing ran. After submission the only truthful outcome for a lost result is `UnknownAfterSubmission`; cancellation cannot recall a submitted remote command, and the system never replays it.
- Executors record the submission boundary through the `Audit` trait before sending the exec request (D9).

## D7. Secret-handle lifecycle

- `CredentialRef` (public-safe reference: `CredentialId`, kind, revision, fingerprint label) is serializable; secret material is not.
- `CredentialMaterial` (backend): not `Clone`, not `Serialize`, `Debug` prints `CredentialMaterial(<redacted>)`, zeroized on drop (`zeroize`). Obtained only from `CredentialStore::resolve_for(&AuthorizedTarget, purpose)` as a `CredentialLease` with `expires_at` (default ≤ 60 s) and scoped access `lease.with_secret(|bytes| …)`; the lease refuses access after expiry.
- Clearing caveats are documented on the type: zeroize clears the owned buffer only; copies made by SSH libraries, the allocator, swap or core dumps are outside its control.
- No general `get_secret(id)` exists. Credential references cannot be file paths; mocks hold fake markers (`FAKE-SECRET-…`).

## D8. Error model and result mapping

- `ErrorCode` (closed enum, snake_case on the wire): `unauthenticated` 401, `forbidden` 403, `not_found` 404, `validation` 422, `revision_conflict` 409, `approval_required` 428, `unavailable` 503, `timeout` 504, `outcome_unknown` 502.
- `PublicError { code, message, request_id, details? }`; `details` is a closed enum (field violations, current revision for conflicts, approval reference, retry-after ms). Messages are fixed per code plus validated field paths; provider/SQL/SSH strings never pass through.
- Foreign-owned and absent resources both return `not_found` with identical message and details. `forbidden` is used only for scope failures that do not depend on the resource's existence.
- Policy refusal is **not** an error: it is `OperationOutcome::Refused { reason }`, returned as a successful call result. `approval_required` is an error only when an unapproved mutation is submitted; the prepare flow returns a pending approval status instead.
- MCP mapping: protocol errors are reserved for malformed requests/unknown tools; typed outcomes and `PublicError`s are returned as structured tool results with `isError: true` for errors and refusals.

## D9. Operation canonicalization and approval invariants

- `OperationIntent { operation_id, installation_id, owner, server_id, server_revision, tool: ToolId, tool_schema_version, arguments: CanonicalArguments, risk, created_at, expires_at }`. Secrets are excluded by construction: argument schemas mark sensitive fields and canonicalization rejects them.
- Canonical form `lmcp-canon-v1`: JSON with object keys sorted by UTF-8 bytes, no insignificant whitespace, strings as-is (no Unicode normalization), integers only (floats rejected), `null` members removed, max depth 16, max 64 KiB. Digest = SHA-256 over `"lmcp-canon-v1\n" || canonical_json(binding)` where `binding` is the intent minus `operation_id`/`created_at`. A version bump changes the prefix and so invalidates stale approvals.
- Layers are separate types: `OperationIntent` (requested) → `PolicyEvaluation` (preflight, from Executor) → `ApprovalEvidence` (human decision, from Approvals) → `ExecutionAuthority` (backend-only, minted by `Approvals::consume` or by policy for operations that do not need approval). `ExecutionAuthority` has no public constructor, is not `Clone`/`Serialize`, and is consumed by `Executor::submit`.
- Which operations need approval is decided per server by `ApprovalMode` (D11), resolved fresh into the target on every operation.
- Invariants: (1) approval binds identity, installation, server, revision, tool, schema version, canonical arguments, risk and expiry through the digest; (2) consume is atomic and single-use; (3) any change → new intent and approval; (4) approval never overrides a policy denial — policy is re-evaluated at submit; (5) a consumed approval is never reset after ambiguity; (6) `confirm: true` and mutation scope never create approval evidence; (7) audit records `SubmissionPending` durably before dispatch, or the mutation is rejected with `unavailable`.

## D10. Outcomes and bounded output

- `OperationOutcome::{Succeeded, Refused, FailedBeforeSubmission, UnknownAfterSubmission}`. `Succeeded` means the remote command completed and reported an exit status (non-zero is still `Succeeded` at transport level; `exit_status` is separate). `verification` is a separate optional field.
- `BoundedOutput { text, truncated, original_bytes }`; default limit 1 MiB per stream (matches `ExecutionLimits`), truncated on a UTF-8 boundary; invalid UTF-8 is replaced and flagged.

## D11. Per-server approval mode

Decided by the product owner on 2026-10-11 (resolving open question Q2): approval for remote mutations is a per-server setting, stored with the server configuration, with two modes and a safe default.

| `ApprovalMode` | Wire value | Needs human approval |
|---|---|---|
| `MediumAndAbove` (default) | `medium_and_above` | medium, high and critical risk |
| `HighAndAbove` | `high_and_above` | high and critical risk only |

- Low risk never needs approval. Critical risk and unknown/denied operations are refused by policy under both modes; the mode only moves the approval threshold and can never approve a denial. `confirm: true` is never approval evidence under either mode.
- Field: `approval_mode` on `ServerSummary`, `CreateServerRequest` (optional, defaults to `medium_and_above`) and `UpdateServerRequest` (optional); `ApprovalMode::requires_approval(risk)` is the single threshold function; `ResolvedTarget.approval_mode`/`AuthorizedTarget::approval_mode()` carry the current value to the executor's preflight and submit-time recheck.
- Changing it is a security-relevant edit: **only an interactive browser session** may set a non-default value on create or change it on update (`forbidden` for MCP and delegated tokens, even with inventory-write scope). An agent therefore cannot turn approvals off for itself. Other edits remain available to tokens with inventory-write scope.
- A change bumps the server revision and publishes a `TargetInvalidation`, so any approval bound to the old revision is invalidated (`binding_mismatch`). Re-requesting under the new mode is required.
- Sudo is not a separate trigger: privileged operations are classified medium or above, so the mode governs them.
- Legacy stdio mode is unchanged (single-user `confirm` semantics); it has no `approval_mode`.
- Per-owner or installation-wide defaults, and per-operation overrides, are out of scope for v1; the contract can add them as new optional fields without breaking consumers.

Verified by `tests/public_models.rs::approval_mode_wire_default_and_thresholds` and `tests/mocks.rs` (`approval_mode_defaults_to_medium_and_above_and_toggles_per_server`, `agents_cannot_change_or_weaken_the_approval_mode`, `changing_the_approval_mode_invalidates_pending_approvals`). Simulated adapters only; Specs 01, 03 and 06 must implement the same rules (see their approval-mode requirements).

## Open questions (non-blocking, recommended choice recorded)

- **Q1** legacy server keys — see D3.
- **Q2** (resolved 2026-10-11) human-approval semantics for remote mutations: approved by the product owner with the per-server `ApprovalMode` toggle described in D11 (default `medium_and_above`, optional `high_and_above`). No longer blocks the 00-12 freeze.
- **Q3** default lease lifetime 60 s and output limit 1 MiB are contract defaults; Spec 03 may lower them.
- **Q4** `wasm32-unknown-unknown` target is not installed on this host; step 00-11 needs it (`rustup target add wasm32-unknown-unknown`).
