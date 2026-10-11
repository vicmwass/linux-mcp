# 06 — Approvals and audit

Implementation: [step-by-step plan and dependencies](plans/06-governance-implementation.md). Follow the [coordination workflow](plans/README.md).

## Deliverable and ownership

Own `crates/governance` and its database migrations. Supply the Approvals and Audit contract implementations, authenticated management handlers, fixture scenarios and tests. Work against mock identity and preflight evaluation.

## Approval requirements

- Persist an immutable intent binding owner, target/revision, operation type, canonical parameters, risk, creation time and expiry. The browser displays this stored intent, not agent-provided descriptive text.
- Which operations need approval is set per server by `approval_mode` (decided in preflight by 03 from the resolved target): `medium_and_above` is the default, `high_and_above` is the owner's explicit opt-in. Approval records bind the target revision, so changing the mode (which bumps the revision) invalidates unused approvals. Neither mode can approve a denied operation or treat `confirm: true` as approval.
- Human approval occurs through an authenticated, CSRF-protected dashboard interaction. For the first release the owner is the approver; organizational multi-party approval is out of scope.
- Agents can request approval and check its status but cannot approve themselves using mutation scope or `confirm: true`.
- Atomically consume an approval once before dispatch. Changed arguments, target revision, identity, expiration or revoked permissions invalidate it.
- A consumed approval is never automatically reset after network ambiguity. New attempts require reconciliation and, where applicable, a new approval.
- Approval does not override a policy denial. The execution engine rechecks current policy.

## Audit requirements

Record owner, request/operation IDs, server ID/revision, tool/action, redacted parameters, approval evidence, submission state, timestamps and outcome. Record refusals and authorization failures without leaking foreign server metadata. Output is not stored by default because it may contain secrets; retention and optional redacted output storage are explicit installation settings.

Before mutation submission, persist an intent/submission-pending record; if durable audit is unavailable, reject new mutations. Record confirmed submission and final outcome when possible. Recovery marks unresolved dispatches as unknown, never assumes success or replays them. Logs complement durable audit and do not substitute for it.

## Acceptance

Concurrent consumption permits at most one dispatch. Cross-user, expired, altered and already-consumed approvals fail. Crashes before/during/after submission leave reconcilable records. Audit reads are owner-scoped. Secret redaction and retention tests pass. The UI and direct MCP agent flows use the same approval implementation.
