# 05 — Independent Rust dashboard

Implementation: [step-by-step plan and dependencies](plans/05-dashboard-implementation.md). Follow the [coordination workflow](plans/README.md).

## Deliverable and ownership

Own `crates/dashboard`: Leptos UI, its Rust web backend, management API client and authenticated MCP client. It has its own process and remains available when MCP is disconnected. Develop against HTTP/MCP fixture servers from 00.

## Initial screens and workflows

1. Login/logout and account connection status.
2. Owned server list and add/edit/delete forms: hostname, port, username, environment, approval requirement, credential provisioning, trust enrollment and revision-conflict recovery. The approval requirement is a two-option setting, "Medium risk and above" (default) or "High risk and above", shown with plain-language examples of what runs without approval under each, a warning when choosing the weaker option, and the note that changing it invalidates pending approvals. It is changed only in the interactive session and is never shown as editable to agents.
3. Agent setup: installation MCP URL, authorization instructions, grant status and revocation. Never display stored credentials in setup snippets.
4. Server detail: capabilities and existing supported user inspection/actions; classified command interface with visible target and bounded results.
5. Exact-operation approval screen and owned audit history, including refusal, failure and unknown outcome.

Inventory management uses 01's API. All Linux inspection and administration uses MCP through the backend, preserving the existing independent-client design. Browser code never opens SSH connections or receives stored secrets. Users may submit new secrets through a protected write-only form; clear the form after provisioning and never include values in hydration, analytics or error reports.

## Requirements and acceptance

- No handwritten JavaScript/TypeScript application logic is required; Rust compiles browser interactions to WebAssembly. HTML/CSS and generated tooling assets remain normal build artifacts.
- Backend enforces authorization independently of UI visibility. User-scoped caches are cleared on logout/account changes.
- Capability/tool discovery determines enabled features. Do not advertise unimplemented service/log/file tools.
- Show target, exact action, risk and expiry before approval, plus the server's approval requirement in effect. Match approval parameters to the execution intent.
- Distinguish fresh/cached data, disconnected state, policy refusal and unknown remote outcome. Never offer blind automatic retry for mutations.
- Keyboard-accessible forms, responsive layouts, useful validation and empty states.
- Two simultaneous user sessions expose no cross-user data. Dashboard pages remain usable during MCP outage, with live operations disabled. Tests cover server creation, trust enrollment, inspection, approval and revocation using fixture services.

## Management-client roadmap integration

[Spec 12](12-client-page-design.md) independently produces mockups, visual tokens and component specifications. Its initial published handoff is an enforced prerequisite for 05-02's page shell and shared Leptos components. Backend architecture can proceed independently. Page agents pin the baseline version they consume and follow tracked review/adoption work for later revisions.

This spec owns the shared page shell, persistent server selector, visible target, navigation, search/filter patterns, timestamps, connection state and operation-result presentation. Resource pages from 08/09 plug into that shell; unsupported modules remain unavailable rather than becoming raw-command fallbacks. Monitoring views from 10 and workflow views from 11 reuse these components.

The dashboard backend validates required tool input and structured result schemas after discovery. Unknown schema versions disable affected actions with a useful explanation. Policy refusal, remote execution failure and unknown outcome remain distinct. Render remote output as untrusted text; never execute HTML or instructions obtained from logs/files. Cache entries include owner, server revision and collection time.

Post-change verification shows the observed resource state and timestamp separately from command success. It does not claim rollback or proof of success where no reliable inspection exists. All core pages remain usable without WebMCP or an embedded model.
