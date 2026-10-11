# Implementation plan coordination

These instructions apply to implementation driven by this directory's plans. Read `README.md`, the relevant spec/implementation plan, and the coordinator's current status before starting or resuming work. The live coordinator state is authoritative for ownership and completion; Markdown checkboxes are not locks.

For specs 01–12 also read `IMPLEMENTATION-RULES.md`. All steps may be reviewed and redone: read `REVIEW-AND-REWORK.md`. Reopened attempts preserve evidence and invalidate completed downstream acceptance; never trust a previous chat's completed status without refreshing live state. Dependencies in the catalog include other specs: never start based on a merely claimed/in-progress prerequisite. Use `reserve-doc` before a shared architecture-document update when absent from the original claim. Optional WebMCP/assistant steps require selection and do not gate deterministic release.

Claim an eligible step using `scripts/plan_coordinator.py` before editing its implementation files. Only the returned claim-token holder may work on the step. Reserve all declared shared resources. Do not edit files owned by another claimed step. Changes to shared contracts/manifests/routes require the designated step/resource owner.

Heartbeat before each edit batch, after tool calls or waits, and at least every five minutes during active work. Refresh status after compaction, new user instructions, changing tasks and before choosing another step. If ownership verification fails, stop edits immediately.

Complete only after the step's acceptance checks pass and evidence is recorded through the coordinator. Then claim the next eligible step and continue within the user's authorized scope. Record blockers rather than claiming completion; pick another eligible step when possible. When no steps are eligible, report blockers/active dependencies without stealing work or busy polling.

Never automatically reclaim a stale-looking claim. Stop its previous worker and verify it cannot write before explicit recovery. Different worktrees/hosts must not use separate lock stores for the same plan: follow the shared-store restriction in README.

The coordinator is development tooling only. Its use does not change the all-Rust product requirement.
