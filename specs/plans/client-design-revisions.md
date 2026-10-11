# Reviewing and redoing client designs

Spec 12 completion is a versioned handoff, not a permanent design lock. The spec text, mockups, components and page decisions can be reviewed and changed. An active worker's edit reservations still apply. Record feedback immediately in an unclaimed review-request artifact or the user conversation; do not overwrite an active worker's files.

This is an optional parallel-revision workflow, not the only way to redo designs. Like any step, design work supports the general [review/reopen commands](REVIEW-AND-REWORK.md). Reopen when a correction must invalidate the existing completion gate and downstream acceptance; use a supplemental draft revision when exploring a new version without silently invalidating the published baseline. Both retain history and use fresh claims.

## Starting a revision after a published baseline

1. Read current coordinator state and `design/client/baselines.json`; identify the version being reviewed and whether the request affects one screen, shared tokens/components or a full redesign.
2. Create a new plan/registry, for example `12-client-design-r02-implementation.md` and `12-client-design-r02.steps.json`, using the six-step template below and a unique `plan_id` of `12-client-design-r02`. Record baseline, reason, scope and affected consumers.
3. Register the new plan in the catalog without changing original step completion records. Add its exact `document` filename to the registry. Use qualified step IDs when duplicate `12-01` style IDs occur across revisions, e.g. `12-client-design-r02/12-02`.
4. Claim revision steps normally. Keep draft outputs in `design/client/revisions/r02/`; reference unchanged baseline assets explicitly. Shared-system edits/publication require the same existing resource names, so revision workers cannot collide with current design workers.
5. Complete review/QA, publish versioned evidence and an impact/adoption checklist. Keep the prior baseline available. Update the implementation-baseline pointer only as an explicit recorded publication/adoption action; a draft alone cannot change it.
6. Existing page agents keep their pinned baseline until assigned migration work. Add tracked adaptation steps under component/page ownership; do not reset completed implementation steps or claim the revised design is already implemented.

During initial design, feedback can be resolved within the claimed step and rechecked before completion. If a completed portion needs rework before the first publication, record a supplemental revision plan with dependencies on that portion and integrate its changes in the pending initial QA/handoff. If the user makes review a prerequisite for ongoing implementation, use explicit blockers and coordinate affected workers; silence is not approval.

## Six-step revision template

Use the real baseline's completion gate as the first dependency: initial baseline is `12-client-design/12-10`; a published revision uses that revision's `12-06`. IDs below are local to the new revision plan. Exact scope and checks must be filled in before registration; a revision may add extra page-specific steps for parallel work.

| Step | Dependencies | Resource reservation | Actions and completion evidence |
|---|---|---|---|
| 12-01: Review and scope | Previous baseline gate | `client-design-review` | Inspect feedback/screens; record open/addressed/deferred findings; define scope, impact and review criteria |
| 12-02: Revise foundations | 12-01 | `client-design-system` | Update scoped draft tokens/components/navigation; preserve prior assets; check contrast/consistency and document compatibility |
| 12-03: Revise screen mockups | 12-02 | Affected `client-design-*-pages` resources | Update scoped draft screens/states; annotate changes and target/approval semantics; check responsive flows |
| 12-04: Review and validate | 12-03 | `client-design-preview`, `client-design-review` | Assemble preview; inspect desktop/tablet/mobile and keyboard/critical safety flows; resolve critical findings and save evidence |
| 12-05: Prepare adoption handoff | 12-04 | `client-design-handoff` | Produce version/diff, changed component/page list, data-contract impact and tracked implementation migration requirements |
| 12-06: Publish revision | 12-05 | `client-design-release` | Publish artifacts/checksums and review outcomes; retain prior version; explicitly record implementation-baseline/adoption decision |

For a narrowly scoped change, 12-02 can complete with evidence that foundations were reviewed and remain unchanged; it cannot falsely claim edits. Review outcomes record who reviewed and whether approval was actually requested/given. An agent may mark its validation complete, but cannot invent a human approval.

Registries stay immutable once in use to protect scheduling integrity; this does not make the underlying design immutable. New revision registries are the mechanism for repeatable rework and fresh locks/evidence.
