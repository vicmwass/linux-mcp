# Monitoring/workflow states

Monitoring: loading, empty history, explicit gap/unavailable sample, counter reset, stale collection, disconnected live query, unauthorized owner, unsupported field. Units and actual collection time stay attached to each series; no fabricated healthy zero.

Schedules: enabled, quota-invalid, expired/revoked grant (stopped), deleted target (stopped), disconnected/backoff, next-run, no schedule. Delegated read-only authority and bounded budgets are explicit; never silently use broader credentials.

Alerts: no alerts, pending window, sustained breach, recovery, unavailable data, deduplicated recurrence; rule validation distinguishes thresholds/units/window and unavailable-data behavior. Initial channel is inbox.

Troubleshooting/fleet: read-only progress, partial evidence, per-target unavailable/time, stale target revision, unsupported step, unauthorized/revoked scope. Show count of successful observations and total, never conceal incomplete target results.

Maintenance: exact resolved intent, approval-required/expired, submitted-once/running, refused, failed-before-submit, unknown-after-submit, observed verified/mismatch/unavailable, paused/revoked, restart/reconciliation. Broad workflow names never approve dynamic actions. Unknown mutation blocks dependent continuation; read-only inspection comes first.

Every page uses shared navigation and state controls; no AI/WebMCP/provider is present or needed. External notifications and model configuration remain optional separate work.
