# Agent implementation plans and coordination

All 13 specs have detailed implementation plans and machine-readable step registries. [The catalog](catalog.json) is the cross-spec dependency graph; plans below describe the same steps in human-readable form. State tracks actual progress and review/rework attempts separately from the scheduling definitions.

| Spec | Implementation plan | Steps | Completion gate |
|---|---|---|---|
| 00 | [Contracts](00-contracts-implementation.md) | 12 | 00-12 |
| 01 | [Inventory/secrets/trust](01-inventory-implementation.md) | 8 | 01-08 |
| 02 | [Identity](02-identity-implementation.md) | 8 | 02-08 |
| 03 | [Execution](03-execution-implementation.md) | 8 | 03-08 |
| 04 | [MCP service](04-mcp-implementation.md) | 8 | 04-08 |
| 05 | [Dashboard](05-dashboard-implementation.md) | 9 | 05-09 |
| 06 | [Approvals/audit](06-governance-implementation.md) | 7 | 06-07 |
| 07 | [Deployment/integration](07-deployment-implementation.md) | 8 | 07-08: M1 |
| 08 | [First console](08-console-implementation.md) | 11 | 08-11: M2 |
| 09 | [Administration modules](09-administration-implementation.md) | 30 | Independent module gates; 09-30 summary |
| 10 | [Monitoring/alerts](10-monitoring-implementation.md) | 9 | 10-09 |
| 11 | [Workflows/optional agents](11-workflows-implementation.md) | 13 | 11-09 core; 11-11/13 opt-in |
| 12 | [Independent client page design](12-client-design-implementation.md) | 10 | 12-10: initial versioned design handoff |

There are 141 steps, including four optional extension steps. Read [shared implementation rules](IMPLEMENTATION-RULES.md) for file ownership, validation, evidence and architecture-document reservations. [Every step can be reviewed and redone](REVIEW-AND-REWORK.md), with previous evidence retained and dependent acceptance revalidated. Other agents may already be implementing Spec 00; writing these plans does not reset or complete their work.

## Dependency enforcement

Every registry step has `depends_on`. Local IDs reference its own plan; qualified IDs such as `00-contracts/00-12` reference another catalog plan. A claim is refused unless EVERY listed prerequisite has status `completed`. Claimed, blocked, released and pending prerequisites never qualify. Missing references and local/cross-spec cycles reject the catalog before scheduling. The same check runs again before completion.

All catalog registries load together under the existing transaction lock; their current state and resource reservations are shared. Adding new registries preserves existing claims and tokens. Existing frozen registries cannot be silently changed. Readiness against mocks is separated from final real-adapter gates in the dependencies, so a mock demo never unlocks a real release gate by itself.

The catalog prevents new 01–06 work before Spec 00's contract gate. Spec 12 starts immediately without backend prerequisites; its initial published design gate is required for 05-02's shared page shell, and workflow pages wait for that shell. Deployment design/harness can proceed after contracts; production images wait for real service handoffs. M2 starts after M1. Administration modules and monitoring can proceed independently after their respective gates. Workflows can build with mocks after contracts but cannot integrate before console/monitoring handoffs.

## Mandatory agent loop

1. Read this file, `AGENTS.md`, the relevant spec and its implementation plan. Read repository instructions and inspect unrelated local changes.
2. Run `status`. On resuming, verify your existing claim with `heartbeat` before editing. Never infer ownership from a past chat message.
3. Run `claim --owner <unique-agent-session-id>` to atomically pick the first dependency-ready step whose resources are free. An optional `--step` selects an eligible step explicitly. Save the returned token in your working notes, never in product code.
4. Read the claimed step in full, inspect its inputs and implement only that scope. Heartbeat at least every five minutes and at edit/tool boundaries. Update progress using `heartbeat --note`.
5. Run the step's checks. If blocked, run `block` with a reason and handoff evidence; its reservations are released. If interrupted voluntarily, `release` returns the step to pending with handoff notes.
6. Run `complete` with meaningful evidence: changed files/revision, actual validation commands/results and limitations. The coordinator verifies ownership and dependencies before marking it completed and releasing resources. Do not mark a failed or partial step completed.
7. Run `status`, then `claim` again. Continue until the authorized plan is completed or no eligible work remains. A blocked step requires explicit `unblock --reason` after its cause is resolved.

Do not stop solely because one step finished. Do not implement future specs merely because Spec 00 finished unless the user authorized those implementations. Creating these plans has not started product implementation.

## Commands from the repository root

```powershell
python scripts/plan_coordinator.py status
python scripts/plan_coordinator.py claim --owner agent-session-123
python scripts/plan_coordinator.py heartbeat --step 00-01 --token CLAIM_TOKEN --note "Baseline inspected"
python scripts/plan_coordinator.py complete --step 00-01 --token CLAIM_TOKEN --evidence "Files: ...; Checks: ...; Result: ..."
python scripts/plan_coordinator.py block --step 00-01 --token CLAIM_TOKEN --reason "Dependency unavailable" --evidence "Attempted ...; next action ..."
python scripts/plan_coordinator.py release --step 00-01 --token CLAIM_TOKEN --reason "Session ending; handoff ..."
python scripts/plan_coordinator.py unblock --step 00-01 --reason "Dependency restored; evidence ..."
```

Pass `--plan specs/plans/<registry>.json` before the command to work within one spec. The default is Spec 00. `status` reports pending, claimed, blocked and completed states, dependencies, current owners, resource reservations and stale-heartbeat warnings. `claim` with no available work makes no changes and explains the state.

```powershell
# Read the complete graph and live progress without taking a step.
python scripts/plan_coordinator.py --all status

# Work within the explicitly assigned inventory spec.
python scripts/plan_coordinator.py --plan specs/plans/01-inventory.steps.json claim --owner inventory-agent-session

# Only when authorized across all specs: take the next eligible catalog step.
python scripts/plan_coordinator.py --all claim --owner agent-session

# Opt-in package selection also requires its normal dependencies to be complete.
python scripts/plan_coordinator.py --plan specs/plans/11-workflows.steps.json --enable webmcp claim --owner browser-extension-agent

# Validate documents/registries/graph without modifying live claim state.
python scripts/check_implementation_plans.py

# Design can start now, independently of Spec 00/backend work.
python scripts/plan_coordinator.py --plan specs/plans/12-client-design.steps.json claim --owner design-agent-session

# Reviews and rework apply to ALL completed specs/steps, not just designs.
python scripts/plan_coordinator.py --plan specs/plans/05-dashboard.steps.json review --step 05-04 --reviewer reviewer-session --reason "Finding and requested change"
python scripts/plan_coordinator.py --plan specs/plans/05-dashboard.steps.json reopen --step 05-04 --reason "Redo after review; scope and evidence"
```

When using `--all`, commands accept a qualified step such as `01-inventory/01-03` or its globally unique ID. `--enable assistant` selects the other optional package; repeat `--enable` to select both. Neither option bypasses dependencies or grants permission to externally deploy/use a provider. Optional steps remain unavailable by default and core workflow completion does not depend on them.

## Locking and recovery guarantees

The tool takes an OS file lock around dependency/resource checking and an atomic state-file replacement. Simultaneous cooperating processes on the same local filesystem cannot both claim a step or the same named resource. Claims persist after process exit; the short transaction lock is released automatically by the OS. There is no automatic claim expiry, since an old worker may still be editing.

For crash recovery, first stop the old worker and verify it cannot continue writing. Then run `recover --step ID --token OLD_TOKEN --reason "Worker stopped; verification and handoff ..."`. The exact current token is required, recovery is recorded, and the step returns to pending. A returning old worker fails heartbeat/completion checks. This is cooperative coordination, not an operating-system prohibition on file edits.

All agents must use the SAME absolute `--state-dir` on the SAME host/local filesystem. Default: `specs/plans/.coord`. State covers all registered plans in that directory, so shared resource names lock across specs. Independent clones/worktrees do not automatically share this directory. Do not rely on Git pushes, cloud-synced folders or network filesystem semantics for distributed locking; use a transactional central coordinator before distributing agents across hosts. Separate worktree outputs also need integration before a dependency can count as completed; this initial workflow assumes one shared checkout.

The state directory is ignored by Git because it contains live claims. Preserve it across agent restarts. Do not delete/reset it to free work. Save `status` output alongside milestone handoffs and back up the state directory when moving the checkout. Lost/corrupt state requires stopping all workers and reconciling files/evidence before restoring scheduling. The tool refuses corrupt state and registry drift rather than silently resetting progress.

## Ownership, changes and completion

Registry `resources` are logical exclusive editing areas; they must reflect actual file overlap. New plans must use the same names for shared paths, particularly `workspace-manifests`, `contracts-registry`, `architecture-doc` and `shared-routing`. Update architecture documentation through its reserved resource. Modules can proceed independently without central registration edits; integration steps own those registrations.

Registry topology is stable once scheduled so dependencies cannot be bypassed, but outputs and completed steps remain reviewable/reworkable. Use `review` and `reopen` for new attempts without changing the registry. If topology must change, stop affected workers, reconcile state, review a versioned registry and explicitly migrate progress under the shared transaction lock. Unrelated claims must be preserved. `completed` means acceptance checks passed for the recorded attempt in this shared checkout, not that a remote branch has unmerged changes or that outputs are permanently approved. Never claim external approval or real integration merely because mocks passed.

## Step template for subsequent specs

Every step needs: stable ID, objective, prerequisite step IDs, reserved resources/allowed files, implementation sequence, acceptance commands/results, output artifacts and handoff instructions. Give each independently editable module its own resource and integration step. Every plan ends with integration validation, documentation and a recorded completion gate.
