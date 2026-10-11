# MCP compatibility (Spec 00, step 00-08)

Types: `crates/contracts/src/public/mcp.rs`. Fixtures: `crates/contracts/fixtures/mcp/` (`legacy/` captured, `proposed/` and `incompatible/` hand-written). Schemas: `crates/contracts/schemas/mcp/`. Tests: `crates/contracts/tests/mcp_fixtures.rs`. This document does **not** claim live MCP transport interoperability; no HTTP/OAuth transport exists.

## Versions

| Label | Meaning | Status |
|---|---|---|
| `linux-mcp/legacy-1` | Today's stdio server: nine tools, text-only results | Implemented; captured from the real binary |
| `linux-mcp/v1` | Remote extension: structured results + prepare/submit/status tools | Proposed |

Unknown labels (`linux-mcp/v2`, missing, wrong label on a v1 body) are rejected, never guessed. A client that does not see the `linux-mcp` entry under `capabilities.experimental` in `initialize` must assume legacy behavior.

## Legacy facts (from captured output, 2026-10-11)

- `initialize` echoes the client's requested protocol version; capabilities `{"tools": {}}`; server info `rmcp 3.2.0`.
- Policy refusals, missing servers, validation and SSH failures are **successful** results with `isError: false` and text. Only argument deserialization failure sets `isError: true`; an unknown tool is JSON-RPC error `-32602`.
- `list_servers` takes no arguments; the other eight take `server_id`. `delete_user` requires `confirm` and `remove_home`; `execute_command` takes optional `confirm`.

## Mapping to shared contracts

| Legacy tool | Shared `ToolId` | Risk | Notes |
|---|---|---|---|
| `list_servers` | same | low | Remote: owner-scoped inventory, not a command |
| `get_server_capabilities`, `list_users`, `get_user` | same | low | read-only inspection (`servers:inspect`) |
| `create_user`, `enable_user`, `disable_user` | same | medium | mutation (`servers:mutate`) |
| `delete_user` | same | high | mutation; requires approval remotely |
| `execute_command` | same | classifier decides | `Operation`/`RiskLevel` from the existing command classifier |

Argument objects become `CanonicalArguments` (secret-named keys rejected, no floats). `server_id` becomes a UUID `ServerId` remotely; legacy string keys stay inside the legacy adapter (decisions.md Q1).

## Adapter for unstructured text

`classify_legacy_text(tool, text, is_error)` recovers: `ExecCompleted{exit_code}`, `ToolSucceeded`, `ConfirmationRequired`, `Refused`, `Failed`, `PossiblyUnknownOutcome` (timeout text "remote outcome may be unknown"), `InvalidArguments`, else `Unrecognized` — never assumed success. It is prefix-based and therefore fragile to wording changes; the long-term fix is the structured v1 result. A legacy `Failed` for `execute_command` after submission cannot be distinguished from a pre-submission failure unless it matches the timeout phrase; adapters treat anything not provably pre-submission as possibly run.

## Proposed v1 extension

- New tools: `prepare_operation` (same shape as the management `OperationRequest`), `submit_operation {operation_id, approval_id?}`, `get_operation_status {operation_id}`. Legacy tools are untouched.
- `ToolResultEnvelope {schema_version, request_id, result}` with `result.kind` = `outcome` | `approval_required` | `error`. `isError` is true for `error`, `refused`, `failed_before_submission` and `unknown_after_submission`; a non-zero exit is still a `succeeded` outcome.
- `approval_required` returns the approval ID, status, intent digest, expiry and a dashboard-relative path, never credentials.

## Approval semantics

`confirm: true` remains the legacy agent-intent flag (`LEGACY_CONFIRM_IS_APPROVAL_EVIDENCE = false`). It is never accepted as approval evidence on the remote service: remote mutations needing approval return `approval_required`; only a human decision in the dashboard, consumed atomically, mints authority. Legacy stdio behavior is preserved exactly (including `confirm` semantics) for the single-user adapter.
