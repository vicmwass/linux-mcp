# Current server surface (audit baseline, 2026-10-11)

Source of truth: `src/mcp/server.rs`, `src/tools/*.rs`, `src/config/servers.rs`, `src/execution/*`, `src/security/policy.rs`, `src/ssh/*` at commit `31fb043` plus the uncommitted working tree observed by step 00-01. This records **current behavior**, not proposed extensions. Proposed extensions are listed separately at the end and in [decisions.md](decisions.md).

## Process and transport

- Single binary `linux-mcp` (Rust edition 2024, rmcp 3.2.0, `server` + `transport-io`). JSON-RPC over stdio only; `ServerHandler` uses rmcp defaults (protocol version `2025-11-25` is rmcp's `LATEST`).
- Inventory is the static file `config/servers.toml` loaded once at startup. There is no owner, revision, enabled flag or host-trust status; any MCP client that can reach stdio can use every configured server.
- Server IDs are free-form strings chosen by the operator (for example `ubuntu-01`), not UUIDs.
- Host keys are checked against `LINUX_MCP_KNOWN_HOSTS` or `$HOME/.ssh/known_hosts`; unknown keys are rejected. SSH authenticates with the configured private-key **file path**. Sudo optionally reads a password file path.

## Tool inputs (exact, from `#[derive(Deserialize, JsonSchema)]` structs)

| Tool | Parameters | Notes |
|---|---|---|
| `list_servers` | none | |
| `get_server_capabilities` | `server_id: string` | |
| `list_users` | `server_id: string` | |
| `get_user` | `server_id: string`, `username: string` | |
| `create_user` | `server_id: string`, `username: string`, `home?: string\|null`, `shell?: string\|null` | username is lowercased by the handler |
| `disable_user` | `server_id: string`, `username: string` | `usermod -L` (password lock only) |
| `enable_user` | `server_id: string`, `username: string` | `usermod -U` |
| `delete_user` | `server_id: string`, `username: string`, `confirm: bool` (required), `remove_home: bool` (required) | refuses unless `confirm` |
| `execute_command` | `server_id: string`, `command: string`, `confirm?: bool` (default `false`) | classified command string |

Unknown fields are not rejected (no `deny_unknown_fields`).

## Tool results (exact)

Every handler returns `String`. rmcp converts it with `CallToolResult::success(text)`, so **every** result — success, policy refusal, validation error, SSH failure and timeout — is a single text content item with `isError` absent/false. There is no structured content and no output schema. Protocol-level JSON-RPC errors only arise from rmcp itself (unknown tool, argument deserialization failure).

| Tool | Success text | Failure text prefix |
|---|---|---|
| `list_servers` | blocks `ID: …\nHost: …\nPort: …\nUser: …\nEnvironment: …` joined by blank lines; or `No Linux servers are configured.` | n/a |
| `get_server_capabilities` | `Server:`, `Distribution:`, `Version:`, `Kernel:`, `Architecture:`, `Init system: {Debug}`, `Package manager: {Debug}`, `Available commands:` indented list | `Failed to detect server capabilities:\n…` |
| `list_users` | blocks `Username/UID/GID/Home/Shell`; or `No users found.` | `Failed to list users:\n…` |
| `get_user` | one `Username/UID/GID/Home/Shell` block | `Failed to get user:\n…` |
| `create_user` | `User '<u>' created successfully.` | `Failed to create user:\n…` |
| `disable_user` / `enable_user` | `User '<u>' has been disabled/enabled successfully.` | `Failed to disable/enable user:\n…` |
| `delete_user` | `User '<u>' has been permanently deleted[ along with their home directory].` | `Failed to delete user:\n…` (includes the missing-confirmation refusal) |
| `execute_command` | `Exit code: N\nSTDOUT:\n…\nSTDERR:\n…` | `Failed to detect server capabilities:`, `Command rejected:`, `Security policy rejected the command:`, `This operation requires explicit confirmation.`, `This operation is not permitted.`, `Command execution failed:` |

Consequences for contracts: an adapter must classify these strings to map them to typed outcomes; policy refusal is distinguishable from transport failure only by prefix. A non-zero exit code is reported as a normal result. Timeout text says the remote outcome "may be unknown", but the type system does not carry that.

## Internal execution and policy types

- `Operation` (23 variants, snake_case `Display`), `RiskLevel {Low, Medium, High, Critical}`; `Operation::requires_sudo` is true for everything except read-only/network-read/file-read.
- `SecurityPolicy::evaluate(analysis, capabilities, confirmed, command, is_root) -> Result<PolicyDecision {Allowed, RequiresConfirmation, Denied}>`. Catastrophic deletes, unknown and critical operations are denied; high risk and sudo require `confirmed`. `confirm` is the agent's own boolean, not human approval.
- `ExecutionEngine::execute_structured` returns `ExecutionResult { server_id, success, stdout, stderr, exit_code: u32, operations, risk }`. `ExecutionLimits` declares 1 MiB stdout/stderr bounds and 60 s timeout; only the timeout is enforced on this path (output bytes are not truncated).
- `ExecutionRequest`/`ExecutionPlan`/`ExecutionPayload::Script` exist but are unused by the request path.
- `SshManager::execute_with_stdin` retries once only when the cached connection was closed or a channel failed to open (before exec); it never replays after exec submission. Timeout evicts the connection.

## Existing tests

`cargo test --locked --offline` on 2026-10-11: 51 passed, 0 failed; 23 pre-existing warnings (unused/dead code). No integration or live SSH tests run in CI.

## Proposed extensions (not implemented)

- Owned, revisioned inventory with UUID `ServerId` and host-trust state (Spec 01).
- Trusted `Principal` per request (Spec 02).
- Structured results with typed outcomes, approval-required refusals and versioned prepare/submit flow (Specs 00/04/06).
- Bounded output with truncation flags (Spec 03).
- `confirm: true` stays an expression of agent intent; it is never approval evidence on the remote service. Legacy stdio keeps today's confirmation semantics unchanged.
