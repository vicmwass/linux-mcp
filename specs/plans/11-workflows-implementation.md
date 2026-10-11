# Spec 11 — Workflows and optional agent extensions implementation

Source: [11](../11-workflows-and-agent-extensions.md). Registry: `11-workflows.steps.json`. Follow [shared rules](IMPLEMENTATION-RULES.md). Own `crates/workflows/`, workflow dashboard modules, namespaced migrations and explicitly selected extension modules. Checks: deterministic workflow/state tests, PostgreSQL recovery tests, MCP fixtures, browser builds and real service integration. Core release ends at 11-09; WebMCP and embedded AI are opt-in and do not block core completion.

## 11-01 — Freeze bounded workflow contracts

Dependencies: `00-contracts/00-12`. Resources: `contracts-registry`, `workflow-design`.

1. Define workflow templates, typed parameters, owned targets, bounded steps/budgets and per-step observed results.
2. Define pause/approval/dispatch/verify/unknown/reconcile states and safe resumability without mutation replay.
3. Freeze template/progress/fleet comparison schemas and fixtures in `specs/design/11/`; scope first templates to supported service/metric queries.

Acceptance: dynamic unapproved shell instructions cannot become workflow steps; all mutation authority remains the existing approval/policy boundary.

## 11-02 — Bootstrap workflow storage and runner skeleton

Dependencies: `11-01`. Resources: `workspace-manifests`, `migration-registry`, `workflow-bootstrap`.

1. Add crate, namespaced template/run/step/progress migrations and contract MCP client abstraction.
2. Implement fixture runtime/clock and opt-in extension build flags disabled by default.
3. Establish disjoint troubleshooting/fleet/mutation/page module ownership and exports.

Acceptance: migrations/crate build and fixture runner start; no model provider or browser-agent dependency is required.

## 11-03 — Implement read-only troubleshooting

Dependencies: `11-02`. Resources: `workflow-troubleshooting`.

1. Implement bounded failed-service, resource-pressure and disk-usage evidence templates against frozen MCP fixtures.
2. Recheck user authority and target revision at each query; attach collection times and explicit partial failures.
3. Test malicious outputs, tool/schema absence, revoked access and unavailable targets.

Acceptance: outputs are evidence, never instructions; unsupported actions cannot be synthesized as raw shell fallbacks.

## 11-04 — Implement fleet comparison

Dependencies: `11-02`. Resources: `workflow-fleet`.

1. Compare only explicitly selected owned targets with bounded per-user concurrency.
2. Normalize compatible resource observations and show times/revisions/unsupported cases separately.
3. Test partial fleets, revoked target access, stale caches and cancellation.

Acceptance: failed targets are visible and excluded from false complete comparisons; no cross-user enumeration.

## 11-05 — Implement guarded maintenance and recovery

Dependencies: `11-02`. Resources: `workflow-mutations`.

1. Implement typed saved maintenance steps with exact arguments/target revision and per-step policy/approval.
2. Persist dispatch/progress before action and reconcile unknown submitted work by inspection after restart.
3. Test changed arguments/revision, repeated approval, crash windows and mid-workflow revocation with fixture dispatch counters.

Acceptance: at most one dispatch attempt per consumed step approval; no automatic reset/replay and no generic rollback claim.

## 11-06 — Implement workflow pages and progress

Dependencies: `11-02`, `05-dashboard/05-02`. Resources: `dashboard-workflows`.

1. Build troubleshooting, saved workflow, fleet result and run-progress pages using fixture APIs and completed shared dashboard components/design baseline.
2. Display per-step evidence/approval/verification/unknown outcome and stop conditions.
3. Test keyboard interaction, partial results, owner-scoped history and interrupted workflow views.

Acceptance: no broad workflow-label approval substitutes for exact mutation approval; core UI works without optional agents/model.

## 11-07 — Compose real workflow adapters

Dependencies: `11-03`, `11-04`, `11-05`, `11-06`, `08-console/08-11`, `10-monitoring/10-09`. Resources: `shared-routing`, `dashboard-shell`, `workflow-integration`.

1. Register workflow management/pages and compose real identity, governance, MCP and persistence.
2. Exercise service troubleshooting/resource pressure and fleet comparisons against disposable targets.
3. Execute an approved supported maintenance step and inspect actual state; verify per-step revocation and revision checks.

Acceptance: all Linux interactions use MCP with delegated user authority; evidence records actual tools/platforms and limits.

## 11-08 — Validate restart, ambiguity and authorization

Dependencies: `11-07`. Resources: `workflow-fault-tests`.

1. Inject process/database/MCP failures before and after dispatch and restart the runner.
2. Verify persisted unknown/reconcile states, no replay and stop-on-revocation behavior.
3. Run two-user templates/history/approval isolation and partial fleet failure tests.

Acceptance: recovery tests prove no unapproved redispatch; incomplete evidence cannot be shown as verified success.

## 11-09 — Publish deterministic workflow release

Dependencies: `11-08`. Resources: `workflow-release`, `deploy-manifests`.

1. Run core workflow/database/browser suites with all optional extensions disabled.
2. Publish `specs/handoffs/11.md` with supported templates, resumability/verification semantics and operating budgets.
3. Publish core release profile and acceptance traceability; leave unselected optional steps pending.

Acceptance: Package A meets Spec 11 independently of WebMCP/AI; optional implementation cannot be claimed delivered through this gate.

## 11-10 — Assess and freeze optional WebMCP integration

Dependencies: `11-09`. Resources: `webmcp-design`. Opt-in: `webmcp`.

1. Confirm this package is selected; verify current official specification/browser support and Rust bindings feasibility.
2. Choose supported browser actions and map them to existing backend authorization/approval paths.
3. Document generated bindings, unsupported browser behavior and security tests; freeze extension schemas.

Acceptance: demonstrably supported approach is documented; unavailable platform support blocks this optional step only.

## 11-11 — Build and validate optional WebMCP adapter

Dependencies: `11-10`. Resources: `dashboard-webmcp`, `shared-routing`. Opt-in: `webmcp`.

1. Implement selected curated actions behind opt-in feature flags using the existing backend.
2. Test compatible browsers, unsupported graceful fallback, cross-user calls and attempted agent self-approval.
3. Publish `specs/handoffs/11-webmcp.md` and exact browser/version support evidence.

Acceptance: browser agent/manual actions share safeguards; default deterministic app still builds/functions with extension disabled.

## 11-12 — Design optional embedded assistant

Dependencies: `11-09`. Resources: `assistant-design`. Opt-in: `assistant`.

1. Confirm package selection and choose a maintained model-provider adapter through current official documentation.
2. Define user-configured provider credentials, outbound data consent, context/output redaction and call/time budgets.
3. Define tool orchestration through existing MCP user authority and immutable human approvals; publish adversarial fixtures.

Acceptance: self-hosted core remains independent of any provider; model output cannot change identity, targets or approval rules.

## 11-13 — Build and validate optional assistant

Dependencies: `11-12`. Resources: `workflow-assistant`, `dashboard-assistant`, `shared-routing`. Opt-in: `assistant`.

1. Implement bounded provider/tool orchestration and opt-in dashboard interface without browser provider secrets.
2. Test malicious log/file/model instructions, provider outages, user isolation and mutation self-approval attempts.
3. Run opt-in integration with explicitly configured provider/test credentials; publish `specs/handoffs/11-assistant.md` and data boundaries.

Acceptance: security/budget tests pass, provider failures leave deterministic UI usable, and disabled build needs no provider connection.
