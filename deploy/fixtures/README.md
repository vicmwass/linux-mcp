# Disposable fixture topology

`compose.fixture.yaml` is a test-only profile. It uses an isolated Compose project name, private networks and a loopback-only dashboard port. It must never be selected by a production profile or receive production credentials. The nginx containers are protocol stubs for readiness, routing and isolation tests; they do not claim to implement identity, MCP or provider behavior.

## Start and seed

From the repository root:

```powershell
docker compose -f deploy/fixtures/compose.fixture.yaml up -d
docker compose -f deploy/fixtures/compose.fixture.yaml ps
```

Wait for every health check before running a test. Seed data is in `tests/integration/fixtures/seed.json`; it contains fake credentials only. The seed defines two installations, Alice and Bob, equal display labels with different server IDs, and two disposable target addresses. A future fixture adapter loads this data into the database/provider stubs rather than reading `config/servers.toml` or local SSH files.

## Isolation and faults

Only the dashboard stub is published, on `127.0.0.1:18765`. The database, identity, MCP and provider networks are internal. Tests must validate that Alice/Bob IDs and installations cannot cross those boundaries. Fault hooks are service-level operations: stop `fixture-mcp` for unavailable-provider tests, pause or restart `fixture-db` for audit/database failures, and use a fake provider adapter to inject `before_submit`, `after_submit` and `output_flood` cases. No fixture command may target a real host.

## Teardown

Run `docker compose -f deploy/fixtures/compose.fixture.yaml down --volumes --remove-orphans` and verify the project has no containers, networks or volumes. The teardown check must run even after a failed test. Never run this compose file against a production Compose project name.
