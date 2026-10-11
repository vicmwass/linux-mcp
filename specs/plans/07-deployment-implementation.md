# Spec 07 — Deployment and integration implementation

Source: [07](../07-deployment-and-integration.md). Registry: `07-deployment.steps.json`. Follow [shared rules](IMPLEMENTATION-RULES.md). Own `deploy/`, `tests/integration/` and operations documentation. Checks: configuration rendering/validation, locked image builds, isolated Compose/provider/database tests, real-token MCP/browser tests and disposable Linux targets. This plan prepares reviewable deployment artifacts; publishing on paid/public infrastructure needs the user's deployment authorization.

## 07-01 — Define topology and integration test matrix

Dependencies: `00-contracts/00-12`. Resources: `deployment-design`.

1. Map separate dashboard/MCP processes, management API owner, database, provider, reverse proxy and secret injection boundaries.
2. Define hosted/self-hosted profiles, persistent volumes, network routes, UID permissions and test target requirements.
3. Turn Spec 07's matrix into named test cases and release evidence templates, initially fixture-backed.

Acceptance: every foundational release criterion has an owner/test; topology contains no global user impersonation or local-machine bind assumption.

## 07-02 — Build fixture topology and harness

Dependencies: `07-01`. Resources: `deployment-harness`.

1. Add isolated network/database/provider setup and mock service images for early contract tests.
2. Build Alice/Bob and two-installation test seeds with fake credentials and disposable SSH targets.
3. Define readiness waits, teardown target validation and fault injection without production access.

Acceptance: reproducible fixture environment starts/stops safely; fixtures are visibly separate from production configuration.

## 07-03 — Build production images and configuration

Dependencies: `07-01`, `02-identity/02-08`, `04-mcp/04-08`, `05-dashboard/05-09`. Resources: `deploy-manifests`.

1. Produce locked multi-stage dashboard/MCP images with nonroot runtime and compiled browser assets.
2. Compose real database/provider services, migrations, probes and externally supplied config/master keys.
3. Validate image contents, permissions, no baked secrets and no dependency on saved-machine reachability for readiness.

Acceptance: clean image builds and config validation pass; no test adapter can activate in production profile.

## 07-04 — Implement bootstrap, HTTPS and network controls

Dependencies: `07-03`. Resources: `deploy-manifests`, `deployment-bootstrap`.

1. Configure domain/issuer/resource URLs, reverse proxy TLS, secure cookies and HTTP origin constraints.
2. Implement first-admin bootstrap, startup migration ordering and operational limits/metrics.
3. Document private-network/VPN routes, outbound policy and verified host-key enrollment.

Acceptance: a clean local isolated installation initializes once and connects through the test HTTPS topology; restrictive defaults hold.

## 07-05 — Run foundation isolation and real-client matrix

Dependencies: `07-02`, `07-04`. Resources: `foundation-integration-tests`.

1. Run two-user browser/direct-MCP tests for inventory, credentials, caches, audit and session IDs.
2. Test real OAuth client, human approval, policy refusal, key mismatch and audit outage against disposable targets.
3. Independently verify safe mutation effects and record supported client/distribution versions.

Acceptance: all real-adapter read/execution gates pass; foreign IDs fail before network access and no confirmation-only bypass exists.

## 07-06 — Validate runtime lifecycle and fault handling

Dependencies: `07-05`. Resources: `foundation-fault-tests`.

1. Change/delete targets and rotate credentials without restarting; verify new requests use current revisions.
2. Revoke grants and inject transport failure before/after command submission and service outages.
3. Verify unknown outcomes, no replay, bounded resources and dashboard availability without MCP.

Acceptance: all lifecycle gates pass; cancellation/revocation does not claim guaranteed termination of already submitted remote commands.

## 07-07 — Implement and verify backup, restore and upgrades

Dependencies: `07-04`. Resources: `deployment-recovery`.

1. Document protected database/master-key backup and restore procedures plus compatibility/migration rules.
2. Restore to isolated installation and test ownership/decryption, wrong/missing keys and independent issuer trust.
3. Rehearse supported rolling upgrade or document required downtime; never imply unsupported schema downgrade.

Acceptance: backup restoration works with required keys and fails safely without them; two independent installations share no trust/data by accident.

## 07-08 — Publish M1 release evidence and self-hosting guide

Dependencies: `07-06`, `07-07`. Resources: `deploy-manifests`, `deployment-release`.

1. Run final image/config/root regression checks and trace all 00–07 acceptance gates to actual results.
2. Publish install/update/rotation/recovery guides, supported topology and limits in `specs/handoffs/07.md`.
3. Provide concrete reviewable deployment bundle; distinguish local verification from any external publish.

Acceptance: M1 can be installed independently with persistent state; later console/optional-agent modules are not required. No public deployment occurs without authorization.
