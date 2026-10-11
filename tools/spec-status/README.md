# Live spec status

From the repository root:

```powershell
python scripts/spec_status_server.py
```

Open http://127.0.0.1:8779. Stop with Ctrl+C. Use `--port 8780` for another port
or `--state-dir PATH` to read an existing coordinator store on this host.

This development tool uses Python's standard library, plain HTML and JavaScript.
It polls every three seconds. It reads the catalog, registries and one atomic
state snapshot without locking or writing them. It never invokes the coordinator.
Missing state, invalid JSON or registry drift produce an unavailable/stale view,
not fabricated pending work. No coordinator state is initialized by the viewer.

Rows correspond to catalog plans; revisions remain separate from their baselines.
Counts cover required steps, with optional extensions counted separately. A plan
is complete when all required steps are complete; otherwise blockers take
precedence, followed by active/partially completed work, then pending. Dependencies
and resource conflicts are shown at step level. Heartbeats older than ten minutes
are warnings only. Reopened steps reduce current completion counts. Completion
evidence shown is for the current attempt; historical evidence stays in the
coordinator. Completion is not proof of deployment or human approval.

The server binds only to loopback and serves two fixed routes, not the checkout.
JSON uses an explicit field allowlist and excludes claim-token fields. Notes and
evidence are plain text; authors should not put secrets in free-text notes.
No remote hosting, runtime metrics, external assets or product dependencies.

Checks: `python -m unittest discover -s scripts -p test_spec_status_server.py`.
The tooling implementation plan is `implementation-plan.md`, registered separately
as `steps.json` using the existing shared coordinator state directory.
