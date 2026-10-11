# Spec 11 design — bounded workflow contracts (step 11-01)

Status: design, 2026-10-11. Scope: Package A (deterministic guided workflows) only. Packages B (WebMCP) and C (embedded assistant) are optional and are **not** designed or enabled here. No product code. Companion documents: [model.md](model.md) (template model and lint rules), [state-machine.md](state-machine.md) (run/step states, resumability, authority), [fleet.md](fleet.md) (fleet comparisons). Machine-readable artifacts: [schemas/](schemas/), [fixtures/](fixtures/), [tool-allowlist.json](tool-allowlist.json), and the checker [validate.py](validate.py).

Inputs: [Spec 11](../../11-workflows-and-agent-extensions.md), frozen contracts ([HANDOFF](../../contracts/HANDOFF.md)), the governance state machine ([design/06](../06/README.md)), the MCP adapter design ([design/04](../04/README.md)), the identity design ([design/02](../02/README.md): delegated authority), Specs 08 and 10 (resource and metric tools).

## Principles

1. **A workflow is data, not code.** A template is a validated JSON document interpreted by a deterministic runner. It cannot contain commands, scripts, expressions or dynamic instructions.
2. **A closed step vocabulary.** `inspect`, `compare`, `checkpoint`, `suggest`, `mutate`, `verify`. Nothing else exists.
3. **Only registered, structured tools.** Steps name a tool and schema version from the allowlist ([tool-allowlist.json](tool-allowlist.json)). `execute_command` and any raw-shell tool are denied outright (L4). A tool name discovered at runtime is never enough (L3).
4. **No new authority.** Every mutation goes through the existing prepare → policy → approval → atomic consume → durable audit → submit path (Spec 06). A template, a run, or the word "workflow" is never approval. The server's `approval_mode` (D11) applies unchanged; denials are final.
5. **Remote output is untrusted data.** It is rendered as text and may feed a later step only through a closed validator (`service_unit`), and only into declared arguments (L6). It can never become a command.
6. **Bounded everything.** Steps, tool calls, duration, observation bytes, targets and concurrency all have hard caps in the schema; budgets are enforced by the runner.
7. **At most one dispatch.** A submitted mutation is never replayed or resumed. An unknown outcome stops the run until a human reconciles (see [state-machine.md](state-machine.md)).
8. **Honest results.** Missing data is a gap, never zero; partial fleets are labeled partial; verification is `verified`, `mismatch` or `unavailable`, never inferred from an exit code.
9. **Owner-scoped.** Owner is never part of a public DTO; it comes from the authenticated principal and constrains every read and write. Targets are owned servers resolved per step.

## First templates (supported queries only)

| Template | Kind | Purpose | Tools used |
|---|---|---|---|
| `failed-service-diagnosis` | troubleshooting (read-only) | Inspect a service; if failed, collect recent logs; suggest the restart template with the observed unit | `get_service`, `query_logs` |
| `disk-usage-fleet` | fleet comparison (read-only) | Compare disk usage across up to 10 owned servers, partial failures reported | `get_disk_usage` |
| `restart-service` | maintenance | Inspect → restart (approval-bound) → verify service state | `get_service`, `service_action` |

Resource-pressure troubleshooting (CPU/memory) uses `query_metrics` and joins this set when Spec 10 freezes its metric contract. Tool IDs for Specs 08 and 10 are **provisional**: [tool-allowlist.json](tool-allowlist.json) marks them so, and a template referencing an unregistered tool is rejected at publish time.

## What is frozen here, and how it enters the shared contract

Frozen in this step (v1, documented and fixture-tested): the template schema, run progress schema, fleet comparison schema, lint rules L1–L9 / R1–R5 / F0–F3 and the closed step and parameter vocabularies. Versions are carried in `schema_version` (`linux-mcp/workflow-template/v1`, `linux-mcp/workflow-run/v1`, `linux-mcp/fleet-comparison/v1`).

These are **additive extensions** of the Spec 00 contract. The dashboard (browser) needs the public run/fleet DTOs, so they belong in `linux-mcp-contracts` as a new `public::workflows` module with schemas under `schemas/workflows/`. Per the HANDOFF extension process this needs the contracts owner: a `CONTRACT_VERSION` minor bump (1.1.0), an entry in `tests/schema_compat.rs` exact-set assertions and the fixtures from this directory. This step reserved `contracts-registry` but deliberately does **not** edit the frozen crate; the request is recorded here for 11-02 (or a dedicated contracts step) to execute under the proper reservation and FROZEN-manifest update (`python scripts/verify_contracts_freeze.py write`).

## Acceptance trace (step 11-01)

| Acceptance | Evidence |
|---|---|
| Dynamic unapproved shell instructions cannot become workflow steps | no command-bearing step kind or parameter type exists in the schema; `execute_command` denied (L4); parameter names that look like shell input rejected (L5); fixtures `raw-shell-tool`, `parameter-named-command`, `unregistered-tool`, `unknown-argument-name` |
| All mutation authority remains the approval/policy boundary | `mutate` steps only prepare an operation intent; no field on a template, run or step can express approval; [state-machine.md](state-machine.md) transitions all go through Spec 06 states; fixtures `mutation-in-troubleshooting`, `read-only-flag-mismatch` |
| Template, progress and fleet schemas frozen with fixtures | `schemas/`, 10 valid + 26 invalid fixtures, `python specs/design/11/validate.py` exits 0 |
| Bounded steps/budgets and per-step observed results | budgets schema with hard caps; `StepProgress.observation`; fixtures `missing-budgets`, `budget-above-hard-cap`, `steps-exceed-budget` |
| Pause/approval/dispatch/verify/unknown/reconcile and safe resume | [state-machine.md](state-machine.md); run fixtures `awaiting-approval`, `unknown-outcome`, `completed-verified`, `verification-unavailable`, `blocked-revision-changed`; invalid `second-dispatch-after-unknown` (R3) |
| First templates limited to supported service/metric queries | three templates above; allowlist marks provisional tools |

## Open items

- Reconcile provisional tool IDs and argument names with the frozen Spec 08 and 10 contracts (08-01, 10-01) before 11-02; any mismatch is a template-schema `tool.schema_version` bump, not a runtime guess.
- `suggest` hands a validated parameter set to the dashboard; the dashboard (Spec 05) must present the exact resolved parameters and require a separate human action to start the maintenance run. That UI is not designed here.
- Run scheduling (recurring maintenance) is out of scope for v1; runs start only by an explicit user action.
