# linux-mcp project contract

## Architecture

- Keep `src/mcp/server.rs` thin. It orchestrates capability lookup, command resolution, analysis, policy, planning, and execution; it must not become a second policy or SSH implementation.
- Treat Rust as the enforcement boundary. Never expose private keys, raw connection objects, or unrestricted remote execution to MCP callers.
- The intended request path is: MCP input -> capabilities -> parse/resolve -> analyze -> security policy -> execution plan -> execution engine -> SSH.
- Unknown or ambiguous operations fail closed. A policy refusal is a successful policy decision (`Denied`), not a policy-system error.
- Keep target-specific behavior in capabilities and resolvers rather than duplicating distro checks in tools.

## Change discipline

- Before changing behavior, read the owning module and its nearest tests. Prefer the smallest change at that boundary.
- Preserve public APIs and existing async/concurrency patterns unless the task requires a contract change.
- Do not make compound shell parsing more permissive without corresponding parser, analyzer, policy, and security tests.
- Do not claim support for a command, interpreter, init system, or package manager unless detection, resolution, policy, and execution all agree.
- Run `cargo fmt --check`, `cargo check`, and the focused test command for the changed module when possible.

## Repository reality

- The checked-in tool modules are currently `src/tools/system.rs` and `src/tools/users.rs`; do not assume the larger target structure in `linux-mcp-context.md` already exists.
- `src/tools/system.rs` contains capability-shaped code in addition to the canonical `src/capabilities/` modules. Treat this as an integration surface and avoid creating a third capability implementation.
- SSH host-key verification uses `known_hosts`; changes to trust behavior require explicit security review.