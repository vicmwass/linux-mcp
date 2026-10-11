# Rust multi-user Linux MCP specification pack

Date: 2026-10-11. Status: proposed; none of these documents imply implemented support.

## Agent implementation plans

All specs now have [step-by-step implementation plans](plans/README.md), totaling 141 registered steps with local and cross-spec prerequisites. The coordination workflow and `scripts/plan_coordinator.py` provide atomic local claims, resource reservations, progress, blockers, completion evidence and review/reopen attempts for every step. Agents must read the instructions before editing, claim an eligible step, complete its checks, record evidence and then claim the next eligible authorized step. Product progress is tracked separately in live state; planning changes neither reset nor complete another agent's work.

## Product scope

Deliver an all-Rust dashboard and MCP service in two deployment modes: a shared hosted installation with private per-user inventories, and independently self-hosted installations with their own users, database, secrets and domain. Adding, editing and removing a machine takes effect without rebuilding or restarting the service.

The dashboard is an independent web application. Its backend manages inventory through the management API and performs Linux administration through an authenticated MCP client. External agents connect directly to the same MCP service. Browser code never receives stored SSH or sudo credentials. Leptos is the proposed UI, Axum the HTTP framework, SQLx the database layer, and the existing Rust execution engine the foundation. Dependency versions are selected and verified during implementation.

Foundation scope: login, private inventory, credential provisioning, host-key enrollment, MCP connection setup, capability discovery, existing user tools, classified command execution, approval and audit. The first management-console milestone adds Overview, Services, Logs and Users through spec 08. Broader administration, monitoring and guided workflows have separate delivery specs 09–11. Embedded AI and WebMCP remain optional, separately gated enhancements.

## Build specifications

| Spec | Independently delivered component | Dependencies for real integration |
|---|---|---|
| [00](00-contracts.md) | Shared Rust contracts, schemas and mocks | None; complete first |
| [01](01-inventory-and-secrets.md) | Persistent inventory and encrypted credentials | 00, auth context from 02 |
| [02](02-identity-and-authorization.md) | Dashboard identity and MCP authorization | 00 |
| [03](03-execution-core.md) | Tenant-aware Rust execution library | 00, adapters from 01 |
| [04](04-mcp-service.md) | Remote MCP endpoint and tool adapters | 00, 01, 02, 03, 06 |
| [05](05-dashboard.md) | Independent Rust dashboard and MCP client | 00, management API from 01, 02, 04, 06 |
| [06](06-approvals-and-audit.md) | Approval and audit services | 00, identity from 02 |
| [07](07-deployment-and-integration.md) | Hosted/self-hosted packaging and integration suite | All components |
| [08](08-first-management-console.md) | Structured health, service and log tools plus console pages | 00, 03–06; real identity/inventory for release |
| [09](09-administration-modules.md) | Independent process, package, file, storage, network, scheduling and access modules | 00, 03–06; module-specific contracts |
| [10](10-monitoring-and-alerts.md) | Metrics collection, history and alerts | 00–04, 06; metrics contracts |
| [11](11-workflows-and-agent-extensions.md) | Guided workflows, fleet comparisons and optional agent extensions | 00, 04–06; relevant tools from 08–10 |
| [12](12-client-page-design.md) | Independent visual system, screen mockups, interaction states and versioned design handoff | None to start; published baseline gates 05's page shell |

After 00 is frozen, 01–06 can be built in parallel using contract-owned mocks. 07 can start packaging and test infrastructure immediately, but release acceptance waits for real integrations. Parallel builds do not mean simultaneous access to shared implementation files: each component owns its crate and migrations.

Spec 12 starts independently now. It designs every planned page family using synthetic data; backend work can proceed alongside it. Production page implementation consumes its published baseline. All specs/steps support review and rework: completed evidence records one attempt rather than permanent approval, and reopening requires dependent acceptance to be revalidated.

Specs 08–11 are incremental delivery packages, not prerequisites for the foundation release. Each freezes its additional contracts through 00 before parallel server, client and test work begins. Their acceptance gates extend 07 only for the milestone being released.

## Integration with the management-client plan

[`LINUX-MANAGEMENT-CLIENT-PLAN.md`](../LINUX-MANAGEMENT-CLIENT-PLAN.md) defines the product experience and feature roadmap. This pack defines work ownership, interfaces, dependencies and acceptance. The following mapping is the agreed planning baseline; neither document claims these features exist today.

| Client-plan phase | Delivery milestone | Build specs and acceptance |
|---|---|---|
| Phase 1: independent client and MCP foundation | M1: secure multi-user foundation | 00–07; isolated inventories, existing tools, approval/audit, hosted and self-hosted smoke tests |
| Phase 2: first management console | M2: Overview, Services, Logs, Users | 08 with 03–06 adapters; investigate a service failure, approve a supported change and verify the result |
| Phase 3: broader administration | M3: incremental resource modules and monitoring | 09 modules plus 10; release each module independently with structured contracts and isolation tests |
| Phase 4: guided and agent-assisted workflows | M4: bounded workflows and optional agent interfaces | 11; manual and agent paths share authorization, approval and audit |

The newer multi-user requirement moves OAuth/HTTP, durable approvals and audit into M1; they cannot wait for a later console milestone. The independent dashboard process, MCP-only Linux administration, capability-driven UI, disconnected pages and unknown-outcome handling remain requirements from the client plan. Inventory, credential provisioning and approvals use management APIs; those APIs must not become a second Linux-command execution path.

## Parallel delivery after the foundation

- M2: health, services and logs tool modules can develop separately, with page modules developed against schema fixtures. Users reuse existing tools; new access features belong to 09.
- M3: process/package/file modules can run independently; storage/network/scheduled-job modules follow their own contract gates. Monitoring can run alongside them after metrics schemas are frozen.
- M4: troubleshooting and fleet comparison use read-only fixtures first. Saved mutation workflows integrate only after per-step approval and outcome tracking pass.
- Shared routing, migrations and contract files have one integration owner. Feature teams own module files and propose central registration changes to that owner.

## Proposed workspace

```text
crates/contracts/       # 00: DTOs, service traits, errors, mock fixtures
crates/inventory/       # 01: repositories, secrets, management API
crates/identity/        # 02: sessions, token verification, authorization
crates/execution-core/  # 03: existing validation, policy, SSH
crates/mcp-service/     # 04: MCP transport and tool handlers
crates/dashboard/       # 05: Leptos server/browser code, MCP client
crates/governance/      # 06: approval and audit
crates/resource-tools/  # 08/09: independently owned Linux resource modules
crates/monitoring/      # 10: collection, history, alerts
crates/workflows/       # 11: deterministic workflow orchestration
deploy/                # 07: images, Compose, operations documentation
tests/integration/     # 07: end-to-end and isolation tests
```

Dependency direction: components depend on `contracts`; contracts depend on none of them. Runtime binaries compose implementations. Shared database access is through owner-provided services, never another component's tables. Browser builds use only public DTOs, excluding server-side secret and identity internals.

## Integration gates

1. **Contract gate:** agree schemas, auth model, trait signatures, migrations ownership, mock fixtures and error semantics; compile all component skeletons with mocks.
2. **Read gate:** two users log in; each sees only their inventory through both dashboard and MCP; cross-user guessed IDs fail. No mutation required.
3. **Execution gate:** credential provisioning, verified host trust, real SSH inspection, policy refusal, approvals and audit work against disposable targets.
4. **Lifecycle gate:** inventory edits, credential rotation, deletion, token revocation and disconnects invalidate relevant state without replaying mutations.
5. **Release gate:** clean self-hosted installation, separate-installation isolation, backup/restore and hosted deployment checks pass.

Each component supplies unit tests, its contract tests, a runnable demo with mocks, setup instructions and a short integration example. The assembled system must pass tests with real adapters; mocks alone are not release evidence. Each later feature also passes structured-output compatibility, capability/platform checks, tenant isolation, bounded-output handling and post-change verification where available.

## Decisions fixed for the first release

- PostgreSQL for both hosted and self-hosted deployments, avoiding two database implementations initially. SQLite is a later optional adapter.
- One owner per server; no teams or shared machine grants initially.
- Separate dashboard and MCP processes, packaged together for self-hosting.
- Standards-based OAuth for remote MCP; browser sessions are distinct from agent credentials.
- Authenticated endpoint URL is shared within an installation; possession of a URL grants no access.
- Local encrypted credential storage is the initial adapter; external secret managers are an extension.
- Preserve existing stdio/TOML mode as a separate legacy single-user adapter during migration.
- Human approval for remote mutations is per server (`approval_mode`): `medium_and_above` by default, `high_and_above` as an explicit owner setting stored with the server configuration. Only an interactive browser session can set it; agents cannot. Approved by the product owner on 2026-10-11 (see [decisions D9/D11](contracts/decisions.md)).

Before coding 02/04, choose the supported identity provider and verify the current MCP authorization requirements and target client compatibility. Before freezing 00, approve the stronger human-approval semantics for remote mutations; existing `confirm` arguments alone are not human approval evidence.
