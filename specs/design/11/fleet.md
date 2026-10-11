# Spec 11 fleet comparison semantics

Schema: [schemas/fleet-comparison.schema.json](schemas/fleet-comparison.schema.json). Fixtures: [fixtures/fleet](fixtures/fleet), [fixtures/invalid-fleet](fixtures/invalid-fleet). Template example: `disk-usage-fleet`.

## Scope and bounds

A fleet comparison compares **owned** servers only: the user selects targets (≤ 20, template default ≤ 10); each is resolved through `TargetResolver` for the run owner, so a foreign ID is `not_found` like any other. Collection runs with bounded concurrency (≤ 4) through the MCP service, one read-only tool call per target per `inspect` step, each with its own timeout. Fan-out never exceeds the template's `max_targets`, `max_concurrency` or `max_tool_calls`.

## Result shape

`FleetComparison` has `requested_count`, `included_count`, `columns` (field pointer, label, unit), one `row` per requested target, `aggregates`, `max_age_s`, `complete` and `partial_reason`.

Row `status`: `collected`, `failed` (call error/timeout), `unavailable` (target unreachable, trust or credential missing), `unsupported` (capability/tool missing on that machine), `skipped_budget`, `stale` (an observation older than `max_age_s`). Every row carries the target's `server_revision` at collection and, when present, `collected_at` and `age_s`.

## Rules (lint F0–F3)

| Rule | Meaning |
|---|---|
| F0 | Document does not satisfy the schema |
| F1 | `complete` must be true **iff** every requested row is `collected` and `partial_reason` is null. A comparison with any failed, unavailable, unsupported, skipped or stale row is partial, with a reason |
| F2 | A missing observation is a gap: `values` is `null` for every non-collected row, and never a fabricated `0` or default. A collected row must have values |
| F3 | Counts are honest: `included_count` equals the number of collected rows, and every aggregate states `included_count` and `requested_count` so a maximum over 1 of 3 servers can never be read as a fleet-wide maximum |

## Presentation requirements (for Spec 05/12)

- Partial results show "N of M servers" next to every aggregate and list each excluded server with its reason; failed rows are visible, not hidden.
- Units come from the column; times are UTC and show collection age; stale values are marked with text, not colour alone.
- Remote strings in `values` are untrusted text and are escaped.
- No automatic retry that could mutate: fleet runs are read-only by schema (`fleet_comparison` kind cannot contain `mutate`). Re-collection is a new run.
- Revocation or loss of authority mid-run turns not-yet-collected targets into `unavailable` with `partial_reason = authorization_lost`; collected rows stay as observed, labeled with their collection time.

## Owner scope and storage

Comparisons are persisted as part of an owner-scoped run (same ownership rules as [state-machine.md](state-machine.md)). Only numeric/string values defined by the template columns are stored, not raw command output. If a server is later edited or deleted, rows keep the historical `server_revision` and are labeled as such; they never claim to describe the current revision.
