# Spec 00 — Step-by-step implementation plan

Status: ready for scheduling, implementation not started. Source: [Spec 00](../00-contracts.md). Scheduling authority: [step registry](00-contracts.steps.json) plus coordinator state. Follow [coordination workflow](README.md) and local `AGENTS.md` for every step. Never edit this document to claim work.

## Scope and sequencing

Build contracts and mocks only, not real authentication, SQL storage, SSH changes, HTTP services or production UI. Preserve current server behavior. This plan creates the stable boundary needed by Specs 01–06. Provider selection and real OAuth compliance belong to 02/04; define provider-neutral identity boundaries here without claiming protocol compliance.

```text
00-01 -> 00-02 -> 00-03 -> 00-04 --+-> 00-06 --+
                          |       +-> 00-07 --+-> 00-09 -> 00-10 -> 00-11 -> 00-12
                          +-> 00-05 -> 00-08 -+
```

Exact prerequisites are in the registry: 06/07 both require 04 and 05; 08 requires 05. After 03, steps 04/05 can run concurrently; after their prerequisites, 06/07/08 can run concurrently. Resource claims prevent overlapping shared-file edits. Each module supplies its own module file and test file; only 02/10/12 edit central exports/manifests. Agents may use temporary external probe crates for local checks before central integration; do not change central registration without its resource reservation.

The prospective paths below become authoritative in 00-01's design map. If a scope change introduces overlap, stop and reconcile the frozen registry rather than editing another worker's files.

## 00-01 — Audit and design baseline

Prerequisites: none. Reserve `contracts-design`, `architecture-doc`.

1. Read Cargo manifests/lockfile, current MCP handlers, config, capability, policy, execution result and SSH boundaries. Inspect existing tests and unrelated working-tree changes; do not overwrite them.
2. Record actual existing tool input/output schemas and distinguish current behavior from proposed approval extensions.
3. Write `specs/contracts/decisions.md`: crate/feature layout, identity trust boundary, ID/time/revision formats, async trait object strategy, schema library, cancellation semantics, secret-handle lifecycle and result/error mapping.
4. Write `specs/contracts/ownership.md` assigning module paths below to resources. Specify no browser dependency on backend secrets/identity. Prefer `public`, `backend`, `test-support` feature boundaries and validate exact defaults during 02.
5. Establish operation canonicalization and approval invariants; capture unresolved design questions with recommended choices. An unresolved security-critical boundary blocks this step, rather than being silently deferred to a consumer.
6. Run and record baseline `cargo test --locked` (offline if dependencies are available). Distinguish pre-existing failures from new work. Update the architecture explanation with planning/extraction boundaries, not implemented features.

Acceptance: design/ownership documents cover every 00 requirement, current tool mapping and proposed extensions; baseline results recorded; no runtime change. Completion evidence identifies artifacts, decisions and any justified nonblocking limitations.

## 00-02 — Workspace and crate skeleton

Prerequisite: 01. Reserve `workspace-manifests`, `contracts-registry`, `architecture-doc`.

1. Add workspace support while retaining the current root package and entry point. Add `crates/contracts` with a stable crate name and minimal dependencies selected from 01.
2. Set up feature-gated module structure for public DTOs, backend-only contracts and test-support. Browser/default builds must not include secret providers or production identity adapters.
3. Create declared module placeholders so later agents edit only their own files; publish exact export registration instructions for 10. Keep this step compilable.
4. Configure shared test support and document commands for each feature combination. Update the lockfile deliberately; do not upgrade unrelated dependencies.
5. Run root regression tests and contracts checks for the supported feature combinations; update architecture explanation for the actual workspace change.

Acceptance: existing application still builds/tests, minimal contracts crate compiles and has no dependency on implementation crates. No authentication/database/SSH implementation is introduced.

## 00-03 — Public models and errors

Prerequisite: 02. Resource `contracts-public-models`. Own `crates/contracts/src/public/{ids,inventory,errors,pagination}.rs` and corresponding module tests.

1. Implement opaque IDs and revision with validated parsing, explicit wire representations and checked revision increments. Include installation identity where public contracts require it.
2. Implement server summary/create/update DTOs, environment/trust status, pagination cursors, UTC timestamps and revision-conflict information. User-submitted DTOs cannot set their owner.
3. Define sanitized public errors and request IDs. Foreign-owned/absent resources share externally observable error semantics.
4. Provide representative round-trip fixtures and invalid-input tests. Keep credential content and backend target details out of responses.

Acceptance: serialization and invalid-input tests pass; IDs cannot be mixed accidentally in Rust APIs; server summaries never include secrets; errors do not leak provider internals. Document fields and validation constraints for 04–08.

## 00-04 — Identity and credential boundaries

Prerequisite: 03. Resource `contracts-backend-models`. Own `src/backend/{identity,target,credentials}.rs` and boundary tests.

1. Define trusted Principal, scopes, installation identity and session/token identity. Construction is available only through a documented trusted-backend adapter boundary, never public deserialization.
2. Define resolved/authorized target and owner-scoped credential/trust references with revision and enabled-state evidence. An identifier alone is not an authorized target.
3. Define nonserializable redacted credential handles and bounded access/lifetime strategy. State which values require memory clearing and what clearing cannot guarantee.
4. Ensure public/browser builds cannot import backend-only modules. Add compile-fail or feature-boundary checks for secret serialization and unsafe Principal construction from untrusted DTOs.

Acceptance: backend integration has a documented construction path; public builds cannot expose private types; debug/serialization boundaries are tested. No mock credential can accidentally select an arbitrary host path.

## 00-05 — Operations, approval and outcomes

Prerequisite: 03. Resource `contracts-operation-models`. Own `src/public/operations.rs`, `src/backend/operation_authority.rs` and their tests.

1. Define operation intent, tool identity, target revision, canonical argument representation, risk, expiry and approval reference.
2. Specify deterministic canonicalization and explicitly exclude secrets from intents. Include algorithm/schema version so a later change invalidates stale approval bindings safely.
3. Separate requested intent, policy evaluation, human approval evidence and executable authority. A public boolean or arbitrary approval ID cannot manufacture execution authority.
4. Define submission state, bounded output/truncation and succeeded/refused/failed-before-submit/unknown-after-submit outcomes. Preserve exit status separately from transport/policy status and verification.
5. Add exact-binding, expiry, altered-arguments and unknown-outcome fixtures, plus sensitive-field rejection/redaction checks.

Acceptance: approval binds identity, operation, target revision and arguments; result types cannot imply a disconnect was a definite failure after submission. Publish fixtures needed by 06–08.

## 00-06 — Management API schemas

Prerequisites: 04, 05. Resource `contracts-management-api`. Own `src/public/api.rs`, `schemas/management/`, fixtures and tests.

1. Define `/api/v1` request/response contracts for inventory CRUD, credential provisioning/rotation, host-trust observation/enrollment, approval requests/decisions/status and owned audit queries.
2. Specify route/method/status mappings, pagination, timestamps, revision preconditions, validation and sanitized error envelopes with request IDs.
3. Mark credential inputs write-only; never put them in response types or fixture real values. Host-key observation is not verified enrollment.
4. Generate/check schemas with the library selected in 01 and provide success/failure examples. Require caller identity from backend context, not request owner fields.

Acceptance: every management operation in 00/01/06 has a schema and example; stale updates, foreign IDs and approval errors have consistent contracts; no production handlers are added.

## 00-07 — Async service traits

Prerequisites: 04, 05. Resource `contracts-service-traits`. Own `src/backend/services.rs` and compile probes.

1. Define Inventory, TargetResolver, CredentialStore, Identity, Executor, Approvals and Audit interfaces from Spec 00.
2. Include request context, deadlines/cancellation, pagination and typed outcomes. Separate public authentication inputs from trusted authenticated context.
3. Make secure resolution a prerequisite to secret access/execution. Define ownership checks before network access and revision rechecks before submission.
4. Define atomic consume/dispatch boundaries, audit ordering and invalidation events with revision fallback. Specify what cancellation can and cannot guarantee after submission.
5. Compile minimal object-safe adapter implementations and document call sequences without implementation-crate dependencies.

Acceptance: every required operation has a compilable signature and error semantics; ordinary consumers cannot bypass authorization through a general secret getter. No SQL/HTTP/russh dependency enters contracts.

## 00-08 — MCP compatibility fixtures

Prerequisite: 05. Resource `contracts-mcp-fixtures`. Own `schemas/mcp/`, `fixtures/mcp/`, MCP compatibility documentation and tests.

1. Capture current tool schemas and actual result envelopes from source, not assumptions in older README prose. Label fixture provenance and current versus proposed version.
2. Map current tools to shared IDs/results and represent policy refusals distinctly from protocol errors. Document where current unstructured output needs an adapter.
3. Define versioned approval-required/prepare/submit extensions and agent-facing status/results. Preserve legacy stdio semantics explicitly; never silently strengthen a boolean into approval evidence.
4. Provide initialization/discovery/call fixtures and incompatibility cases so dashboard and service teams can build separately.

Acceptance: existing tool shapes match source; proposed extensions are visibly versioned; unknown schema versions are rejected/disabled rather than guessed. This step does not claim live MCP transport interoperability.

## 00-09 — In-memory adapters and scenarios

Prerequisites: 06, 07, 08. Resource `contracts-mocks`. Own `src/test_support/`, scenario fixtures and adapter tests.

1. Implement all service traits with deterministic in-memory state and controlled clock/network/executor outcomes. Use thread-safe state for atomic approval consumption and revision updates.
2. Add Alice/Bob, same hostname with different ownership/credentials, missing trust, changed revision, revoked identity, policy refusal and approval-required scenarios.
3. Add timeout-before-submit and disconnect-after-submit. Record execution attempts to prove no replay/foreign connection occurred.
4. Implement reset/seed APIs, fake secret markers and a demo scenario entry point. Test-support is opt-in and never a production auth bypass.
5. Test concurrency and revocation semantics; distinguish simulated behavior from real database/OAuth/SSH guarantees.

Acceptance: consumers can instantiate every trait without external services; cross-owner access fails; concurrent approval consume allows one dispatch; scripted failures produce exact contract outcomes.

## 00-10 — Register and validate the assembled crate

Prerequisite: 09. Reserve `contracts-registry`, `workspace-manifests`, `contracts-schema-tests`, `architecture-doc`.

1. Integrate module exports and feature flags; resolve naming/type drift through module owners and recorded contract decisions.
2. Wire schema-generation/compatibility checks and fixture round trips. Pin contract/schema versions, including canonicalization.
3. Run `cargo fmt --all -- --check`, root regression tests, contracts tests with all supported features and the browser/public feature build. Select exact package names from 02.
4. Inspect dependency graph for reverse dependencies and backend leakage. Fix only integration-owned files; any module correction requiring another resource must be explicitly coordinated.
5. Update architecture explanation to reflect the contracts crate and unchanged runtime behavior accurately.

Acceptance: one integrated contracts crate builds and tests; wire schemas match Rust types; current application remains functional. Failures return this step to blocked/pending rather than bypassing acceptance.

## 00-11 — Consumer and security probes

Prerequisite: 10. Reserve `contracts-consumer-probes`, `architecture-doc`. Own `crates/contracts/examples/` and `tests/consumer-probes/`.

1. Create minimal compile/run probes representing inventory, identity, executor, MCP adapter, dashboard/public build and governance consumers. These are mocks/examples, not production component skeletons that claim real functionality.
2. Run Alice/Bob isolation, revoked identity, stale target, changed arguments, secret redaction, expired/reused approval and unknown-outcome scenarios across the mock call sequence.
3. Build browser/public contract usage for `wasm32-unknown-unknown` when the target is installed; document missing tooling precisely. Required release validation cannot be marked passed by skipping the build; if unavailable, block this step until it can be run.
4. Record exact commands and results in `specs/contracts/validation.md`; separate real compile checks from simulated security behaviors. Update architecture validation summary.

Acceptance: all consumer probes and required boundary checks pass; no real secrets/external machine access; no claim that mock isolation proves production isolation.

## 00-12 — Freeze and handoff

Prerequisite: 11. Reserve `contracts-design`, `contracts-registry`, `architecture-doc`.

1. Trace every Spec 00 requirement to an artifact and passing test in `specs/contracts/acceptance.md`.
2. Publish `specs/contracts/HANDOFF.md`: crate/features, adapter examples, stable API/MCP schema versions, ownership map, commands, compatibility policy and known limitations.
3. Record the exact frozen artifact revision/checksums; explain how later resource schemas extend the contract without silently breaking consumers. Do not create a Git commit or tag merely to fill a field unless authorized.
4. Run final compatibility/regression checks after any handoff-driven changes and update the architecture explanation.
5. Record the product-owner decision of 2026-10-11 (per-server `ApprovalMode`, default `medium_and_above`, toggle `high_and_above`, restricted to interactive sessions) in decisions D9/D11, acceptance, handoff and the affected specs 01/03/04/05/06/12.
6. Complete the coordinator step with evidence, refresh status and confirm all 12 steps are completed. Only then report Spec 00's contract gate passed. Subsequent spec implementations require their own registered detailed plans.

Acceptance: complete traceability, reproducible validation, no unresolved blocking design decisions and a consumer-ready frozen contract. Any remaining security-critical or compile failure blocks the freeze.
