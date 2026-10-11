# 03 — Tenant-aware execution core

Implementation: [step-by-step plan and dependencies](plans/03-execution-implementation.md). Follow the [coordination workflow](plans/README.md).

## Deliverable and ownership

Extract existing parsing, resolution, capability detection, policy, user tools, execution and SSH into `crates/execution-core`. Preserve working legacy stdio behavior through an adapter. Own execution logic only; use mock target and credential providers until 01 integrates.

## Requirements

- Every network operation requires an authorized target resolved for the trusted Principal. Recheck ownership/enabled state and revision immediately before submission.
- Cache keys include owner, server ID, effective SSH username and server revision. Apply the same isolation to host trust, capability caches and connection pools.
- On inventory, trust or credential changes, prevent new work from using old connections; invalidate old caches and close idle sessions. Never replay submitted mutations. Document handling of running operations.
- Preserve existing command classification, policy refusals, user validation, sudo secret isolation, timeouts and host-key checks. Preserve current tool semantics, including that password locking does not revoke SSH keys.
- Separate preflight policy evaluation from execution so 06 can bind an approval to the exact resolved operation. Re-evaluate policy and target revision at submission.
- Preflight evaluation uses the resolved target's current `approval_mode`: under `medium_and_above` (default) medium, high and critical-risk mutations require approval; under `high_and_above` only high and above do. Low risk never needs approval, and policy denials (unknown/critical operations, catastrophic deletes) are unaffected by the mode. Re-evaluate at submission against the freshly resolved mode, so a mode change after approval rejects the old authority.
- Bound command output, duration and per-user concurrency; cancellation must distinguish work never submitted from work with an unknown remote outcome.
- SSH errors and audit events cannot reveal secrets. Emit submission evidence for operation tracking.

## Acceptance

Existing meaningful tests pass after extraction. Two users with identical server labels/hostnames never share sessions or capability data. Rotating credentials or changing host trust stops reuse of old sessions for new work. Foreign-owned targets fail before secret resolution or network connection. Disconnect-after-submit yields unknown outcome without retry. Tests use an SSH test adapter; live tests use disposable explicitly configured machines.
