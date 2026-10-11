# Spec 00 contracts — integration handoff (v1.0.0)

Status: **frozen** at the revision below once the coordinator records step 00-12 completed (live coordinator state is authoritative: `python scripts/plan_coordinator.py status`). Consumers: Specs 01–06 first, then 07–12. Traceability: [acceptance.md](acceptance.md). Validation: [validation.md](validation.md). Decisions: [decisions.md](decisions.md). Ownership: [ownership.md](ownership.md). MCP compatibility: [mcp-compatibility.md](mcp-compatibility.md).

## What you get

Crate `linux-mcp-contracts` (path `crates/contracts`, workspace member). It depends on no implementation crate (no tokio, SQL, HTTP, rmcp or russh). The root `linux-mcp` binary does not use it yet; the 03 extraction adapter will.

| Feature | Contents | Use from |
|---|---|---|
| `public` (default) | IDs, revisions, timestamps, inventory/operation/API/MCP DTOs, errors, canonicalization, route table | everything, including browser builds (`wasm32-unknown-unknown` verified) |
| `schema` (default) | `schemars` derives | schema generation and tests |
| `backend` | `Principal`, `AuthorizedTarget`, credential leases, `ExecutionAuthority`, async service traits | server-side crates only |
| `test-support` | in-memory `World`, scenarios, `Driver`, `block_on` | tests, demos, mock-backed services; never production |

```toml
# browser/dashboard client code
linux-mcp-contracts = { path = "../contracts", default-features = false, features = ["public"] }
# server crates
linux-mcp-contracts = { path = "../contracts", features = ["backend"] }
# dev-dependencies for mock-backed work
linux-mcp-contracts = { path = "../contracts", features = ["test-support"] }
```

## Stable versions

| Surface | Version | Where pinned |
|---|---|---|
| Contract | `1.0.0` | `CONTRACT_VERSION`, crate version |
| Management API | `/api/v1` | `MANAGEMENT_API_VERSION`, `ROUTES` |
| MCP extension (proposed) | `linux-mcp/v1` | `MCP_EXTENSION_VERSION` |
| MCP legacy surface (current) | `linux-mcp/legacy-1` | `MCP_LEGACY_VERSION` |
| Canonicalization | `lmcp-canon-v1` | `CANONICALIZATION_VERSION` |

`tests/schema_compat.rs` fails if these, the committed schemas, the route table or any fixture disagree.

## Where to start, per consumer

| You are building | Read | Run |
|---|---|---|
| 01 inventory | `Inventory`, `TargetResolver`, `CredentialStore` in `src/backend/services.rs`; `management_api` fixtures; `ApprovalMode` rules (D11) | `examples/inventory_client.rs` |
| 02 identity | `Identity`, `Principal::from_verified`, `VerifiedIdentity` (sole trusted construction point) | `examples/identity_flow.rs` |
| 03 execution | `Executor`, `AuthorizedTarget`, `OperationIntent`, `ExecutionAuthority::recheck`, `ApprovalMode::requires_approval` | `examples/executor_flow.rs` |
| 04 MCP service | `src/public/mcp.rs`, `tests/consumer-probes/adapter.rs` (reference mapping) | `examples/mcp_adapter.rs` |
| 05 dashboard | `public` feature only, `prelude`, `ROUTES`, `schemas/management/` | `examples/dashboard_public.rs`; `tests/consumer-probes/public-consumer/` |
| 06 governance | `Approvals`, `Audit`, `ApprovalEvidence`, `AuditTicket` | `examples/governance_flow.rs` |

`test_support::scenarios::Driver` is the reference call sequence (resolve → intent → evaluate → refuse | request approval | consume → durable audit → submit). Copy its order, not its mock policy.

## Rules every consumer must follow

1. Identity comes only from a `Principal` produced by an identity adapter; never from arguments, URLs, server IDs or MCP session IDs.
2. A `ServerId` is not a target. Resolve to an `AuthorizedTarget` first; ownership failures are `not_found`, identical to absent IDs.
3. Secrets only via `CredentialStore::resolve_for` for an authorized target; leases expire (≤ 60 s) and are redacted in `Debug`.
4. Execution authority is minted only by atomic approval consumption or by a policy verdict of `allowed`; recheck revision and policy at submission.
5. After submission, a lost result is `unknown_after_submission`. Never replay. A consumed approval is never reset.
6. `confirm: true` is not approval. Approval needs an interactive browser session of the owner.
7. Per-server `ApprovalMode` (default `medium_and_above`) sets the approval threshold; changing it needs an interactive session and invalidates pending approvals. Policy denials are unaffected.
8. Audit before submission or reject the mutation with `unavailable`.

## Compatibility policy

- **Compatible (minor/patch):** new optional fields with a safe default; new modules, tools (new `ToolId`s) and routes; new enum variants only where consumers are documented to treat unknown values as an error (they are, because enums are closed — see below).
- **Breaking (major version bump + consumer migration plan):** removing or renaming a field, route or enum wire value; making an optional field required; tightening validation; changing ID, timestamp, revision or error-code representation; changing canonicalization or digest binding (also bump `CANONICALIZATION_VERSION`, which invalidates stale approvals by design); changing an MCP envelope version.
- Request and response types use `deny_unknown_fields` and closed enums. Consequence: adding a field or enum variant is not transparent to an older peer. Roll out **servers/readers first, then clients/writers**, and treat a new optional request field as unavailable until every server in the installation supports it.
- Unknown schema versions (`linux-mcp/v2`, wrong label) are rejected, never guessed.
- Committed schemas are generated, not hand-edited: `LINUX_MCP_UPDATE_SCHEMAS=1 cargo test -p linux-mcp-contracts`, review the diff, bump the relevant version if breaking.

## Extending the contract (Specs 08–11 and any later resource)

Only the contracts owner edits shared contracts (registry, public/backend models, central exports). Propose a change by adding a request to the contracts owner with: new module file path and owning resource, wire types with `deny_unknown_fields` and schema derives, fixtures (success and failure, plus unsupported-platform/partial-failure/stale-result/malicious-output cases for resource tools), a `ToolId` and `ToolSchemaVersion` for each tool, risk classification, and the approval binding implications. New modules live in their own files (declared as children of their owner file), get schemas under `schemas/<area>/`, and are added to the exact-set assertions in `tests/schema_compat.rs`. A tool name discovered at runtime is not evidence of a compatible result schema.

## Frozen artifact

Base commit: `31fb043` (`Fix execution engine integration and add shared sudo support`) plus the uncommitted working tree. **No Git commit or tag was created**; commit and tag only when the repository owner authorizes it. The frozen set is checksummed in [FROZEN.sha256](FROZEN.sha256) (hashes of file bytes with CRLF normalized to LF):

- Manifest SHA-256: `a25ba41c22e464d70f62a849786871fc8eb1af46189e4998e2e1571bd4646139`
- Files covered: everything under `crates/contracts/` (excluding build output) and `specs/contracts/*.md` except this file.
- Verify: `python scripts/verify_contracts_freeze.py verify` (exit 0 = unchanged).
- Re-freeze after an approved change: `python scripts/verify_contracts_freeze.py write`, then update the digest above.

Tests of record (2026-10-11, rustc 1.95.0): public-only 37, default 50, backend 81, all features 122 passed; `cargo test -p linux-mcp` 51 passed; clippy clean; wasm32 public builds pass. See [validation.md](validation.md) for commands and the real-versus-simulated split.

## Known limitations

- Mock adapters simulate the contract on one mutex; they prove neither production isolation, database atomicity, OAuth/OIDC validation, the 60 s revocation target, nor SSH behavior. Real adapters in 01, 02, 03 and 06 must pass the same scenarios against real services.
- `Principal`/`VerifiedIdentity` construction is a reviewed boundary, not compiler-enforced against code in the same crate family that links `backend`.
- Canonicalization rejects secret-named keys, not secrets inside free-form values such as command text. Redaction of such values is the execution/audit owners' job.
- The mock risk classifier and policy are stand-ins; real classification stays in Spec 03.
- Legacy server keys (strings) versus UUID `ServerId` mapping is an adapter concern (decisions Q1, recommended UUIDv5 per installation).
- `Page<T>` aliases share the generic schema title "Page"; the schema file names are authoritative.
- `schema_compat` uses a small validator for the keywords `schemars` emits; it does not enforce `pattern`/`format` (the Rust types do).
- rustfmt: only `linux-mcp-contracts` is formatted and checked (`cargo fmt -p linux-mcp-contracts -- --check`); root application files were never rustfmt-clean and are untouched.
- During step 00-11 the user approved installing the `wasm32-unknown-unknown` target, and `uuid` was set to `default-features = false` so wasm resolves offline.

## Open items for other specs

- Specs 01, 03, 04, 05, 06 and 12 carry the per-server approval-mode requirement (added 2026-10-11); the Spec 12 page design is an open request, [`design/client/review-requests/approval-mode.md`](../../design/client/review-requests/approval-mode.md), not yet designed.
- Provider selection and real OAuth compliance remain Spec 02/04 tasks; target MCP client compatibility is unverified.
