# 07 — Deployment and integration

Implementation: [step-by-step plan and dependencies](plans/07-deployment-implementation.md). Follow the [coordination workflow](plans/README.md).

## Deliverable and ownership

Own `deploy`, integration harnesses and installation/operations documentation. Produce images for the independent Rust dashboard and MCP service, plus a Compose bundle with PostgreSQL and the selected identity provider. Start against mock images, then replace them at each integration gate.

## Deployment requirements

- Hosted and self-hosted use the same application contracts. Each self-hosted installation has independent database, domain/issuer configuration, credential encryption keys and persistent storage; no central hosted-service dependency is required for core use.
- Database and credential material persist outside application images. Supply external configuration, secret injection, UID permissions, HTTPS/reverse-proxy configuration and first-admin bootstrap instructions.
- Provide deterministic locked builds, explicit migrations, schema compatibility checks and readiness/liveness probes. Readiness never needs a successful connection to every saved machine.
- Document outbound SSH routing, private network/VPN requirements, inbound MCP/dashboard routing, allowed destinations and verified host-key enrollment.
- Back up database plus required encryption keys using separately protected storage. Test restore into an isolated installation. Never claim a database-only backup can recover encrypted credentials without keys.
- Inventory/credential edits require no service restart. Provider/domain/master-key configuration changes may require a controlled restart or explicit rotation procedure.
- Define rolling-upgrade support, migration ordering and recovery steps; do not promise arbitrary downgrade across destructive schema changes.
- Include per-user resource limits, redacted operational metrics, dependency failure behavior and audit retention settings.

## Integration matrix

| Scenario | Required result |
|---|---|
| Alice and Bob create machines with identical names | Separate inventories, credentials, caches and audit |
| Alice guesses Bob's IDs through every tool/API | No foreign data or execution |
| Agent connects using installation URL | OAuth identity controls visible/usable servers |
| User changes inventory or rotates credential | New requests use current revision without restart |
| User deletes machine or revokes agent grant | New access blocked; running outcome reported honestly |
| Host key changes | Connection blocked until explicit reverification |
| Agent sets confirm without human approval | Approval-required; no privileged mutation |
| Disconnect occurs after submission | Unknown outcome, no automatic replay |
| Audit storage fails | New mutations fail before dispatch |
| Two self-hosted installations | Independent data, keys and token trust |
| Dashboard runs while MCP is stopped | Pages available, live operations unavailable |
| Upgrade and backup/restore | Ownership and decryptability preserved |

## Completion criteria

Run existing Rust regression tests, component contract suites, real HTTP/OAuth/MCP integration tests, browser workflows and disposable SSH target tests. Test both ordinary inspection and guarded mutations; independently verify remote effects. Record supported clients, distributions, deployment topology and known limitations. Never execute lifecycle tests against production targets.

Release only after all README gates pass with real adapters. Update the architecture explanation to describe implemented behavior at that time; these specs remain proposals until delivered.

## Incremental console release gates

M1 covers specs 00–07 only. M2 adds 08's structured health/services/logs and user pages with a real investigation, approved action and verified state. M3 adds each selected 09 module independently and 10's authorized collection/history tests. M4 adds selected 11 packages, including resumability, partial failures and agent/manual permission parity. Optional AI/WebMCP are never prerequisites for a core self-hosted installation.

For every new resource tool, run schema compatibility, supported-distribution, ownership, approval, bounded-output and unknown-outcome checks through both direct MCP and the dashboard client. Render malicious log/file fixtures safely. Record supported tools/platforms per release and ensure unavailable capabilities are disabled in the UI.
