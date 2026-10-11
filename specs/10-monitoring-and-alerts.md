# 10 — Monitoring, history and alerts

Implementation: [step-by-step plan and dependencies](plans/10-monitoring-implementation.md). Follow the [coordination workflow](plans/README.md).

Status: proposed. Independently extends the client roadmap after metrics contracts are frozen.

## Deliverables and boundaries

Own `crates/monitoring`, its migrations, collection scheduler, metrics history API and alert evaluator. Own dashboard monitoring modules through 05's extension points. Add structured CPU, memory, disk and network queries through resource-tools and 04. This work can run alongside 09; it does not depend on package/file mutation support.

Collection is application-owned and uses an MCP client with explicit user-delegated, read-only authority. Do not use an unrestricted global service credential. Define grant expiration/refresh and revocation; stop collection when authorization, ownership or enabled state changes. Define exactly one collector owner per schedule across replicas, using leases and deduplication.

## Contracts and behavior

- Metrics include owner, server/revision, observation time, units, sampling interval and supported/missing fields. Counters define reset semantics. Missing samples are gaps, never zero values.
- Bound per-user frequency, concurrent SSH work, history volume and query range. Use backoff on failures and define retention/aggregation.
- Alerts bind owner, target, metric, threshold, evaluation window and delivery settings. Deduplicate notifications and distinguish recovery, sustained breach and unavailable data.
- First delivery channel is an in-dashboard alert inbox. External notification channels are separate adapters with explicit user configuration and secret handling.
- History and alerts remain owner-scoped after machine edits/deletion; define retention and label historical revisions honestly. Store numeric metrics rather than arbitrary raw command output.

## Acceptance

Fixture time-series cover counter reset, sample gaps, threshold flapping, sustained breach and recovery. Replicas do not duplicate scheduled work. Revoked grants and deleted machines stop new collection. One user's schedules cannot exhaust another's configured share. History, alert queries and UI isolate users. Restart preserves schedules and history; outage does not cause a catch-up storm.
