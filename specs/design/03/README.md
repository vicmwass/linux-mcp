# Spec 03 design — execution-core extraction and boundaries (step 03-01)

Status: design, 2026-10-11. Inputs: [Spec 03](../../03-execution-core.md), the current source at commit `31fb043` (read in full for this step), frozen contracts ([HANDOFF](../../contracts/HANDOFF.md)). Companion documents: [mapping.md](mapping.md) (current code → contracts, with the gaps found), [semantics.md](semantics.md) (submission boundary, outcomes, limits, pool/cache keys, policy layering), [regression.md](regression.md) (baseline to preserve). No product code changes in this step.

## Goal and non-goals

`crates/execution-core` becomes a tenant-aware library implementing the `Executor` contract (`inspect`, `evaluate`, `submit`, `invalidate_target`) on top of the existing, working parsing, resolution, capability detection, policy, user tools and SSH code. The existing stdio server keeps working through a **legacy adapter**. Non-goals for 03: HTTP, OAuth, inventory storage, approvals persistence, audit persistence (Specs 04/01/02/06), and any weakening of today's policy or secret handling.

## Target crate layout (decided; created in 03-02 and later)

```text
crates/execution-core/
  src/
    commands/      parser, normalize, resolver, operation   (moved unchanged)
    capabilities/  detector, models, cache keyed by TargetCacheKey (cache moves from manager)
    security/      policy (SecurityPolicy::evaluate unchanged) + threshold layer (new)
    classify/      planner: tool+args+capabilities -> Classification (new, additive)
    tools/         users, system (moved; take a Transport, not SshManager)
    transport/     SshTransport trait; russh adapter; fake adapter (tests)
    executor/      Executor implementation: preflight, submit, pool, limits
    legacy/        single-user adapter: servers.toml, file-path secrets, text results
```

Root package `linux-mcp` keeps `src/main.rs` and `src/mcp/server.rs`; after 03-02 it depends on `execution-core`'s `legacy` module only. No module of the new library imports the root crate. `linux-mcp-contracts` is used with `backend` only; the library never enables `test-support` outside dev-dependencies.

## Findings that shape the work (verified in source)

| # | Finding | Where | Consequence |
|---|---|---|---|
| F1 | Capability cache is keyed by `server_id` string only | `capabilities/manager.rs` | Must be keyed by `TargetCacheKey` (owner, server, effective SSH user, revision); otherwise two owners with the same label share data |
| F2 | Connection pool key is `"{server_id}::{username}"` | `ssh/manager.rs` | Same: use `TargetCacheKey`; revision bump already covers credential, trust and target changes |
| F3 | Host-key check reads a process-wide `known_hosts` file | `ssh/connection.rs` | Replace with the verified fingerprint carried by `HostTrustRef`; mismatch fails closed. File-based check stays in the legacy adapter only |
| F4 | Private key and sudo password are file paths from `ServerConfig` | `config/servers.rs`, `ssh/connection.rs`, `ssh/sudo.rs` | Multi-user path takes `CredentialLease`s; paths exist only in the legacy adapter |
| F5 | `ExecutionLimits` declares 1 MiB output caps but the SSH read loop collects everything | `execution/limits.rs`, `ssh/connection.rs` | Enforce bounds while reading; report `BoundedOutput.truncated` |
| F6 | Any error after `channel.exec` (including "no exit status") is a generic failure | `ssh/connection.rs`, `ssh/manager.rs` | Must map to `UnknownAfterSubmission`; the legacy text may stay, the typed outcome may not lie |
| F7 | The timeout wraps connect + queue + exec and evicts the connection | `ssh/manager.rs` | Keep; add the submission marker so timeout before exec = `FailedBeforeSubmission`, after = `UnknownAfterSubmission` |
| F8 | Dedicated user tools make several remote calls (existence, session check, then the sudo mutation) | `tools/users.rs` | Only the single privileged mutating exec crosses the submission boundary; prechecks are read-only inspection |
| F9 | `SecurityPolicy::evaluate` returns `Allowed`/`RequiresConfirmation`/`Denied` from risk and sudo | `security/policy.rs` | Keep as the denial source. The multi-user approval decision uses `ApprovalMode` (D11), not the legacy confirmation rule |
| F10 | The frozen `Executor` trait has no "classify" step, yet an `OperationIntent` needs trustworthy `risk` and resolved arguments | `crates/contracts` | Add an additive planner in execution-core (not a contract change) and make `evaluate` recompute and verify |
| F11 | Host-key observation (Spec 01 route) needs an SSH handshake without authentication, and connect-time destination checks need DNS pinning | Spec 01, `ssh/connection.rs` | 03 provides the transport primitives; 01 needs them through an interface. See open question Q-03-1 |
| F12 | Unused `ExecutionRequest`/`ExecutionPlan`/`Script` types exist | `execution/{request,plan}.rs` | Not carried into the library unless a step needs them; listed for removal in 03-02 evidence |

## Legacy versus multi-user authority

| Aspect | Legacy single-user adapter | Multi-user path |
|---|---|---|
| Caller identity | fixed local `Principal` (`AuthKind::LegacyLocal`); no scopes enforced beyond tool list | `Principal` from Spec 02, scopes + ownership |
| Target | entries of `config/servers.toml`; string IDs mapped to UUIDv5 per installation (decision Q1) | `AuthorizedTarget` from `TargetResolver` |
| Credentials | key/sudo-password **file paths** read by the adapter | `CredentialLease` only |
| Host trust | `known_hosts` file | verified fingerprint on the resolved target |
| Approval | tool-local `confirm` exactly as today (legacy `SecurityPolicy::evaluate`) | `ApprovalMode` threshold, human approval via Spec 06 |
| Result text | unchanged strings (the 00-08 fixtures) | typed `OperationOutcome` |
| Isolation | none required (single user) | cache/pool/trust keys per `TargetCacheKey` |

The two share parsing, resolution, classification, policy denial, sudo handling and the SSH implementation; they differ only in the adapter in front. The legacy adapter must never be reachable from the remote service and must not gain approvals.

## Open questions (recorded, not blocking 03-01)

- **Q-03-1 host-key probe and destination guard.** `Inventory::observe_host_key` (Spec 01) needs a handshake; connection-time destination re-checks (Spec 01) need to run in the transport. Recommended: 03 exports a `HostKeyProbe` and accepts an injected `DestinationGuard`; because components may depend only on contracts, the trait definitions need an additive contracts extension request (HANDOFF process) before 01-05/03-03 implement them.
- **Q-03-2 per-user concurrency numbers.** Default recommended: 4 concurrent operations per user, 1 in-flight mutation per server (the connection mutex already serializes per connection); confirm in 03-06.
- **Q-03-3 output drain policy** after the cap is reached: recommended to keep draining until channel close (bounded by the operation timeout) but discard bytes, so the exit status is still observed.
