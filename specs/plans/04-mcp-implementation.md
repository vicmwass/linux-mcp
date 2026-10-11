# Spec 04 — Remote MCP service implementation

Source: [04](../04-mcp-service.md). Registry: `04-mcp.steps.json`. Follow [shared rules](IMPLEMENTATION-RULES.md). Own `crates/mcp-service/`, MCP router/composition files under `shared-routing` and service evidence. Checks: format, MCP crate tests, protocol fixtures, HTTP/auth integration, named real client and disposable-target tests.

## 04-01 — Verify transport and freeze adapter mapping

Dependencies: `00-contracts/00-12`. Resources: `mcp-design`.

1. Verify selected rmcp transport support and current official HTTP/session/auth requirements.
2. Map frozen tool schemas, versioned prepare/submit approval flow and sanitized error semantics.
3. Document origins, body/output/time/session limits and supported-client test procedure in `specs/design/04/`.

Acceptance: no unsupported library feature or undocumented protocol assumption blocks implementation; contract changes go through the contracts owner.

## 04-02 — Bootstrap mock-backed HTTP endpoint

Dependencies: `04-01`. Resources: `workspace-manifests`, `mcp-bootstrap`, `shared-routing`.

1. Add service crate, composition modules, configuration, `/mcp` and test-only contract adapters.
2. Implement initialize/discovery/request lifecycle and bounded protocol handling.
3. Add fixture client/HTTP harness; ensure mock endpoint is impossible to expose through production settings accidentally.

Acceptance: protocol fixtures pass locally with mocks; transport demo is labeled unaudited/test-only and not a release authentication claim.

## 04-03 — Implement per-request identity and session binding

Dependencies: `04-02`. Resources: `mcp-auth-session`.

1. Consume Identity interface, establish Principal on every request and bind session to subject/resource.
2. Add discovery/challenges, scope gating, origin checks and reconnect authorization.
3. Test swapped/guessed session IDs, expired/revoked authority and malformed headers using identity mocks.

Acceptance: MCP session IDs grant no access; token/body/URL user fields cannot change Principal.

## 04-04 — Implement inventory and inspection tools

Dependencies: `04-02`. Resources: `mcp-inspection-tools`.

1. Adapt list_servers, capability and user-read tools to owner-scoped interfaces.
2. Apply per-call ownership, schema/version validation, bounded output and stable errors.
3. Test bypassing discovery with foreign IDs, unsupported schemas and unavailable providers.

Acceptance: every read tool independently constrains owner; absent/foreign IDs remain indistinguishable; no credential getter is exposed.

## 04-05 — Implement mutation, approval and audit tools

Dependencies: `04-02`. Resources: `mcp-mutation-tools`.

1. Adapt user mutations/classified execution through preflight, immutable intent, approval and audit interfaces.
2. Require durable dispatch record, current authorization/revision and atomically consumed approval before submission.
3. Test confirm-only bypass, altered intents, audit outage and disconnect after submit; return approval link/status without self-approval capability.

Acceptance: all remote mutation entry points share safeguards; refusal, failed-before-submit and unknown outcomes remain distinct.

## 04-06 — Compose real adapters and resource limits

Dependencies: `04-03`, `04-04`, `04-05`, `01-inventory/01-08`, `02-identity/02-08`, `03-execution/03-08`, `06-governance/06-07`. Resources: `mcp-composition`, `shared-routing`.

1. Replace fixture providers with real inventory/identity/executor/governance adapters and register tools.
2. Enforce user/session concurrency, body/output/time limits and redacted operational diagnostics.
3. Run real-token HTTP tests for isolation, revocation, denied operations and durable unknown-outcome audit.

Acceptance: production service has no mock auth/provider fallback; real dependencies satisfy frozen contracts without route-specific bypasses.

## 04-07 — Validate real MCP client interoperability

Dependencies: `04-06`. Resources: `mcp-client-tests`.

1. Connect at least one named remote MCP client through actual OAuth discovery/authorization and HTTPS test topology.
2. Exercise initialize/discovery/read calls, approval-required flow and controlled mutation against disposable targets.
3. Reconnect, revoke grant and attempt cross-user session/target reuse; verify no automatic mutation replay.

Acceptance: record client/version, actual authorization flow and outcomes; fixture-only interoperability is insufficient.

## 04-08 — Publish service handoff and compatibility baseline

Dependencies: `04-07`. Resources: `mcp-release`.

1. Run service/root regression suites and capture supported schemas, limits and client compatibility.
2. Document production launch, dependency failures, approval URL construction and separate legacy mode behavior.
3. Publish `specs/handoffs/04.md` for dashboard/deployment teams; update actual architecture and validation.

Acceptance: all Spec 04 criteria pass with real adapters/client; network publishing remains the separately authorized deployment action.

## Scope addition (2026-10-11): per-server approval mode

Whether a call returns `approval_required` depends on the target server's `approval_mode`; MCP tokens cannot change it. Cover both modes in adapter tests. Decision record: [contracts decisions D11](../contracts/decisions.md).
