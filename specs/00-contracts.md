# 00 — Shared contracts and mock kit

Implementation sequence: [12-step agent plan](plans/00-contracts-implementation.md). Use the [claim/progress workflow](plans/README.md) before implementing any step; completion is recorded in coordinator state, not inferred from this proposal.

## Deliverable

A small Rust contracts crate with versioned serializable DTOs, async service interfaces, schema fixtures and in-memory test doubles. This is the first integration milestone, not a full implementation of the product.

## Required types

- Opaque `UserId`, `ServerId`, `CredentialId`, `OperationId`, `ApprovalId`, `RequestId` and monotonically increasing `ServerRevision`.
- `Principal`: authenticated subject, scopes, session/token identifier and installation identity. Constructed only by trusted authentication adapters; never deserialized from tool arguments.
- `ServerSummary`: ID, display name, hostname, port, username, environment, revision, enabled state, approval mode and host-trust status. No secret or credential content.
- `ApprovalMode`: per-server setting, stored with the server configuration, that decides which mutations need human approval: `medium_and_above` (default) or `high_and_above`. It is part of create/update requests and summaries, lowers or raises only the approval threshold (never a policy denial, and `confirm: true` is never approval), is resolved fresh into `ResolvedTarget` on every operation, and bumps the server revision when changed so pending approvals are invalidated. Changing it away from the default requires an interactive browser session, never an MCP or delegated token.
- `ResolvedTarget`: owner, current revision, network target, host-trust reference and credential references; server-side only.
- `OperationIntent`: operation ID, owner, server ID/revision, tool name, canonical arguments, risk and expiry. Secrets are excluded.
- `OperationOutcome`: succeeded, refused, failed-before-submission, or unknown-after-submission; optional exit status and bounded stdout/stderr.
- Errors: unauthenticated, forbidden, not-found, validation, revision-conflict, approval-required, unavailable, timeout and outcome-unknown. Foreign-owned and absent server IDs are indistinguishable to callers.

## Service boundaries

Freeze compilable signatures for these operations, including cancellation, timeouts and pagination:

| Interface | Required operations | Implementation owner |
|---|---|---|
| Inventory | list owned, create, get owned, update expected revision, delete | 01 |
| TargetResolver | resolve authorized target from principal and server ID | 01 |
| CredentialStore | provision, resolve for authorized target, rotate, delete | 01 |
| Identity | authenticate browser/request, validate MCP token, revoke | 02 |
| Executor | inspect, evaluate intent, submit authorized operation, invalidate target | 03 |
| Approvals | request, approve, atomically consume bound approval | 06 |
| Audit | begin durable operation record, record submission/outcome, list owned | 06 |

Backend credential handles must be nonserializable, redact Debug output and have bounded lifetimes. Do not expose general-purpose secret resolution to dashboard or tool handlers.

## Public API conventions

Version management routes under `/api/v1`. Owner comes from authentication, never a submitted `owner_user_id`. Use cursor pagination and UTC timestamps. Updates require the last observed revision and return conflict on stale writes. Use structured errors with request ID; do not return internal SQL or secret-provider errors.

Publish exact request/response schemas for inventory CRUD, credential write/rotation, host-trust enrollment, operation approval and audit queries. Publish MCP fixtures for existing tool inputs and results, plus versioned approval/result extensions. Management and MCP adapters map to the same domain contracts.

## Mock scenarios and acceptance

Ship users Alice and Bob, owned servers for each, a shared hostname with distinct credentials, changed revisions, revoked tokens, missing trust, policy refusal, approval-required, timeout-before-submit and disconnect-after-submit. No fixture contains real secrets.

The approval-mode scenario is part of the kit: the same operation needs approval on a default server and runs on a `high_and_above` server, high-risk operations and denials are unaffected, an agent token cannot change the mode, and a mode change invalidates an approved-but-unused approval.

All consuming crates compile against mock adapters. Schema round trips and compatibility fixtures pass. Rust types and wire schemas agree. Breaking changes require a contract version change and consumer migration plan. Only this spec's owner edits shared contracts; other workstreams propose changes through that owner.

## Incremental resource contracts

Specs 08–11 extend the same contract process per milestone. Freeze resource query/mutation schemas, collection timestamps/units, pagination/truncation, capability requirements, verification results and approval bindings before parallel server/UI work. Add unsupported-platform, partial-failure, stale-result and malicious-output fixtures. A tool name discovered at runtime is insufficient evidence that its result schema is compatible.
