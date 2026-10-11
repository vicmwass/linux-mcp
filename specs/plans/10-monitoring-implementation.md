# Spec 10 — Monitoring and alerts implementation

Source: [10](../10-monitoring-and-alerts.md). Registry: `10-monitoring.steps.json`. Follow [shared rules](IMPLEMENTATION-RULES.md). Own `crates/monitoring/`, metrics resource-tool module, monitoring dashboard modules and namespaced migrations. Checks: deterministic clock/time-series fixtures, PostgreSQL lease/concurrency tests, module/browser builds and real read-only MCP collection. Independent of Spec 09 mutations.

## 10-01 — Freeze metrics, schedules and alert contracts

Dependencies: `07-deployment/07-08`. Resources: `contracts-registry`, `monitoring-design`.

1. Define CPU/memory/disk/network units, counters/resets, missing samples, timestamps and target revisions.
2. Define user-delegated read-only collection grants, schedule budgets, leases/deduplication and history retention.
3. Freeze schemas/fixtures for history queries, alert rules/evaluations/inbox and collection errors in `specs/design/10/`.

Acceptance: no global all-user collection authority; missing samples are gaps; leases have explicit crash/retry semantics and numeric-only history by default.

## 10-02 — Bootstrap monitoring crate and persistence

Dependencies: `10-01`. Resources: `workspace-manifests`, `migration-registry`, `monitoring-bootstrap`.

1. Add crate and migrations for schedules, samples/aggregates, lease/dedupe records and alert inbox.
2. Configure bounded retention, quotas, indexes and test clock/PostgreSQL harness.
3. Create separate module/page placeholders and feature flags.

Acceptance: fresh/restart migrations and crate checks pass; all tables include owner/installation and history target revision.

## 10-03 — Implement structured metrics tools

Dependencies: `10-01`. Resources: `resource-metrics`.

1. Implement bounded platform-aware numeric queries in `resource-tools/src/metrics/` through authorized execution.
2. Normalize units, observation times and counter resets; return unsupported/missing rather than invented values.
3. Test partial output, wrap/reset, unavailable targets, malformed values and foreign IDs.

Acceptance: metric schemas and resource tests pass; no raw command-output persistence or mutation capability.

## 10-04 — Implement scheduler, grants and replica leases

Dependencies: `10-02`. Resources: `monitoring-collector`.

1. Implement user-delegated MCP read client, grant refresh/revocation checks and target-currentness checks.
2. Implement schedule leasing, deduplication, bounded concurrency/backoff and quota enforcement.
3. Test two replicas, lease expiry/crash, revoked owner/grant, deletion and restart without catch-up storm.

Acceptance: duplicate schedule dispatch is bounded/prevented by defined lease semantics; no new collection after the revocation bound; user workloads obey quotas.

## 10-05 — Implement history and retention API

Dependencies: `10-02`. Resources: `monitoring-history`.

1. Implement owner-scoped numeric sample ingestion and bounded time-range history/aggregation queries.
2. Implement retention and historical revision labels, preserving gaps/reset semantics.
3. Test foreign target/history IDs, concurrent ingestion, duplicate samples and deletion/retention boundaries.

Acceptance: PostgreSQL/history contracts pass; old samples cannot masquerade as current target measurements.

## 10-06 — Implement alert evaluator and inbox

Dependencies: `10-02`. Resources: `monitoring-alerts`.

1. Implement threshold windows, sustained breach/recovery/unavailable states and deduplicated dashboard inbox entries.
2. Keep rules/delivery settings owner-scoped; external message channels remain disabled/unimplemented initially.
3. Test flapping, gaps, counter reset, duplicate evaluations and recovery with controlled clock.

Acceptance: deterministic fixtures produce expected meaningful alerts without repeated unchanged notifications; no unsolicited external messages are sent.

## 10-07 — Implement monitoring and alert pages

Dependencies: `10-02`. Resources: `dashboard-monitoring`.

1. Build metrics/history charts, schedule settings, alert rules and inbox against frozen fixtures.
2. Display units/time windows, gaps, collection/grant state, revision and retention limits.
3. Test absent samples, revoked grants, large history ranges, cross-user cache and accessible chart alternatives.

Acceptance: browser/public feature builds and fixture workflows pass; missing values are not charted as zero or healthy.

## 10-08 — Compose and validate real collection

Dependencies: `10-03`, `10-04`, `10-05`, `10-06`, `10-07`, `08-console/08-11`. Resources: `shared-routing`, `resource-exports`, `dashboard-shell`, `monitoring-integration`.

1. Register metric tools, schedule/history/alert routes and pages; compose real identity/MCP/storage adapters.
2. Run two collectors/two owners against disposable targets and verify quotas, leases, revocation and target updates.
3. Induce outages/restart and verify history gaps/backoff, no catch-up storm and alert recovery.

Acceptance: actual read-only delegated collection and browser/history isolation pass; mock clocks do not substitute for replica/storage integration.

## 10-09 — Publish monitoring release handoff

Dependencies: `10-08`. Resources: `monitoring-release`, `deploy-manifests`.

1. Run full monitoring/resource/dashboard suites and retention/restore checks.
2. Document budgets, supported metrics, authorization lifecycle and operational lease/recovery behavior.
3. Publish `specs/handoffs/10.md` and independently enabled monitoring release profile.

Acceptance: Spec 10 criteria pass; notification inbox requires no external provider or Spec 09 mutation feature.
