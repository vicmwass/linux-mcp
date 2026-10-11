# Spec 05 — Rust dashboard implementation

Source: [05](../05-dashboard.md). Registry: `05-dashboard.steps.json`. Follow [shared rules](IMPLEMENTATION-RULES.md). Own `crates/dashboard/`, dashboard fixture services and UI tests. Checks: format, dashboard unit/contract tests, server and `wasm32-unknown-unknown` builds, browser accessibility/workflow tests and final real-service integration. No stored credentials in hydration or browser bundles.

## 05-01 — Define page and adapter architecture

Dependencies: `00-contracts/00-12`. Resources: `dashboard-design`.

1. Map initial pages, management/MCP calls, owner-scoped caches and disconnected states to frozen contracts.
2. Select supported Leptos/Axum integration and build pipeline using official documentation during implementation.
3. Write route/component ownership, feature discovery, error presentation and credential-form boundaries in `specs/design/05/`.

Acceptance: no direct browser SSH, no Linux-execution REST fallback, and core pages require neither AI nor WebMCP.

## 05-02 — Bootstrap server, browser build and page shell

Dependencies: `05-01`, `12-client-design/12-10`. Resources: `workspace-manifests`, `dashboard-shell`, `shared-routing`.

1. Create dashboard crate, SSR/hydration features and fixture API/MCP services.
2. Implement Spec 12's published tokens, shared Leptos components, layout, persistent target selector, navigation extension points and status/error components; record the baseline version from its handoff.
3. Add backend/public feature boundary checks and build server/WASM artifacts.

Acceptance: pages render independently of MCP availability; production server cannot use fixture identity; keyboard navigation and representative visual/component checks against the design baseline pass.

## 05-03 — Implement login and backend authority adapter

Dependencies: `05-02`. Resources: `dashboard-auth`.

1. Build login/logout/session UI and backend Identity adapter against contract fixture flows.
2. Implement delegated MCP authority consumption, safe redirect handling and account-change cache clearing.
3. Test expired session, changed account, revoked grant and CSRF failures.

Acceptance: browser cookies do not masquerade as MCP tokens; no shared backend credential impersonates users. Real provider wiring waits for 09.

## 05-04 — Implement inventory and credential/trust forms

Dependencies: `05-02`. Resources: `dashboard-inventory`.

1. Build owned list/add/edit/delete pages and management client with revision conflicts/pagination.
2. Add write-only secret provisioning/rotation and observed-versus-verified fingerprint enrollment.
3. Clear secret inputs, prohibit analytics/error echoes and test foreign IDs, failed save, changed revision and empty state.

Acceptance: fixture API tests and browser forms pass; stored credentials cannot be fetched/displayed; enrollment never silently trusts observed keys.

## 05-05 — Implement MCP client lifecycle and discovery

Dependencies: `05-02`. Resources: `dashboard-mcp-client`.

1. Add backend MCP initialization, schema-version verification, discovered-tool adapters and user-bound session caching.
2. Implement reconnect/backoff, timestamped cached results and unavailable-action handling.
3. Test unsupported schema, server outage, cross-account session reuse and no mutation replay after disconnect.

Acceptance: pages remain available during MCP outage; mismatched schemas disable affected actions instead of guessing output formats.

## 05-06 — Implement capabilities, users and operation results

Dependencies: `05-05`. Resources: `dashboard-server-pages`.

1. Build capability/user inspection and supported classified-operation components using the backend MCP client.
2. Display target, bounded stdout/stderr, refusal, definite failure and unknown outcome distinctly; escape remote output.
3. Test password-lock wording, unsupported tools, malicious output and timestamps/stale revision labels.

Acceptance: no fabricated health/service features; unknown operations cannot trigger blind retry; all Linux interactions flow through MCP.

## 05-07 — Implement approval, audit and agent setup pages

Dependencies: `05-05`. Resources: `dashboard-governance`.

1. Display stored exact intent, target/revision, risk/expiry and authenticated human approval action.
2. Add owned audit timeline and operation verification presentation; separate command success from observed state.
3. Add MCP URL/grant setup and revocation UI without secrets in snippets; test tampering/expired approval and audit outages.

Acceptance: agents cannot approve themselves through page backend APIs; approval uses immutable server intent rather than descriptive agent text.

## 05-08 — Integrate fixture-backed UI and accessibility

Dependencies: `05-03`, `05-04`, `05-06`, `05-07`. Resources: `dashboard-shell`, `shared-routing`, `dashboard-ui-tests`.

1. Register page modules and run full fixture workflow from login through inventory, inspection, approval and outcome.
2. Check keyboard navigation, responsive forms, focus/error states and absence of secret hydration.
3. Test simultaneous Alice/Bob browsers, logout and disconnected pages; run standard server/WASM builds.

Acceptance: fixture-backed UI passes all 05 workflows; results are explicitly mock-backed until the next gate.

## 05-09 — Integrate real services and publish handoff

Dependencies: `05-08`, `04-mcp/04-08`. Resources: `dashboard-integration`.

1. Replace fixture adapters with real management, identity, MCP and governance services through actual delegation.
2. Exercise two users, secret/trust provisioning, disposable-target inspection, approval, revocation and outage/unknown-outcome browser workflows.
3. Publish `specs/handoffs/05.md`, build/setup commands and actual supported pages/limits.

Acceptance: all Spec 05 criteria pass with real providers; credential markers absent from browser assets/network responses; dashboard stays usable while MCP is stopped.

## Scope addition (2026-10-11): per-server approval mode

The add/edit server form includes the approval requirement setting ("Medium risk and above" default / "High risk and above") with examples, a warning for the weaker option, a note that saving invalidates pending approvals, and a read-only explanation outside an interactive session. The approval screen shows the requirement in effect. Consume the published Spec 12 baseline for visuals; the setting is requested in `design/client/review-requests/approval-mode.md`. Decision record: [contracts decisions D11](../contracts/decisions.md).
