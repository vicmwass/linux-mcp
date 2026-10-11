# Rules shared by plans 01–12

Read the repository instructions, relevant spec, plan, current coordinator status and prerequisite handoffs before each claim or resumption. The numbered sections in each plan correspond exactly to its `.steps.json` registry. Local dependency IDs refer to the same plan; `plan-id/step-id` refers to another catalog plan. Eligibility is enforced, not a suggestion.

Every step follows this sequence: claim, inspect inputs, implement the numbered actions, run acceptance checks, update documentation, record evidence, complete, claim next. Use mocks only where explicitly allowed. A mock-backed acceptance does not satisfy a later real-integration gate. Tests mentioned in a step must be implemented if not already present. Planning is not authorization to operate on production machines or publish/deploy publicly.

All steps and outputs may be reviewed and redone through [review/rework](REVIEW-AND-REWORK.md). A reopened attempt starts from current artifacts and review scope; never blindly repeat destructive changes or remote mutations. Previous completion evidence remains historical, while dependent steps require fresh validation. Spec 12's design-only work uses prototype/visual checks rather than backend tests and has no backend prerequisite.

## Scope, files and checks

The plan header specifies owned paths and standard checks. Step resources reserve only the indicated module or editing area. No agent may edit another crate's implementation just because it consumes that crate; propose the fix to its owner and block dependent work if necessary. Shared Cargo manifests/lockfile, central contract exports, routers, migration runner and deploy manifests require their named resources. Migration SQL belongs to per-crate namespaced directories with unique prefixes; only the migration-registry owner registers ordering. Later feature plans integrate their own additive registrations under the same locks, without modifying completed/frozen steps.

Run targeted unit/contract tests while developing. Before completion run the plan's applicable format, compile, feature/boundary and regression checks. Replace illustrative package names with those established in the step's crate bootstrap. Record exact commands/exit results, changed paths, revision/checksum and limitations in `specs/implementation-evidence/<step-id>.md` and the completion message. Commands must really pass; missing required tooling blocks the step rather than counting as a skipped success. No tests may read real credentials. Live tests use disposable configured targets only.

## Architecture documentation without serializing all coding

Applicable source, deployment and security edits use the architecture-sync skill. After module checks, reserve the architecture document before editing it:

```powershell
python scripts/plan_coordinator.py --plan specs/plans/01-inventory.steps.json reserve-doc --step 01-03 --token CLAIM_TOKEN
```

`reserve-doc` adds the exclusive `architecture-doc` resource to the current claim atomically. If another worker holds it, keep heartbeating, do useful checks and retry after it becomes free; never edit it without the reservation. If ending the session, release/block with handoff. Do not use this operation to acquire someone else's implementation scope. Completion releases all reservations. Documentation records implemented behavior and actual validation only. This permits parallel module coding with a brief serialized documentation update.

## Handoffs and integration

Every plan's last required step publishes `specs/handoffs/<spec-id>.md`: artifacts, stable schemas/features, setup, mocks versus real adapters, passing checks, limitations and migration/compatibility instructions. Release prerequisites refer to final verified steps, not a crate that merely compiles. An agent can work within its assigned spec by using `--plan`; use `--all claim` only when authorized to implement across the catalog. If no eligible step exists in its authorized scope, report waiting dependencies and active claims instead of stealing work.

Each module in 09 has an independent release gate. Spec 11 core completion excludes optional WebMCP/AI steps; those steps require an explicit enabled package in the release profile and remain pending unless selected. Pending optional steps do not block the deterministic release. Do not mark an unimplemented optional feature completed.
