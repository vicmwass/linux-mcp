# Legacy (current) MCP fixtures

Provenance: captured on 2026-10-11 by running the existing `linux-mcp` binary (commit `31fb043` plus uncommitted tree, rmcp 3.2.0, stdio) and exchanging real JSON-RPC messages: `initialize` (client requested protocol `2025-06-18`, which the server echoed), `tools/list`, and `tools/call` against a **nonexistent** server ID so no SSH connection was attempted. No configured server data appears. Schema version label: `linux-mcp/legacy-1`.

These files are verbatim server output, not hand-written. Observed behavior worth knowing:

- Policy refusals, validation errors, missing servers and SSH failures all arrive as successful results (`isError: false`) with human-readable text. Only argument deserialization failure sets `isError: true` and an unknown tool is a JSON-RPC `-32602` error.
- There is no structured content and no output schema.
- `list_servers` takes no arguments and its `inputSchema` has no `$schema`/`required`.

Positive-path result texts (`list_servers`, `list_users`, `get_user`, `create_user`, …, successful `execute_command`) need live SSH targets and are documented from source in `specs/contracts/current-surface.md`, not captured here.
