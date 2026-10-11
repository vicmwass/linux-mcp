# Review and rework for every spec and step

All specs, steps, design decisions and implementation outputs may be reviewed and redone. Completion records the checks for one attempt; it is not permanent approval or a prohibition on edits. The step topology stays stable while attempts, review notes and completion evidence accumulate.

## Record a review at any time

```powershell
python scripts/plan_coordinator.py --plan specs/plans/03-execution.steps.json review --step 03-04 --reviewer reviewer-session --reason "Finding, evidence, affected behavior and requested change"
```

This appends review history without taking over an active worker's claim or changing its files. A reviewer label is attribution in development tooling, not authenticated proof of human approval. Review notes alone do not block scheduling: a blocking finding must also be handled through the rework/blocking procedure below.

## Redo a completed step

```powershell
python scripts/plan_coordinator.py --plan specs/plans/03-execution.steps.json reopen --step 03-04 --reason "Review requires correction; describe scope and evidence"
```

The coordinator atomically:

1. Finds every transitive dependent in the catalog, including other specs.
2. Refuses to reopen if the step or an affected dependent is actively claimed. Coordinate a handoff: those workers retain their tokens and can release/block after recording work. Never steal their claims.
3. Archives prior completion evidence and attempt number; appends the reason/history.
4. Returns the requested step and completed dependents to pending with incremented attempt numbers. Unfinished pending/blocked work retains its existing state. Unrelated completed work remains completed.
5. Enforces dependencies again. A downstream claim stays blocked until the revised prerequisite is completed. New claims get fresh tokens and the same resource reservations.

After reopening, agents inspect current code and the review scope before editing. Revise or revalidate existing artifacts; do not blindly rerun destructive migrations, duplicate extraction or perform previously executed remote mutations. Record new tests/evidence for this attempt, then complete and pick the next eligible step. Dependent steps may finish through revalidation if their outputs need no change, but must explain why and rerun relevant checks.

This is deliberately conservative: reopening invalidates completed dependent acceptance even when an eventual review proves some outputs unaffected. It avoids presenting an old integration result as proof of corrected behavior. Runtime machines/deployed services are not rolled back by changing tracker status.

## Unfinished work and broader revisions

For a pending/blocked step, update the review notes and resolve blockers before claiming it. For a claimed step, its owner handles feedback within the current claim, or releases/blocks with a handoff. New behavior that changes scope, prerequisites or resource ownership requires a versioned plan revision and explicit state migration; do not silently alter an active registry.

A nonblocking alternative design or broad redesign can use a new supplemental revision plan instead of invalidating a published baseline immediately. Record its original version, scope, dependencies, shared resources, validation and adoption gate. Consumers pin the baseline they use until adoption is assigned; drafts do not silently alter production requirements. [Client design revisions](client-design-revisions.md) provide a concrete example. The original and supplemental work remain reviewable/reopenable by the same commands.
