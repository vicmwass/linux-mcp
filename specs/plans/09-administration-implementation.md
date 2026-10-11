# Spec 09 — Independently releasable administration modules

Source: [09](../09-administration-modules.md). Registry: `09-administration.steps.json`. Follow [shared rules](IMPLEMENTATION-RULES.md). Own resource-tools modules and dashboard page modules named below; no central router edits until a module's registration step. Checks per module: schema fixtures, validation/policy tests, resource/dashboard tests, browser builds, owner-isolation tests and disposable-platform verification.

Each module has contract/design, parallel backend/page and independent release steps. Module release is a completed step usable by deployment independently of the final umbrella summary. No module waits for an unrelated module. The final summary does not mark undelivered modules complete. Storage starts read-only; network/access mutations are a narrowly defined allowlist, never blanket authorization.

## 09-01 — Establish administration extension conventions

Dependencies: `08-console/08-11`. Resources: `administration-design`.

1. Define shared module scaffold, exact DTO versioning, policy/preflight adapters and independently enabled release profiles.
2. Allocate disjoint resource/page/test files for processes, packages, files, storage, network, jobs and access; document supported-platform evidence requirements.
3. Define a release registration checklist and common stale-resource/unknown-outcome fixtures.

Acceptance: each following design step can freeze its contracts independently without touching another module; no new mutation capability is enabled by this scaffold.

## 09-02 — Freeze Processes contracts

Dependencies: `09-01`. Resources: `contracts-registry`, `process-design`.

1. Specify process list/detail, resource units, pagination and signal allowlist.
2. Bind mutation to PID plus start time/identity and current target revision; define protected processes and verification.
3. Publish additive schemas and PID-reuse/refusal fixtures.

Acceptance: module design in `specs/design/09/processes/` contains exact tools/platforms; PID alone cannot authorize termination.

## 09-03 — Implement Processes backend

Dependencies: `09-02`. Resources: `resource-processes`.

1. Implement bounded process queries and guarded signals in `resource-tools/src/processes/`.
2. Recheck process identity before submission, ownership/policy/approval and resulting process state.
3. Run injection, PID reuse, protected-process, output-limit and disconnect tests.

Acceptance: backend contract tests pass; reused PID cannot redirect an approved operation.

## 09-04 — Implement Processes page

Dependencies: `09-02`. Resources: `dashboard-processes`.

1. Implement sortable process list/detail using fixtures in `dashboard/src/pages/processes/`.
2. Present identity-bound signal preview, approval and disappearance/verification states.
3. Test stale PID, unsupported features and unknown outcome UI.

Acceptance: no PID-only mutation or blind retry; fixture UI/feature builds pass.

## 09-05 — Release Processes module

Dependencies: `09-03`, `09-04`. Resources: `shared-routing`, `resource-exports`, `dashboard-shell`, `process-release`.

1. Register tools/pages and enable only compatible targets.
2. Run direct-MCP/dashboard two-user tests and safe disposable-process signal verification.
3. Publish `specs/handoffs/09-processes.md` and enabled deployment profile/test cases.

Acceptance: all per-module 09 criteria pass independently; no unrelated module dependency.

## 09-06 — Freeze Packages contracts

Dependencies: `09-01`. Resources: `contracts-registry`, `package-design`.

1. Specify supported package managers, list/update queries and exact install/remove/update argument contracts.
2. Define preview versus execution evidence, package-manager locks, target revision and privilege/approval rules.
3. Publish schemas/fixtures and platform allowlist in `specs/design/09/packages/`.

Acceptance: no arbitrary shell/package-manager option passthrough; dependency resolution changes between preview/submit are handled honestly.

## 09-07 — Implement Packages backend

Dependencies: `09-06`. Resources: `resource-packages`.

1. Implement distro adapters and bounded structured queries/actions in `resource-tools/src/packages/`.
2. Apply exact package/options validation, locking/failure semantics and post-action version inspection.
3. Test unsupported manager, lock contention, partial install, refusal and unknown outcome.

Acceptance: package batches never claim atomic rollback; outputs and runtime remain bounded.

## 09-08 — Implement Packages page

Dependencies: `09-06`. Resources: `dashboard-packages`.

1. Build package/update tables and explicit package/action previews against fixtures.
2. Show approval, manager lock, partial failures and observed resulting versions.
3. Test unsupported manager, changed preview and disconnected actions.

Acceptance: UI cannot submit arbitrary options or equate command exit success with all packages updated.

## 09-09 — Release Packages module

Dependencies: `09-07`, `09-08`. Resources: `shared-routing`, `resource-exports`, `dashboard-shell`, `package-release`.

1. Register supported tools/pages and module capability flags.
2. Verify disposable distro install/remove and pending updates through real MCP/UI with two users.
3. Publish `specs/handoffs/09-packages.md` with exact tested managers and limits.

Acceptance: supported-platform, isolation, approval and outcome suite passes independently.

## 09-10 — Freeze Files contracts and path boundary

Dependencies: `09-01`. Resources: `contracts-registry`, `file-design`.

1. Define allowed roots, canonical path/symlink policy, size/content limits and byte/text behavior.
2. Specify read/list/metadata and narrowly allowed edit/permission actions with content revision/hash preconditions.
3. Define race-safe remote path handling, backups/recovery guarantees and traversal/symlink fixtures in `specs/design/09/files/`.

Acceptance: path restrictions address time-of-check/time-of-use races; unrestricted file execution/upload and unspecified rollback remain excluded.

## 09-11 — Implement Files backend

Dependencies: `09-10`. Resources: `resource-files`.

1. Implement allowed-root bounded reads/listing and guarded writes in `resource-tools/src/files/`.
2. Enforce race-safe path/content revision checks and operation-specific backups before modifications.
3. Test traversal, symlink swap, stale hash, binary/large file, failure during write and foreign target.

Acceptance: tests prove the specified root boundary; partial write/recovery behavior is explicit and no arbitrary path escapes it.

## 09-12 — Implement Files page

Dependencies: `09-10`. Resources: `dashboard-files`.

1. Build allowed-root browser/metadata and bounded text editor with fixture content.
2. Show exact diff/content revision, approval, backup availability and conflict recovery.
3. Escape content and test malicious HTML/instructions, binary file, oversized file and concurrent edit.

Acceptance: no stored file content is interpreted as page code; stale edits cannot overwrite silently.

## 09-13 — Release Files module

Dependencies: `09-11`, `09-12`. Resources: `shared-routing`, `resource-exports`, `dashboard-shell`, `file-release`.

1. Register allowed file tools/pages with deployment-configured roots.
2. Exercise disposable files through MCP/UI and independently verify boundaries, backups and expected modifications.
3. Publish `specs/handoffs/09-files.md` with supported operations and recovery limits.

Acceptance: traversal/race/isolation/approval tests pass; backups are claimed only where restoration was tested.

## 09-14 — Freeze Storage read-only contracts

Dependencies: `09-01`. Resources: `contracts-registry`, `storage-design`.

1. Define disks/mounts/filesystem usage schemas, units, collection times and permissions.
2. Explicitly exclude partition/format/unmount destructive mutations from this module release.
3. Publish bounded query/unsupported/missing-data fixtures in `specs/design/09/storage/`.

Acceptance: contracts cannot carry undeclared storage mutation authority; later writes require a separate registered design.

## 09-15 — Implement Storage backend

Dependencies: `09-14`. Resources: `resource-storage`.

1. Implement bounded structured disk/mount/usage queries through core in `resource-tools/src/storage/`.
2. Normalize units and represent inaccessible mounts/partial data explicitly.
3. Test unsupported platforms, malformed output, timeouts and owner isolation.

Acceptance: no storage command mutation path; queries do not fabricate zeros for missing filesystems.

## 09-16 — Implement Storage page

Dependencies: `09-14`. Resources: `dashboard-storage`.

1. Build disk/mount/usage views against fixtures with timestamps and units.
2. Show partial/missing data and disable unavailable capabilities.
3. Test large/empty inventories, escaped mount labels and stale samples.

Acceptance: no destructive controls advertised; page handles partial data clearly.

## 09-17 — Release Storage module

Dependencies: `09-15`, `09-16`. Resources: `shared-routing`, `resource-exports`, `dashboard-shell`, `storage-release`.

1. Register read-only tools/pages.
2. Run real disposable-target queries and direct-MCP/dashboard isolation/limit checks.
3. Publish `specs/handoffs/09-storage.md` and explicit read-only capability profile.

Acceptance: safe read-only storage release passes independently of other modules.

## 09-18 — Freeze Network contracts and action allowlist

Dependencies: `09-01`. Resources: `contracts-registry`, `network-design`.

1. Define interface/address/listener/firewall schemas and supported backends.
2. Define a narrow supported rule-change allowlist with exact target/rule, approval and connectivity-risk preview; exclude unsupported interface mutations.
3. Specify verification and recovery limits in `specs/design/09/network/`; publish fixtures.

Acceptance: no generic firewall shell passthrough or promised rollback; an action without a tested safety/verification design is denied.

## 09-19 — Implement Network backend

Dependencies: `09-18`. Resources: `resource-network`.

1. Implement structured network reads and only contracted firewall changes in `resource-tools/src/network/`.
2. Validate addresses/ports/rules, bind approvals and inspect resulting state.
3. Test injection, unsupported backend, control-connection loss and unknown outcome.

Acceptance: no automatic replay after disconnect or undocumented network mutation.

## 09-20 — Implement Network page

Dependencies: `09-18`. Resources: `dashboard-network`.

1. Build interfaces/listeners/firewall views and exact-rule preview using fixtures.
2. Show possible connectivity loss, approval and verification/recovery availability honestly.
3. Test denied/unsupported actions, stale rules and disconnected outcome.

Acceptance: UI cannot grant broader network authority than the backend allowlist.

## 09-21 — Release Network module

Dependencies: `09-19`, `09-20`. Resources: `shared-routing`, `resource-exports`, `dashboard-shell`, `network-release`.

1. Register supported tools/pages and profile flags.
2. Verify reads and narrowly supported rules on disposable network targets with independent access for recovery.
3. Publish `specs/handoffs/09-network.md` with actual limits and recovery evidence.

Acceptance: real isolation/approval/failure tests pass; production firewall changes are not part of validation.

## 09-22 — Freeze Scheduled Jobs contracts

Dependencies: `09-01`. Resources: `contracts-registry`, `jobs-design`.

1. Define cron/systemd timer queries and exact allowed create/update/remove schemas.
2. Specify timezone/schedule interpretation, command allowlist, OS execution identity, privilege and revision checks.
3. Publish schemas/fixtures in `specs/design/09/jobs/`, separating remote jobs from app-owned monitoring schedules.

Acceptance: a scheduled command cannot bypass normal execution policy by deferring it; unsupported arbitrary shell jobs remain denied.

## 09-23 — Implement Scheduled Jobs backend

Dependencies: `09-22`. Resources: `resource-jobs`.

1. Implement contracted timer/cron reads and revision-bound edits in `resource-tools/src/jobs/`.
2. Validate schedules/timezones, command templates, protected entries and OS identity before approval/submission.
3. Test stale entries, invalid schedule, injection, privileges and verification.

Acceptance: no unrestricted scheduled shell execution; owner and machine revision checks precede mutation.

## 09-24 — Implement Scheduled Jobs page

Dependencies: `09-22`. Resources: `dashboard-jobs`.

1. Build cron/timer tables and validated edit previews with explicit timezone/identity.
2. Show command template, required approval, conflicts and observed resulting entry.
3. Test unsupported scheduler, DST/timezone cases and unknown outcomes.

Acceptance: page cannot hide remote execution identity or silently broaden command policy.

## 09-25 — Release Scheduled Jobs module

Dependencies: `09-23`, `09-24`. Resources: `shared-routing`, `resource-exports`, `dashboard-shell`, `jobs-release`.

1. Register scheduler tools/pages and capability gating.
2. Test safe temporary jobs/timers on disposable hosts and remove/verify test entries afterward.
3. Publish `specs/handoffs/09-jobs.md` with supported schedulers/time semantics.

Acceptance: isolation, policy, approval and verification tests pass without leaving running test jobs.

## 09-26 — Freeze Access-management extensions

Dependencies: `09-01`. Resources: `contracts-registry`, `access-design`.

1. Define group/sudo/SSH-key metadata and exact supported changes beyond existing user tools.
2. Specify protected accounts, validated public-key material, sudo-rule allowlist and lock versus revocation semantics.
3. Publish contract fixtures and privilege boundaries in `specs/design/09/access/`.

Acceptance: no arbitrary sudoers text or unvalidated key grant; secret private keys never become inspection output.

## 09-27 — Implement Access backend

Dependencies: `09-26`. Resources: `resource-access`.

1. Implement structured access reads and contracted guarded group/key/sudo edits in `resource-tools/src/access/`.
2. Preserve protected-account checks, syntax validation, safe file update and verification.
3. Test malformed keys, invalid sudo rules, conflict/partial failure and cross-user targets.

Acceptance: cannot weaken protected-user safeguards or grant unrestricted privilege through raw parameters.

## 09-28 — Implement Access page

Dependencies: `09-26`. Resources: `dashboard-access`.

1. Build group/key/sudo views and exact access-change previews against fixtures.
2. Distinguish password lock, key removal and active sessions; show verification and supported reversal only.
3. Test malicious labels, invalid key/rule, denied protected accounts and unknown outcome.

Acceptance: privilege impact is visible and approval cannot be manufactured by UI request arguments.

## 09-29 — Release Access module

Dependencies: `09-27`, `09-28`. Resources: `shared-routing`, `resource-exports`, `dashboard-shell`, `access-release`.

1. Register supported tools/pages without duplicating existing-user mutation routes.
2. Verify temporary account/group/key changes on disposable targets and independently restore/confirm absence.
3. Publish `specs/handoffs/09-access.md` with exact policy scope and validation evidence.

Acceptance: ownership/privilege/approval matrix passes; no test leaves extra privileged access behind.

## 09-30 — Publish complete administration coverage

Dependencies: `09-05`, `09-09`, `09-13`, `09-17`, `09-21`, `09-25`, `09-29`. Resources: `administration-release`.

1. Consolidate supported module versions, platforms, capability flags and module handoffs.
2. Run regression/compatibility checks across enabled combinations and ensure undelivered capabilities cannot appear.
3. Publish `specs/handoffs/09.md`; module gates remain valid for earlier incremental releases.

Acceptance: all seven modules' real release evidence exists; this umbrella step is not required to release a single verified module.
