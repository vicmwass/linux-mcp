# Linux Management Client — Build Plan

Date: 2026-10-11

Status: Proposed. This document describes planned work, not implemented features.

## Relationship to the Rust build specifications

This document is the product-experience roadmap. The [Rust specification pack](specs/README.md) translates it into independently buildable components, contracts and integration gates. Use its phase-to-spec mapping for implementation sequencing. The architecture decisions below incorporate the subsequent multi-user hosted and independent self-hosted requirements; all features remain proposed.

- Rust workspace alongside Linux MCP: Leptos dashboard, Axum backend, SQLx/PostgreSQL storage and the existing Rust execution foundation.
- Separate dashboard and MCP processes; both hosted and self-hosted editions have private per-user inventories.
- Authenticated HTTP MCP is the target transport for the multi-user product. Existing stdio/TOML mode remains a legacy/local adapter.
- Login, user isolation, approval and audit are foundation requirements, before remote mutations are enabled.
- Specs 00–07 deliver the foundation; 08 delivers the first console; 09–10 cover broader administration and monitoring; 11 covers guided workflows and optional agent extensions.

## Objective

Build an independent web application that provides a visual Linux management experience and uses the Linux MCP server for remote administration. Users should be able to inspect machines, manage supported resources, review proposed changes, and understand operation results without interpreting raw terminal output.

## Architecture and boundaries

```text
User / optional browser AI agent
              |
              v
Management dashboard
              |
              v
Application's own web server
  - Serves pages and application interactions
  - Authenticates users and checks permissions
  - Stores preferences, history, approvals and cached results
  - Runs an MCP client
              |
              | MCP: initialize, discover tools, call tools
              v
Linux MCP server
  - Validates operations and enforces execution policy
  - Holds server inventory and SSH/sudo credentials
              |
              | SSH
              v
Managed Linux machines
```

- The client application has its own web server and deployment lifecycle. Linux MCP does not serve its pages.
- Linux inspection and administration between the application backend and Linux MCP use MCP. Inventory, credential provisioning and approvals use authenticated management APIs; they do not execute Linux commands.
- Browser interactions with the application's own web server remain normal web application interactions.
- Use authenticated Streamable HTTP for the multi-user product and the independent dashboard's MCP client. Implement this explicitly; the current server's stdio transport does not provide a remote endpoint.
- Retain stdio as a legacy/local adapter, with the backend managing process lifecycle when that mode is selected. Do not use a shared unscoped stdio process as the multi-user authorization boundary.
- Pages remain available when MCP is disconnected. Mark cached data with its collection time, show the connection state, and disable operations that require a live connection.
- Keep stored SSH keys and sudo secrets on the Linux MCP side; never return them to the browser. A protected write-only provisioning form may submit a new secret, without retaining or echoing it.

## MCP integration design

1. Establish an MCP session and negotiate supported capabilities.
2. Discover tools and validate the required input and result contracts.
3. Map supported tools to dashboard features; show unsupported actions as unavailable.
4. Call tools with validated inputs and present structured results, policy refusals and execution errors distinctly.
5. Handle disconnection and session recovery without automatically replaying mutations.

Tool discovery identifies available functionality, but does not by itself define a usable interface. Agree on stable structured result schemas for services, metrics, processes and other resources. Prefer dedicated tools over parsing terminal output or accepting arbitrary shell text.

## Existing foundation

The server inspected during planning exposes inventory, capability detection, user inspection and mutation tools, plus classified command execution. Verify these contracts against the current implementation before building the adapters.

Existing user locking changes password lock state; it does not revoke SSH keys or terminate active sessions. Present that distinction clearly in the interface.

## User experience

- Persistent server selector and visible target machine on every operation.
- Overview page with connection state, system details, health indicators and recent activity.
- Navigation for Services, Logs, Users, Processes, Packages, Files, Storage, Network and Scheduled Jobs as capabilities are delivered.
- Searchable tables, useful filters, readable empty states and actionable error messages.
- Change previews showing the exact target, requested action, expected impact and approval requirement.
- Operation progress and final results, followed by verification where supported.
- Clear differentiation between successful, refused, failed and unknown-outcome operations.

## Feature roadmap

| Feature | User experience | Required additions |
|---|---|---|
| Server overview | Inventory, reachability, uptime, distribution and health | Structured health queries; collection timestamps |
| Resource monitoring | CPU, memory, disk and network trends | Metrics tools; periodic collection; historical storage |
| Service manager | Inspect services; start, stop and restart; view failures | Dedicated service query and mutation tools |
| Log viewer | Filter by service, severity and time; retrieve recent entries | Bounded log queries; pagination or incremental retrieval |
| Users and access | Inspect and manage accounts, groups, sudo access and SSH keys | Existing user adapters; additional access-management tools |
| Process explorer | Sort by resource use, inspect processes and request termination | Structured process tools; guarded termination |
| Package manager | List packages, inspect updates and preview installation/removal | Package tools; distro-aware behavior; guarded changes |
| File browser and editor | Browse allowed paths, inspect permissions and preview edits | Restricted file tools; size limits; validation; backup support |
| Storage manager | Inspect disks, mounts, filesystem use and large directories | Storage queries; separately guarded storage mutations |
| Network overview | Inspect interfaces, addresses, ports and firewall rules | Network tools; guarded firewall changes |
| Scheduled jobs | Inspect and manage cron entries and systemd timers | Dedicated scheduling tools |
| Audit history | Review requests, approvals, results and verification | Persistent audit records; sensitive-data redaction |

All additions are proposals. Features must reflect the tools and platform capabilities actually available on each machine.

## Guided workflows

- Troubleshooting: collect relevant evidence for a failed service, high resource usage or low disk space, then present proposed actions.
- Change verification: check the resulting service state or resource after a mutation and report the evidence.
- Saved maintenance operations: reusable workflows with explicit targets, bounded steps and approval rules.
- Fleet comparisons: compare failures, pending updates and configuration differences across selected machines.
- Alerts: application-owned monitoring schedules with meaningful notifications and configurable thresholds.

## Security and operation handling

- Authenticate application users and authorize access per server and action.
- Enforce permissions and approval checks in the application backend, while preserving Linux MCP policy enforcement.
- Bind approval to the authenticated user, exact target and operation parameters; expire it and prevent reuse where appropriate.
- A tool argument such as `confirm: true` is not evidence of human approval. Generate confirmation only after the application validates an approval record.
- Approval requirement is a per-server setting stored with the server configuration: "medium risk and above" (default) or "high risk and above". Only the signed-in person, in the dashboard, can change it; agents cannot, and a change invalidates pending approvals.
- Record who requested and approved an operation, its timestamps, outcome and verification, without storing secrets unnecessarily.
- Treat command output, logs and file contents as untrusted data, including when supplied to an AI assistant.
- After a timeout or disconnect, show an unknown outcome when execution may already have started. Reconcile using inspection before offering a retry.
- Do not promise rollback for arbitrary Linux changes. Offer backups or reversal only for operations with an implemented and tested recovery path.

## Optional WebMCP and AI support

WebMCP is a separate enhancement that exposes dashboard actions to compatible browser agents. It does not replace the application's MCP client connection to Linux MCP.

- Expose a curated set of dashboard actions through WebMCP when supported.
- Reuse the same backend permissions, approval flow and operation tracking as manual actions.
- Keep all core pages and controls usable without WebMCP.
- Verify the current WebMCP specification and browser support during implementation.
- An embedded chat assistant is a separate optional feature requiring a model integration and tool orchestration. The ordinary dashboard and deterministic MCP calls do not require an LLM.

## Delivery phases

### Phase 1 — Independent application and MCP foundation

- Set up the application's own page server, authentication and storage.
- Deliver private dynamic inventory, credential provisioning, host trust and authenticated HTTP MCP for hosted/self-hosted deployments (specs 00–07).
- Deliver approval and audit before enabling remote mutations; Phase 2 extends their presentation and verification.
- Implement MCP connection lifecycle, discovery and adapters for existing tools.
- Deliver server selection, capability details, user listing and operation results.
- Handle disconnected sessions, unavailable tools and policy refusals.

Acceptance: the application serves pages independently; all Linux interactions use MCP; credentials remain server-side.

### Phase 2 — First management console

- Add structured health, service and log tools to Linux MCP as needed.
- Deliver Overview, Services, Logs and Users pages.
- Extend foundation approval records and audit history with resource-specific previews and post-change verification (spec 08).

Acceptance: users can inspect a server, investigate a service failure and approve a supported change with a visible, verified outcome.

### Phase 3 — Broader administration

- Add Processes, Packages and Files, followed by Storage, Network and Scheduled Jobs.
- Introduce metrics history, bounded background collection and alerts.
- Apply capability checks and platform-specific handling throughout.

Acceptance: each feature has structured contracts, permission checks, bounded output and clear failure handling.

### Phase 4 — Guided and agent-assisted workflows

- Add saved maintenance workflows, fleet comparisons and troubleshooting guidance.
- Add optional WebMCP exposure and, separately, an embedded AI assistant if desired.

Acceptance: agent-driven and manual actions share the same authorization, approval and audit mechanisms.

## Implementation decisions to resolve

The spec pack fixes the proposed Rust workspace, separate processes, hosted/self-hosted modes, PostgreSQL and one owner per server for the first release. Remaining decisions are:

- Identity provider, supported agent clients and delegated MCP authorization flow.
- Exact structured tool schemas, distribution support and compatibility versions, frozen through spec 00 per milestone.
- Per-user concurrency/output limits, retention periods, collection budgets and monitoring frequency.
- Resource-specific approval policy and supported post-change verification.
- Optional WebMCP/browser support and model integration, evaluated only when those extensions are selected.

## Validation approach

- Check initialization, discovery, schema handling, disconnection and reconnect behavior against Linux MCP.
- Verify that unauthorized or unapproved operations cannot execute through either manual controls or agent tools.
- Check policy refusals, partial failures, timeouts and unknown outcomes in the interface.
- Validate structured tools on supported Linux distributions and capability combinations.
- Exercise mutations only on designated development targets, then independently verify their effects.
- Confirm the application still serves its pages when Linux MCP is stopped.

## Recommended starting scope

Build the independent application and MCP connection first, then deliver Overview, Services, Logs and Users. Extend the Linux MCP server with dedicated structured tools as each page requires them. Add broader administration and agent support after the core operation and approval flows work reliably.
