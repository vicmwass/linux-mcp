# Spec 11 run state machine, resumability and authority

Schema: [schemas/workflow-run.schema.json](schemas/workflow-run.schema.json). Fixtures: [fixtures/runs](fixtures/runs), [fixtures/invalid-runs](fixtures/invalid-runs). Mutation lifecycle states are those of [design/06](../06/README.md); this document layers run-level progress above them and adds nothing that can bypass them.

## Run states

`created → running ⇄ awaiting_approval → completed | completed_partial | stopped | failed | cancelled`, plus `unknown_outcome → reconciling → running|stopped` and `blocked`.

| State | Meaning | `state_reason` |
|---|---|---|
| `created` | run record exists, parameters validated, targets pinned (server ID + revision at start) | – |
| `running` | the runner is executing a step | – |
| `awaiting_approval` | a `mutate` step has a stored intent and a pending approval; nothing is sent | – |
| `unknown_outcome` | a `mutate` step was submitted and its result is unknown | `outcome_unknown` |
| `reconciling` | a human-initiated inspection of the unknown step is in progress | – |
| `blocked` | authority or target changed; the run cannot continue | `authorization_lost`, `revision_changed`, `target_disabled` (R5) |
| `stopped` | a stop condition, budget or approval outcome ended the run cleanly | `stop_condition`, `budget_exhausted`, `approval_expired`, `approval_rejected` |
| `completed` | all steps finished; mutation steps verified or verification explicitly `unavailable` | – |
| `completed_partial` | read-only fleet run finished with failed/unavailable targets | – |
| `failed` | a step failed before any submission and policy says stop | `step_failed` |
| `cancelled` | user cancelled before any submission, or after: see below | `user_cancelled` |

Terminal states: `completed`, `completed_partial`, `stopped`, `failed`, `cancelled`, `blocked`, and `unknown_outcome` until reconciled. Fixtures cover `awaiting-approval`, `unknown-outcome`, `blocked-revision-changed`, `completed-verified` and `verification-unavailable`.

## Step states

`pending → running → succeeded | failed | skipped`; read-only steps stop there. For `mutate`: `pending → awaiting_approval → approved → dispatching → submitted → (succeeded | unknown) → reconciled`, or `refused` / `blocked` before submission. For `verify`: `verified | verification_mismatch | verification_unavailable`.

Mapping to Spec 06: `awaiting_approval` = `ApprovalRequired`; `approved` = `Approved`; `dispatching` = `DispatchPending`; `submitted` = `Submitted`; `unknown` = `UnknownAfterSubmission`; `reconciled` = `Reconciled`. The step's `operation_id`, `approval_id`, `submission` and `outcome_status` copy the governance record; the governance record is authoritative.

## Step execution rules

1. **Before every step**: re-check run authority (below), the target's current revision against `server_revision_at_start`, enabled state, budgets and run deadline. Any failure moves the run to `blocked` or `stopped` and records the reason; no tool call is made.
2. **`inspect`/`verify`**: one MCP call as the run owner's delegated principal; the result is stored as a bounded structured observation (`tool`, `collected_at`, `truncated`, `bytes`); raw output is not persisted by default. A read-only step that was `running` when the process stopped is idempotent and is simply re-run on resume (counted against budgets).
3. **`mutate`**: the runner builds the intent from literals and validated parameters/observations, calls `prepare_operation`, and moves to `awaiting_approval` if the target's `approval_mode` requires it (Spec 06 decides). It then waits. After the owner approves in the dashboard it performs the atomic consume + durable pending record + submit exactly as the manual path does. The runner never approves, never retries a consumed approval and never creates a second intent for the same step and target (R3).
4. **After submission** any ambiguity yields `unknown`; the run enters `unknown_outcome` and executes nothing further.
5. **`verify`** runs after a submitted mutation (even when its result was unknown, if the user chose to reconcile). Result `verified` / `mismatch` / `unavailable` is recorded; exit status alone is never verification.
6. **`checkpoint`** false → `stopped(stop_condition)`; the observation that drove it stays attached for the user.
7. Budgets: `steps`, `tool_calls`, `elapsed_s`, `observation_bytes` are counted in `budgets_used`; exceeding one stops the run before the next call (`budget_exhausted`).

## Resumability and restart recovery

The run record is durable (Spec 11 migrations). On process start the runner resumes only runs it can prove safe:

| Persisted step state at crash | Action |
|---|---|
| `pending`, `succeeded`, `failed`, `skipped`, `verified*` | continue from the next step |
| `running` (read-only) | re-run the step (idempotent) |
| `awaiting_approval`, `approved` | continue waiting / re-check approval validity; an expired or invalidated approval ends the step as `refused` and the run as `stopped` |
| `dispatching` | governance recovery decides: `FailedBeforeSubmission` only if provably not sent, otherwise `unknown`; the runner follows that result |
| `submitted`, `unknown` | run becomes/stays `unknown_outcome`; **never re-dispatch** |

Continuation from `unknown_outcome` requires explicit human action: the user starts a reconciliation inspection (read-only `verify`-style observation of the actual state), then chooses **mark reconciled and continue**, or **stop**. Even when reconciliation shows the change did *not* happen, the run does not re-send: a new attempt is a **new run** with a new intent and a new approval. This follows Spec 06 ("a consumed approval is never reset").

## Authority

A run executes with the owner's **delegated** authority (Spec 02 Flow C), obtained per call. There is no stored service credential and no run-owned long-lived token. Consequences:

- If the delegated grant is revoked, expires without refresh, or the user is disabled, the next step fails authorization and the run becomes `blocked(authorization_lost)`. Revocation latency follows the identity bound (≤ 60 s for new operations); an already-submitted mutation may still complete and is reconciled as above.
- Browser logout ends delegated tokens held for that session; a run waiting for approval simply stays `awaiting_approval` (approval itself needs an interactive session anyway) and cannot make calls until the user signs in again.
- Fleet runs resolve each target independently; losing access to one target is a per-target `failed`/`unavailable` row, while loss of the user's authority blocks the whole run.
- `approvals:decide` is never part of delegated run authority; the runner can request approvals but not decide them.

## Cancellation guarantees

- Before any submission: cancel stops the run; nothing ran.
- After a submission: cancel stops the *workflow*, not the remote command. The step stays `submitted`/`unknown`; the run moves to `unknown_outcome`, not `cancelled`. The UI must not claim remote cancellation.

## Ownership and visibility

`owner` is stored server-side and constrains every query, resume, cancel and approval link; it is not a field of the public DTO and a body containing it is rejected (R1). Foreign or absent run IDs both yield `not_found`. Template visibility is installation-scoped for built-in templates and owner-scoped for any future user templates (not in v1).

## Persistence notes for 11-02

Tables (identity-style namespaced migrations): `workflow_templates(template_id, version, digest, body)`, `workflow_runs(run_id, owner, installation, template_id, version, digest, state, state_reason, parameters, budgets_used, created_at, updated_at)`, `workflow_run_targets`, `workflow_steps(run_id, step_id, target_server_id, state, …)`. Constraint: unique `(run_id, step_id, target_server_id, operation_id)` and a check that at most one operation exists per mutate step and target. Output text is not stored by default (consistent with Spec 06).
