# Spec 00 acceptance trace (step 00-12)

Every requirement of [Spec 00](../00-contracts.md) mapped to the artifact that implements it and the test that proves it. Paths are relative to `crates/contracts/` unless stated. "Real" means compiled/executed by the Rust toolchain; "simulated" means exercised against the in-memory `World` adapters, which do not prove production isolation (see [validation.md](validation.md)). Final results are recorded in [HANDOFF.md](HANDOFF.md).

## Deliverable

| Requirement | Artifact | Evidence |
|---|---|---|
| Small Rust contracts crate | `linux-mcp-contracts` 1.0.0 (`Cargo.toml`, `src/lib.rs`) | `cargo test -p linux-mcp-contracts` (real) |
| Versioned serializable DTOs | `src/public/*`, `src/canonical.rs`; versions pinned in `src/lib.rs` | `tests/public_models.rs`, `tests/operations.rs`, `tests/schema_compat.rs::pinned_versions` |
| Async service interfaces | `src/backend/services.rs` | `tests/service_traits.rs` (object safety, `Send + Sync`, no implementation dependencies) |
| Schema fixtures | `schemas/management/` (20), `schemas/mcp/` (6), `fixtures/**` | `tests/schema_compat.rs` (9), `tests/management_api.rs`, `tests/mcp_fixtures.rs` |
| In-memory test doubles | `src/test_support/{memory,scenarios}.rs` | `tests/mocks.rs` (28, simulated) |
| First integration milestone, not the full product | root `src/` untouched; no HTTP/SQL/SSH/OAuth code | `cargo test -p linux-mcp` 51 passed; `schema_compat::dependency_graph_*` |

## Required types

| Requirement | Artifact | Evidence |
|---|---|---|
| Opaque `UserId`, `ServerId`, `CredentialId`, `OperationId`, `ApprovalId`, `RequestId`, monotonic `ServerRevision` | `src/public/ids.rs` (plus `InstallationId`, `HostTrustId`, `AuditRecordId`, `Timestamp`) | `public_models`: canonical UUID parsing, bounded checked revisions, compile-fail doctests prove IDs cannot be mixed or converted |
| `Principal` built only by trusted adapters, never deserialized | `src/backend/identity.rs` (`Principal::from_verified`) | compile-fail doctests (no `Deserialize`/`Default`); `backend_boundaries`; `backend_boundaries::public_build_cannot_import_backend_modules` (real) |
| `ServerSummary` without secrets | `src/public/inventory.rs` | `public_models::server_summary_round_trips_without_secrets`; `schema_compat::response_schemas_expose_no_secret_fields` |
| `ResolvedTarget` server-side only | `src/backend/target.rs` (`ResolvedTarget`, `AuthorizedTarget`) | `backend_boundaries` (ownership-before-state, trust, cache keys); public build cannot import it |
| `OperationIntent` with owner, server revision, tool, canonical arguments, risk, expiry; secrets excluded | `src/public/operations.rs`, `src/canonical.rs` | `operations`: canonicalization, digest binds every field, secret-named keys rejected, pinned digest reproduced independently |
| `OperationOutcome` succeeded / refused / failed-before-submission / unknown-after-submission with bounded output | `src/public/operations.rs` | `operations::every_outcome_variant_round_trips`, `disconnect_after_submission_is_never_a_definite_failure`, `bounded_output_*` |
| Errors incl. foreign == absent | `src/public/errors.rs` | `public_models::foreign_and_absent_resources_are_indistinguishable`; `mocks::alice_cannot_touch_bobs_resources_and_errors_match_absent_ids` (simulated) |
| Per-server approval mode (decision D11) | `ApprovalMode` in `src/public/inventory.rs`; `ResolvedTarget.approval_mode` | `public_models::approval_mode_wire_default_and_thresholds`; `mocks::approval_mode_*`, `agents_cannot_change_or_weaken_the_approval_mode`, `changing_the_approval_mode_invalidates_pending_approvals` (simulated) |

## Service boundaries

| Interface (owner) | Contract | Evidence |
|---|---|---|
| Inventory (01), TargetResolver (01), CredentialStore (01), Identity (02), Executor (03), Approvals (06), Audit (06) | one trait each in `services.rs`, plus `InvalidationSink` and the `Services` bundle | `service_traits` (compile probes), `mocks` (every method exercised), `examples/` |
| Cancellation, timeouts, pagination | `CallContext`/`RequestContext` deadlines, future-drop cancellation semantics, `PageRequest`/`Page` | `mocks::deadlines_are_enforced_before_work`, `inventory_crud_pagination_and_delete_invalidation`, `public_models::pagination_contracts`; module docs of `services.rs` |
| Credential handles nonserializable, `Debug`-redacted, bounded lifetime | `CredentialMaterial`, `CredentialLease` (≤ 60 s) in `src/backend/credentials.rs` | compile-fail doctests; `backend_boundaries::leases_*`, `debug_output_is_redacted`; consumer probe `secrets_never_appear_*` |
| No general-purpose secret resolution to dashboard/tool handlers | only `CredentialStore::resolve_for(&AuthorizedTarget, purpose)` returns secret material | `service_traits::secrets_are_unreachable_without_an_authorized_target`; public build cannot import credentials |

## Public API conventions

| Requirement | Artifact | Evidence |
|---|---|---|
| Routes under `/api/v1` | `ROUTES` in `src/public/api.rs` (16 routes) | `management_api::route_table_is_consistent_and_complete`; `schema_compat::pinned_versions` |
| Owner from authentication, never a submitted field | all requests `deny_unknown_fields`, no owner field | `management_api::requests_reject_owner_and_unknown_fields`; `schema_compat::requests_with_owner_fields_violate_schemas` |
| Cursor pagination, UTC timestamps | `PageCursor`, `PageLimit`, `Timestamp` (RFC 3339 `Z`, ms) | `public_models::pagination_contracts`, `timestamps_are_utc_millisecond_rfc3339` |
| Updates need last revision; stale writes conflict | `expected_revision` on every mutation | `mocks::changed_revision_*`, `concurrent_stale_updates_allow_exactly_one_winner` |
| Structured errors with request ID, no SQL/secret-provider text | `PublicError` with fixed messages | `public_models::errors_are_sanitized_and_mapped` |
| Exact schemas for inventory CRUD, credential write/rotation, host-trust enrollment, approvals, audit | `schemas/management/*.schema.json`, `fixtures/management/*` | `schema_compat::management_fixtures_validate_against_their_schemas`, `schema_sets_are_exactly_the_published_surface` |
| MCP fixtures for existing tool inputs/results and versioned approval/result extensions | `fixtures/mcp/{legacy,proposed,incompatible}`, `schemas/mcp/`, `src/public/mcp.rs`, [mcp-compatibility.md](mcp-compatibility.md) | `mcp_fixtures` (legacy map equals real `tools/list`; legacy text classified; unknown versions rejected) |
| Management and MCP map to the same domain contracts | MCP adapter probe and `OperationRequest` shared | `consumer_probes::mcp_adapter_*` (simulated) |
| Host-key observation is not trust | `HostTrustStatus::Observed`, observe/enroll routes | `mocks::missing_trust_blocks_until_observed_fingerprint_is_enrolled` |

## Mock scenarios and acceptance

| Scenario | Evidence (simulated) |
|---|---|
| Alice and Bob with owned servers | `scenarios::demo()`; `mocks::alice_cannot_touch_bobs_resources_*` |
| Shared hostname, distinct credentials | `mocks::same_hostname_different_owners_never_share_credentials_or_hosts` |
| Changed revisions | `mocks::changed_revision_invalidates_targets_leases_pools_and_approvals`, `credential_rotation_*` |
| Revoked tokens | `mocks::revoked_expired_and_wrong_kind_identities_fail`, `revocation_after_approval_blocks_execution` |
| Missing trust | `mocks::missing_trust_*`, `changed_host_key_*` |
| Policy refusal | `mocks::policy_refusal_is_an_outcome_not_an_error_and_sends_nothing` |
| Approval-required | `mocks::approval_flow_agents_cannot_self_approve_and_consume_once`, `expired_approvals_*` |
| Timeout-before-submit, disconnect-after-submit | `mocks::timeout_before_submission_*`, `disconnect_after_submission_is_unknown_and_never_replayed` |
| Concurrent approval consume allows one dispatch | `mocks::concurrent_approval_consumption_allows_exactly_one_dispatch` (16 threads) |
| No fixture contains a real secret | `FAKE-SECRET-*` markers only; `mocks::fixtures_contain_only_fake_secret_markers`; `mcp_fixtures::legacy_fixtures_contain_no_server_inventory` |
| All consuming crates compile against mock adapters | six runnable examples and 13 consumer probes; standalone public-only crate builds on host and `wasm32-unknown-unknown` (real) |
| Schema round trips and compatibility fixtures pass; Rust types and wire schemas agree | `schema_compat` (9, real); schemas are regenerated from the types and compared byte-for-byte |
| Breaking changes need a contract version change and consumer migration plan | pinned constants in `src/lib.rs`; [HANDOFF.md](HANDOFF.md) compatibility policy |
| Only this spec's owner edits shared contracts | [ownership.md](ownership.md); HANDOFF change process |

## Incremental resource contracts (Specs 08–11)

Spec 00 supplies the process, not those contracts: later resource schemas (collection timestamps/units, truncation, capability requirements, verification results, approval bindings, unsupported-platform/partial-failure/stale-result/malicious-output fixtures) are added by their owning specs through the extension rules in HANDOFF. The Spec 00 types they build on (`ToolId`, `ToolSchemaVersion`, `RiskLevel`, `BoundedOutput`, `Verification`, `ApprovalMode`, versioned MCP envelope) are in place. No Spec 08–11 fixtures are claimed here.

## Gate decisions

| Decision | Record |
|---|---|
| Stronger human-approval semantics approved before freeze (README gate) | D9/D11 in [decisions.md](decisions.md); approved by the product owner 2026-10-11 as a per-server toggle, default `medium_and_above` |
| Provider/OAuth compliance, real adapters, SSH, SQL | Out of scope; Specs 01–06 |
