# Spec 08 — First management console implementation

Source: [08](../08-first-management-console.md). Registry: `08-console.steps.json`. Follow [shared rules](IMPLEMENTATION-RULES.md). Own `crates/resource-tools/src/{health,services,logs}`, corresponding module tests and `crates/dashboard/src/pages/console/`. Shared exports/tool/page registration occurs only in 08-10. Checks: resource/dashboard module tests, contract/schema fixtures, feature builds and disposable supported-distribution verification. Users reuse existing tools rather than redefining user management.

## 08-01 — Freeze health/service/log contracts

Dependencies: `07-deployment/07-08`. Resources: `contracts-registry`, `console-design`, `workspace-manifests`.

1. Read M1 handoffs and define exact structured DTOs, tools, supported platforms and UI mappings for health, services, logs and existing users.
2. Specify timestamps, units, state/unsupported errors, pagination/truncation, service approval bindings and post-change verification.
3. Add additive versioned schemas/fixtures through the contract owner, bootstrap resource-tools crate and create module ownership placeholders.

Acceptance: schemas/tests compile and are frozen before independent module work; no unsupported backend/raw-shell fallback or unbounded log output.

## 08-02 — Implement structured health tool

Dependencies: `08-01`. Resources: `resource-health`.

1. Implement bounded distribution/uptime/reachability/resource queries through authorized execution.
2. Return observed data with timestamp/revision and explicit missing/unsupported fields.
3. Test malformed command outputs, unavailable target, foreign IDs and platform fixtures.

Acceptance: health contract tests pass; a missing sample cannot become fabricated zero/healthy data. Publish fixtures for the page owner.

## 08-03 — Implement Overview page

Dependencies: `08-01`. Resources: `dashboard-console-overview`.

1. Build overview cards/table against frozen health fixtures and shared target/status components.
2. Show collection time, stale/missing fields, connection state and relevant owned activity.
3. Test loading, failure, empty/unsupported states and escaped remote values.

Acceptance: page is usable with fixture services and cannot display another owner's cached result; real tool integration waits for 10.

## 08-04 — Implement service tools and verification

Dependencies: `08-01`. Resources: `resource-services`.

1. Implement supported init-system list/detail/start/stop/restart with validated service identifiers.
2. Route mutations through exact intent, approval, audit and current-revision checks; query resulting state.
3. Test injection, unknown services, denied action, changed revision and mismatch/unavailable verification.

Acceptance: structured service state and bounded outputs pass; exit code success is not presented as verified service recovery.

## 08-05 — Implement Services page

Dependencies: `08-01`. Resources: `dashboard-console-services`.

1. Build searchable/filterable service listing/detail against fixtures.
2. Add exact-action preview, approval link/status, progress and distinct verification result.
3. Test unsupported init systems, refused/unknown outcomes and active target changes.

Acceptance: no action bypasses shared approval flow; fixture UI has explicit failure/verification states.

## 08-06 — Implement bounded log tools

Dependencies: `08-01`. Resources: `resource-logs`.

1. Implement supported log backend filters for service/severity/time with byte/entry/time budgets.
2. Bind cursors to owner/target/revision/query and return truncation/continuation semantics.
3. Test log floods, malformed lines, unsafe filters, foreign cursors and unavailable backends.

Acceptance: queries are bounded/owner-scoped; sensitive raw log output is not persisted by default.

## 08-07 — Implement Logs page

Dependencies: `08-01`. Resources: `dashboard-console-logs`.

1. Build filters and incremental retrieval against frozen fixtures, using bounded retained browser state.
2. Escape log content and show query time, truncation, stale revision and backend support.
3. Test malicious HTML/instructions, stale/foreign cursor, large output and reconnect without duplicate unbounded accumulation.

Acceptance: logs remain untrusted text; unsupported tools disable retrieval and core navigation stays usable.

## 08-08 — Integrate existing Users page semantics

Dependencies: `08-01`. Resources: `dashboard-console-users`.

1. Reuse existing user adapters and add console navigation/search/result presentation.
2. Preserve protected-account/delete confirmation and password-lock semantics.
3. Test supported operations, denied/unknown outcomes and capability-unavailable actions.

Acceptance: no new unrestricted account mutation path; UI explains password lock does not revoke keys or terminate sessions.

## 08-09 — Validate standalone modules and schema compatibility

Dependencies: `08-02`, `08-03`, `08-04`, `08-05`, `08-06`, `08-07`, `08-08`. Resources: `console-module-tests`.

1. Run each tool/page fixture suite and public/browser feature build.
2. Check additive schema versions and all owner/revision/cursor boundaries.
3. Produce integration adapters/examples without editing central routers yet.

Acceptance: all modules independently pass contracts; unresolved DTO drift blocks central registration.

## 08-10 — Register tools/pages and run real console workflow

Dependencies: `08-09`. Resources: `shared-routing`, `resource-exports`, `dashboard-shell`, `console-live-tests`.

1. Register module exports, MCP tools, page routes and capability gating under exclusive shared ownership.
2. Run Alice/Bob direct-MCP/dashboard inspection on disposable targets; investigate a controlled failed service through logs.
3. Approve a supported service change, independently verify state and test key failure/unknown outcome handling.

Acceptance: M2 real-console workflow and unsupported-distribution cases pass; no module makes a silent raw-command fallback.

## 08-11 — Publish M2 evidence and handoff

Dependencies: `08-10`. Resources: `console-release`.

1. Run applicable root/resource/dashboard regression suites and isolation tests.
2. Extend release test/profile documentation with supported tools/platforms and actual limits.
3. Publish `specs/handoffs/08.md` and M2 acceptance traceability.

Acceptance: Overview/Services/Logs/Users work through authenticated MCP with approval and verification; broader modules remain separately gated.
