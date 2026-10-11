# Spec 06 — Approvals and audit implementation

Source: [06](../06-approvals-and-audit.md). Registry: `06-governance.steps.json`. Follow [shared rules](IMPLEMENTATION-RULES.md). Own `crates/governance/`, namespaced migrations and `specs/design/06/`. Checks: format, governance tests, disposable PostgreSQL concurrency/crash tests, schema fixtures and real identity adapter validation. Executor may be mocked here; real command dispatch is a 04/07 gate.

## 06-01 — Define durable approval/dispatch state machine

Dependencies: `00-contracts/00-12`. Resources: `governance-design`.

1. Map immutable intent, expiry, approval, atomic consumption, dispatch-pending, confirmed submission and outcome transitions.
2. Specify transaction boundaries between consuming approval and durable audit, including crash windows and policy recheck.
3. Define redaction/retention rules and recovery that treats ambiguous dispatch as unknown, never replayable.

Acceptance: state diagram/tests account for every crash point; no transaction claim implies exactly-once remote execution across SSH/network failures.

## 06-02 — Bootstrap tables and repository primitives

Dependencies: `06-01`. Resources: `workspace-manifests`, `migration-registry`, `governance-bootstrap`.

1. Add crate and approval/operation/audit migrations with owner/installation and unique consumption constraints.
2. Define repository module placeholders and disposable PostgreSQL test setup.
3. Verify migrations/restart and schema constraints; keep plaintext outputs/secrets out of default columns.

Acceptance: crate/migrations pass; retention policy and per-crate migration ownership are documented.

## 06-03 — Implement bound approval service

Dependencies: `06-02`. Resources: `governance-approvals`.

1. Persist immutable canonical intents and authenticated human decision evidence; separate request/status from approve authority.
2. Implement atomic one-time consumption with current identity, arguments, target revision and expiry checks.
3. Test concurrent consumption, foreign/expired/altered intent, revoked permissions and confirm-only attempts.

Acceptance: one approval admits at most one dispatch attempt; approval cannot override policy denial or mint authority from an arbitrary ID.

## 06-04 — Implement durable audit and recovery

Dependencies: `06-02`. Resources: `governance-audit`.

1. Implement begin/update/list-owned records with request/target/revision/submission/outcome timestamps and redaction.
2. Add retention and crash recovery for unresolved submission-pending records; store no output by default.
3. Test database outage before dispatch, failure recording final outcome, foreign audit queries and redaction markers.

Acceptance: audit outage prevents new mutations; restart records unknown outcomes without replaying or declaring success.

## 06-05 — Compose transactional dispatch gate and handlers

Dependencies: `06-03`, `06-04`. Resources: `governance-exports`, `shared-routing`.

1. Compose approval consumption and durable dispatch authorization with one defined consistency boundary.
2. Expose request/status/decision/audit management handlers with pluggable auth and CSRF requirements.
3. Run fixture executor dispatch counters under concurrent requests and deliberate crashes at each boundary.

Acceptance: no mutation passes on consumed/invalid approval or missing audit; handlers cannot accept self-approval from agent mutation scope.

## 06-06 — Integrate identity and validate failure matrix

Dependencies: `06-05`, `02-identity/02-08`. Resources: `governance-integration`.

1. Use real browser/token identity contexts to distinguish human decisions from agent calls.
2. Test scope/revocation/CSRF, cross-installation records, crash recovery and per-user audit isolation.
3. Run governance/database concurrency and standard checks; verify outputs remain sanitized.

Acceptance: real authority boundary holds; fake executor tests are labeled as such and do not claim live SSH guarantees.

## 06-07 — Publish governance handoff

Dependencies: `06-06`. Resources: `governance-release`.

1. Publish service/handler examples, state transitions, transaction ownership and expected MCP/dashboard integration.
2. Record passing failure/concurrency tests and operational recovery/retention instructions in `specs/handoffs/06.md`.
3. Recheck Spec 06 traceability and architecture behavior before marking complete.

Acceptance: 04/05 can integrate without inventing approval policy; all unresolved dispatches remain reconcilable, not automatically retried.

## Scope addition (2026-10-11): per-server approval mode

Approval-needed is decided by the target's `approval_mode` in preflight; approvals bind the target revision so a mode change invalidates unused approvals. Tests: approval created/consumed once under either mode; mode change after approval makes consume fail; `confirm: true` never substitutes for approval. Decision record: [contracts decisions D11](../contracts/decisions.md).
