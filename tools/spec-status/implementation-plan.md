# Local spec status viewer implementation plan

Development tooling only; no product spec or scheduling topology changes.
Registry: `steps.json`, selected explicitly with the existing shared state directory.

## viewer-01 — Implement and verify the read-only viewer

Own `scripts/spec_status_server.py`, `scripts/test_spec_status_server.py`, and
`tools/spec-status/**`; reserve `architecture-doc` before architecture edits.
Read the catalog, registries and atomically replaced state directly, without
coordinator commands, writes or transaction locks. Serve a loopback-only HTML
page and allowlisted JSON projection. Poll every three seconds, show required
and optional progress separately, and preserve stale data on errors. Keep
revision plans distinct and show attempt/evidence and dependency waits.

Acceptance: unit/HTTP tests for unchanged source bytes and mtimes, no token
exposure, registry drift/missing/corrupt state, reopen progress, optional work,
atomic replacement and restricted routes. Live endpoint and browser smoke
check. Update architecture documentation with actual validation and limitations.
Record completion through the coordinator without modifying existing registries.
