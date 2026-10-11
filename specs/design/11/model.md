# Spec 11 template model and lint rules

Schema: [schemas/workflow-template.schema.json](schemas/workflow-template.schema.json). Rules are implemented for design-time checking in [validate.py](validate.py) and must be re-implemented identically in the Rust runner (11-03+).

## Template

```text
WorkflowTemplate { schema_version, template_id, version, title, kind, read_only,
                   parameters[≤8], targets{mode,max_targets≤20,max_concurrency≤4},
                   budgets{max_steps≤20,max_tool_calls≤100,max_duration_s≤3600,max_observation_bytes≤256KiB},
                   steps[1..20] }
```

`kind`: `troubleshooting` and `fleet_comparison` are strictly read-only; only `maintenance` may contain `mutate`. `read_only` must equal "no mutate step" (L8) so a consumer can trust the flag without scanning.

A template is immutable once published: a change is a new `version`. A run records `template {id, version, digest}`; the digest is over the canonical template (`lmcp-canon-v1`), so a run is always replayable for audit against the exact definition it used.

## Parameters

Closed types only: `server_ref` (an owned server chosen by the user, resolved per step), `service_unit` (validated unit name), `integer` (with `minimum`/`maximum`), `enum` (closed `values`), `boolean`. There is **no free-form string type**, so user text can never reach a tool argument. Names that look like shell input (`command`, `cmd`, `script`, `shell`, `exec`, `bash`, `sh`, as whole words) are rejected (L5). Values are validated when a run is created, again at each step.

## Step kinds

| Kind | What it does | Notes |
|---|---|---|
| `inspect` | one read-only registered tool call with bound arguments; stores a bounded observation | `on_failure`: `stop` or `continue` (continue only keeps a partial result; it never skips a safety check) |
| `compare` | computes a table cell per target from an earlier `inspect` observation (fleet) | requires `max_age_s`; stale data is labeled, never silently used |
| `checkpoint` | evaluates a predicate on an earlier observation; `if_false: stop` ends the run as `stopped(stop_condition)` | no side effects |
| `suggest` | proposes starting another (maintenance) template with validated parameters | data only; starting it is a separate human action with its own approval |
| `mutate` | prepares an operation intent for one mutating registered tool; waits for approval; dispatches once | `max_risk` ceiling (`medium`/`high`; critical impossible), `approval_expiry_s`, mandatory `verify_step`, `on_unknown` is always `stop` |
| `verify` | read-only registered tool call that observes the post-change state against an expectation | bounded `attempts` (≤5) and `interval_s` (≤30); outcome is `verified` / `mismatch` / `unavailable` |

## Argument bindings

Each argument is exactly one of: `{"literal": scalar}`, `{"param": name}`, or `{"observation": {"step", "path", "validator"}}`. Observation bindings are the only way remote output reaches a later step, and they are restricted:

- the source step must be earlier (L2);
- the validator must be the one the allowlist declares for that tool argument (`observable_validator`; currently only `service_unit` on the `service` arguments) (L6);
- arguments with a closed literal set (for example `service_action.action`) accept literals only (L3/L6);
- the resolved value appears verbatim in the approval screen, because approvals bind the stored, resolved intent (Spec 06), not the template.

## Lint rules (templates)

| Rule | Meaning |
|---|---|
| L1 | Document does not satisfy the schema (unknown fields, types, caps, enum values, missing budgets, unsupported validators, `on_unknown` other than `stop`, risk ceiling `critical`) |
| L2 | Duplicate step IDs, or a reference that is not to an earlier step (verify may be later than its mutate; its `after_step` must be earlier) |
| L3 | Tool is not in the allowlist, wrong `schema_version`, wrong kind for the tool (mutating vs read-only), argument not defined for the tool, required argument missing, closed literal value violated |
| L4 | Denied tool (`execute_command` or any raw-shell tool) |
| L5 | Parameter name resembles raw shell input |
| L6 | Observation binding where it is not allowed or with the wrong validator |
| L7 | More steps than `budgets.max_steps` |
| L8 | Read-only integrity broken (mutate outside maintenance, flag mismatch) or `verify_step` is not a later `verify` whose `after_step` is this mutate |
| L9 | Unresolved parameter/step reference, default outside its range |

A template failing any rule cannot be published or run. Publishing also requires every referenced tool to be registered by the live MCP service for the target at the pinned schema version; a mismatch refuses the run before any call.

## What templates can never express

Shell text, scripts, expressions or conditionals beyond the single `checkpoint` predicate, loops other than the bounded `verify` retry, unbounded fan-out, approvals, credentials, owner or installation identifiers, raw output retention settings, or generic rollback. These omissions are structural: there is no field to hold them.
