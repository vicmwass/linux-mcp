# Spec 03 mapping: current behavior → frozen contracts

Every current tool path and internal component, with the contract it maps to and what changes. "Preserve" means observable behavior and tests stay identical.

## Tool paths (all from `src/mcp/server.rs`)

| Current tool | Current path | Contract mapping | Preserve / change |
|---|---|---|---|
| `list_servers` | `SshManager::list_servers`, no network | not an Executor operation: `Inventory::list_owned` (04 adapter) | legacy adapter keeps text; remote returns owned inventory |
| `get_server_capabilities` | `system::detect_capabilities` over SSH | `Executor::inspect(&AuthorizedTarget) -> TargetInspection` | same detection commands; cache key becomes `TargetCacheKey` (F1); output formatting kept in legacy adapter |
| `list_users`, `get_user` | `users::*`, `getent passwd` | read-only operations, `ToolId` same, risk low, scope `servers:inspect` | preserve parsing/validation; no approval |
| `create_user` | validate username/home/shell → `useradd` via shared sudo | mutating tool, risk medium | preserve validation and quoting; **multi-user** default mode now needs approval (change vs legacy, by D11) |
| `disable_user`, `enable_user` | existence check, `usermod -L/-U` | mutating, medium | preserve; wording stays "password lock does not revoke SSH keys" |
| `delete_user` | `confirm` required, protected-account list, active-session check, `userdel [-r]` | mutating, high | preserve all checks; legacy `confirm` stays for legacy adapter; remote path requires approval |
| `execute_command` | capabilities → `commands::resolve` → `execution::analyze` → `SecurityPolicy::evaluate` → `ExecutionEngine::execute_structured` | `Executor::evaluate` + `submit` | see policy layering in [semantics.md](semantics.md) |

## Component map

| Current component | Destination | Notes |
|---|---|---|
| `commands::{parser, normalize, resolver, operation}` | `execution-core/commands` unchanged | 24 unit tests move with them |
| `execution::analyzer` (`ExecutionAnalysis`) | `classify` input | risk = max over operations; `Operation::Unknown` → critical → denied |
| `security::policy::SecurityPolicy` | `security/policy` unchanged | remains the source of `Denied` (catastrophic delete, unknown/critical, missing capability) |
| `capabilities::{detector, models, manager}` | `capabilities` | manager rekeyed (F1); add invalidation by `TargetInvalidation` |
| `ssh::manager::SshManager` | `transport` + `executor/pool` | pool rekeyed (F2); retry-once rule preserved (closed connection or channel-open failure only) |
| `ssh::connection::SshConnection` | `transport/russh` | host-key callback takes expected fingerprint (F3); credentials by lease (F4); bounded reads (F5) |
| `ssh::sudo` | `transport/sudo` | password only as stdin, `sudo -k -S -p ''` with `sh -c` wrapper and `</dev/null`, or `sudo -n`; stdin zeroed after use; 5 tests move unchanged; password source becomes a lease |
| `tools::{users, system}` | `tools` | take a `Transport` handle instead of `SshManager` |
| `config::ServersConfig` | `legacy` only | never used by the multi-user path |
| `mcp::server` | stays in root, calls `legacy` | formatting of text results unchanged |

## Executor contract mapping

| Contract method | Implementation outline | Notes |
|---|---|---|
| `inspect(ctx, &AuthorizedTarget)` | `ensure_current(revision)` → pooled connection (lease) → cached detection | foreign/absent targets never reach here (resolver already failed) |
| `evaluate(ctx, &OperationIntent, &AuthorizedTarget)` | recompute classification from tool + arguments + capabilities; require the caller's `risk` and resolved arguments to equal it; run `SecurityPolicy`; apply threshold | returns `PolicyEvaluation { intent_digest, server_revision, verdict, policy_version }` |
| `submit(ctx, SubmitRequest)` | re-resolve target, `authority.recheck`, lease credentials, `Audit::record_submitted`, exec, `Audit::record_outcome` | one mutating exec = the submission boundary |
| `invalidate_target(event)` | drop pooled connections and cached capabilities for the server with revision < `event.new_revision`; running operations continue | revision check at use is the backstop if an event is missed |

## Intent binding (finding F10)

`OperationIntent.risk` and `arguments` are supplied by the adapter that builds the intent, but approval binds them. The executor therefore **recomputes** them and rejects mismatches (`Denied { reason: InvalidArguments }`). To let adapters build correct intents, execution-core exports an additive planner:

```text
classify(tool: &ToolId, args: &Value, target: &AuthorizedTarget, caps: &ServerCapabilities)
    -> Result<Classification { risk, operations, canonical_arguments, resolved_command }, Refusal>
```

For `execute_command`, `canonical_arguments` contains the caller's `command` **and** the resolved/normalized `resolved_command` and operation list, so an approval binds what will actually be executed, not only what was typed (the resolver depends on capabilities, which can change without a revision change). `submit` re-resolves and refuses if the resolved command differs from the approved one. This needs no contract change; the argument keys are part of the tool's schema version.

## Legacy policy decisions → verdicts

| Legacy `PolicyDecision` / path | Multi-user verdict |
|---|---|
| `Denied` (catastrophic delete, unknown/disallowed/critical operation, missing capability error) | `Denied { PolicyDenied | CapabilityMissing | UnsupportedOperation }` — never changed by approval or mode |
| `RequiresConfirmation` (high risk, or sudo-needing when `is_root`) | replaced by the `ApprovalMode` threshold: medium+ (default) or high+ → `RequiresApproval`; legacy adapter keeps `RequiresConfirmation` exactly |
| `Allowed` | `Allowed` |
| dedicated-tool validation errors (username, protected accounts, active session) | `Refused { InvalidArguments }` at prepare; active-session check at submit prechecks → `Refused` |
