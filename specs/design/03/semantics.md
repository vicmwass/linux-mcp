# Spec 03 semantics: submission, outcomes, limits, isolation

## Submission boundary

Per submitted operation (a single privileged or ordinary exec; user tools run read-only prechecks first):

```text
NotSubmitted ──(audit.record_submitted ok)──▶ Submitted ──(exec reply + stream drained + exit status)──▶ Completed
                                                    └─ any transport/protocol error, timeout, cancel ──▶ outcome UnknownAfterSubmission
```

1. All work before the exec request is *not submitted*: resolve, revision recheck, credential lease, connect, authenticate, host-key check, capability prechecks, read-only prechecks.
2. `Audit::record_submitted` is called immediately before `channel.exec`. If it fails, nothing is sent and the outcome is `FailedBeforeSubmission { AuditUnavailable }`.
3. Once `exec` is invoked the command may run. Every later failure is `UnknownAfterSubmission` with a cause: `Timeout`, `Disconnected`, `Cancelled`. This includes today's "Remote command did not return an exit status" (finding F6). The only exception to consider (flagged, unverified for russh) is an explicit channel-failure reply to the exec request, which would mean the server refused to start the command; until verified it is treated as unknown.
4. Nothing is replayed after `Submitted`, ever. The existing retry-once rule stays and applies only before exec: closed cached connection, or `ChannelOpenError`.
5. Timeout and cancellation (future drop) before step 2 → `FailedBeforeSubmission { Timeout | Cancelled }`.
6. Connections that errored or timed out are evicted; the next request reconnects.

## Outcome mapping

| Situation | `OperationOutcome` | `SubmissionState` |
|---|---|---|
| policy denial, approval missing/invalid, revision changed, invalid arguments | `Refused { reason }` | `NotSubmitted` |
| connect/auth/host-key/credential/audit failure before exec | `FailedBeforeSubmission { cause }` | `NotSubmitted` |
| command ran, any exit code (including non-zero) | `Succeeded { exit_status, stdout, stderr }` | `Completed` |
| stream/channel error, timeout, disconnect after exec | `UnknownAfterSubmission { cause, partial output }` | `Submitted` |
| process crash between pending and outcome | recovery marks `UnknownAfterSubmission { Unreconciled }` (Spec 06) | `SubmissionPending`/`Submitted` |

The legacy adapter converts these back to the current text results (`Exit code: N\nSTDOUT:…`, `Command execution failed:\n…`, etc.), so the 00-08 legacy fixtures remain valid. The typed outcome is richer than the text; the text is never the source of truth.

## Bounded output and duration

- Per-stream cap 1 MiB (`ExecutionLimits` default, now enforced). Bytes beyond the cap are discarded while draining; `BoundedOutput { truncated: true, original_bytes }` reports it. Truncation is on a UTF-8 boundary; invalid sequences are replaced and flagged `lossy`.
- Operation timeout default 60 s (existing); covers connect + queue + exec. Dedicated user tools and capability detection keep their 30 s per-command timeouts (all user-tool calls, mutations included, use 30 s today).
- Concurrency: per connection serialized by the existing mutex; per user a configurable ceiling (recommended 4) enforced before connecting; exceeded → `unavailable` with `retry_after`. A mutation holds its connection for the duration, so one in-flight mutation per server per user by construction.

## Isolation keys

- Pool, capability cache and any trust cache use `TargetCacheKey { owner, server_id, ssh username, revision }`. No key contains only a hostname or label.
- A credential, trust, host/port/user or enabled change bumps the revision (Spec 01), so old keys become unreachable; `invalidate_target` additionally closes idle connections early. A missed event is safe because `ensure_current` is checked at every use.
- Connection reuse across operations of the same key is allowed; across keys never. Two owners with identical hostname, port and user get separate connections, lease their own credentials and verify their own host fingerprint.
- Foreign or absent targets fail in the resolver before any lease, DNS lookup or connection (Spec 00/01); the executor never accepts a bare `ServerId`.

## Secrets

- Key and sudo-password material arrive as `CredentialLease` (≤ 60 s). They are copied into the SSH library or stdin only for the operation; sudo stdin is zeroed after use (existing behavior, kept); leases are not stored in pools.
- Errors, audit events and logs never include secret bytes, key paths (multi-user), command output beyond the bounded result, or foreign server metadata. The existing sudo error tests (empty/multiline/NUL secret, no disclosure) move unchanged.
- The legacy adapter keeps reading password and key files; that code path is compiled only for the legacy adapter module.

## Threshold layering (decision D11)

```text
raw tool call → classify → SecurityPolicy::evaluate ─ Denied ─▶ Refused (final; approval cannot override)
                                         │ not denied
                                         ▼
                 target.approval_mode().requires_approval(risk) ─ yes ─▶ RequiresApproval
                                         │ no
                                         ▼
                                      Allowed
```

`HighAndAbove` therefore also stops requiring confirmation for medium sudo operations that the legacy rule gated; this is the owner's explicit, interactive-session-only choice. Low risk is never gated. The legacy adapter bypasses the threshold and uses the unchanged confirmation rule.

## Cancellation guarantees (documented for consumers)

- Before the submission marker: dropping the future guarantees nothing ran.
- After it: dropping the future closes the local channel but cannot recall the remote command; the result is `UnknownAfterSubmission { Cancelled }` and the approval stays consumed.
