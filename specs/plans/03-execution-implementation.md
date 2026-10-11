# Spec 03 — Execution core implementation

Source: [03](../03-execution-core.md). Registry: `03-execution.steps.json`. Follow [shared rules](IMPLEMENTATION-RULES.md). Own `crates/execution-core/` and extraction changes to existing `src/{commands,execution,security,ssh,tools,capabilities}` only during reserved extraction/integration steps. Checks: existing regression tests, core tests with fake SSH transport, bounded-output/concurrency tests and designated disposable SSH smoke targets at the final gate.

## 03-01 — Record behavior and extraction boundaries

Dependencies: `00-contracts/00-12`. Resources: `execution-design`.

1. Map all current tool paths, classifiers, sudo handling, trust checks and reconnect behavior to frozen Executor contracts.
2. Capture existing regression commands and expected refused/failed/unknown outcomes.
3. Design provider adapters, preflight/submission split, pool keys and output/cancellation limits in `specs/design/03/`.

Acceptance: extraction preserves existing semantics and explicitly separates legacy single-user mode from multi-user authority.

## 03-02 — Extract library without behavior changes

Dependencies: `03-01`. Resources: `workspace-manifests`, `execution-extraction`, `execution-exports`, `legacy-runtime`.

1. Create execution-core crate and move/reexport existing implementation modules with minimal changes.
2. Adapt root application imports through a legacy adapter; retain stdio/TOML compatibility.
3. Establish module ownership for subsequent steps and rerun existing tests/checks.

Acceptance: root application and core build/tests pass; extraction introduces no HTTP/database/auth behavior or weaker policy.

## 03-03 — Implement authorized target and transport boundary

Dependencies: `03-02`. Resources: `execution-targets`.

1. Implement target/credential/trust provider consumption and fake SSH transport in the designated target adapter module.
2. Recheck owner/enabled/revision and outbound actual-address policy immediately before network submission.
3. Test foreign IDs produce no secret access/connect attempts; preserve host-key and sudo stdin isolation.

Acceptance: no unrestricted network/secret entry point bypasses trusted targets; unknown trust and changed DNS/policy fail closed.

## 03-04 — Implement tenant-aware pools and capability cache

Dependencies: `03-03`. Resources: `execution-caches`.

1. Key all pools/cache entries by owner, server ID, username and revision.
2. Subscribe to invalidation and add current-revision fallback for missed events.
3. Evict idle old sessions and define active-operation handling; test same hostname/labels across users and rotation races.

Acceptance: new work never reuses another owner's or stale-revision session; already-submitted commands are not replayed or falsely canceled.

## 03-05 — Split preflight and guarded submission

Dependencies: `03-03`. Resources: `execution-policy`.

1. Implement intent evaluation and separately authorized submission through contract authority types.
2. Re-evaluate policy and target revision at submission; preserve dedicated-user safeguards and classified command refusal.
3. Test altered arguments/revision, bypass attempts, protected users and policy denial even after approval.

Acceptance: intent/approval cannot become arbitrary command authority; root legacy confirmation semantics remain documented separately.

## 03-06 — Implement limits and submission outcomes

Dependencies: `03-03`. Resources: `execution-limits`.

1. Bound output bytes, operation duration, queueing and per-owner concurrency through transport/task wrappers.
2. Record definite-before-submit versus possibly-submitted states, including channel failure/timeouts/cancellation.
3. Test disconnect after submit, output flood, queue timeout and fairness; never replay a possibly submitted mutation.

Acceptance: stable typed outcomes/truncation and submission evidence; secret-bearing errors/logs remain redacted.

## 03-07 — Compose core and legacy regression adapter

Dependencies: `03-04`, `03-05`, `03-06`. Resources: `execution-exports`, `legacy-runtime`, `execution-integration`.

1. Wire the complete Executor adapter and map existing tool calls through it.
2. Run full fake-transport contract/concurrency suite and root regression tests.
3. Verify legacy single-user entry point still rejects unsupported operations and trusts only configured host keys.

Acceptance: no stale direct-call execution route survives extraction; legacy behavior regression evidence is recorded.

## 03-08 — Validate real providers and publish handoff

Dependencies: `03-07`, `01-inventory/01-08`. Resources: `execution-live-tests`.

1. Exercise inventory/credential/trust adapters and safe inspection on disposable hosts; test key mismatch and credential rotation.
2. Verify disconnect/no-replay and bounded outputs with controlled fault injection, not production disruptions.
3. Publish `specs/handoffs/03.md` with executor integration, limits and known remote-outcome constraints.

Acceptance: real provider/SSH smoke checks pass; governance/agent route enforcement is validated separately by 04/07.

## Scope addition (2026-10-11): per-server approval mode

Preflight evaluation reads the resolved target's `approval_mode` (`ApprovalMode::requires_approval(risk)` from the contracts crate); re-evaluate at submission against the freshly resolved mode. Denials are never affected by the mode. Tests: same operation needs approval under the default and runs under `high_and_above`; high risk still needs approval; denial unchanged. Decision record: [contracts decisions D11](../contracts/decisions.md).
