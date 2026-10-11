# 04 — Authenticated remote MCP service

Implementation: [step-by-step plan and dependencies](plans/04-mcp-implementation.md). Follow the [coordination workflow](plans/README.md).

## Deliverable and ownership

Own `crates/mcp-service`: HTTP transport, session lifecycle, MCP tool adapters and runtime composition. Reuse rmcp where supported; verify its selected version's transport/auth integration during implementation. Build with mock identity, inventory, executor and governance services.

## Requirements

- Expose an authenticated Streamable HTTP endpoint `/mcp`, appropriate authorization discovery and HTTPS deployment support. No credentials embedded in endpoint URLs.
- Bind every request to a verified Principal. Tool discovery can be common; `list_servers` returns only owned inventory. Every tool independently authorizes its server argument.
- Adapt the existing inventory, capabilities, user and classified-execution tools. Preserve policy refusals as distinct from transport failure. Do not expose arbitrary credential lookup or unrestricted raw SSH tools.
- Remote mutations requiring human approval use 06's prepare/approve/execute flow. Keep compatibility decisions explicit: add operation/approval references through versioned schemas or separate prepare/submit tools. Never silently reinterpret existing confirmation guarantees.
- Existing `confirm: true` may express intent but cannot create a human approval. Direct agents receive an approval-required result and dashboard link when applicable. Whether a given operation is approval-required depends on the target server's `approval_mode` (06/03); agents cannot read it as a way to skip approval or change it through MCP.
- Recheck ownership, approval and current revision at submission. Emit durable audit records and enforce per-user limits. Never automatically replay a mutation after reconnect.
- Protect HTTP origins, session handling and request sizes according to the verified MCP transport requirements. Session identifiers grant no authority.

## Acceptance

Initialize, discovery, calls, reconnection and authorization work with at least one specified real remote MCP client. Alice cannot execute against Bob's server even by bypassing discovery. Tokens and sessions cannot cross identities. Revocation, unknown outcomes and unavailable dependencies return stable contract results. A runnable mock-backed MCP service allows dashboard work before real SSH integration.
