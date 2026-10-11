# Repository agent instructions

For implementation of the proposed specs, first read `specs/plans/README.md`, `specs/plans/AGENTS.md`, the relevant spec and its detailed plan. Follow the coordinator's live state throughout the task. Claim an eligible step and its resources before editing implementation files; verify ownership at edit boundaries, record acceptance evidence on completion, then claim the next eligible step within the authorized scope. Do not implement a spec without its registered detailed plan.

Planning/documentation maintenance and unrelated existing-server fixes do not require a product implementation claim, but must not overwrite another agent's claimed work. Read current status first if implementation agents are active. Use the architecture-sync skill for applicable repository changes.

Claims coordinate cooperating agents on one shared checkout/local filesystem. Never treat independent worktree/host claim stores as shared locks. See the plan workflow for blockers, handoff, recovery and central-coordinator requirements.

All specs and steps can be reviewed and redone. Follow `specs/plans/REVIEW-AND-REWORK.md`: preserve previous completion evidence, use `review`/`reopen` for tracked attempts, revalidate dependent acceptance, and coordinate affected active workers before reopening. Completion never permanently locks design or implementation outputs.
