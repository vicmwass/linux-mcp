# linux-mcp

A Rust [Model Context Protocol](https://modelcontextprotocol.io/) server that lets an LLM safely administer remote Linux servers over SSH. The LLM supplies Linux knowledge and intent; Rust is the sole enforcement boundary for validation, security policy, and execution.

> **Status:** early-stage / experimental. Architecture and command coverage are still evolving.

## Why

The goal is arbitrary, *safe* Linux command execution — not just package installs. The LLM never sees SSH credentials and never gets unrestricted shell access. Every request passes through a fixed pipeline before anything runs on a target host:

```text
MCP input
  -> capability lookup      (what does this distro/host support?)
  -> command parse/resolve  (normalize, resolve to a concrete command)
  -> execution analysis     (classify risk, detect destructive operations)
  -> security policy        (Allowed / RequiresConfirmation / Denied)
  -> execution plan
  -> execution engine
  -> SSH
```

A policy refusal (`Denied`) is a normal, successful outcome — not an error.

## Key modules

| Module | Responsibility |
|---|---|
| [`src/capabilities`](src/capabilities) | Distro/package-manager/init-system detection and caching |
| [`src/commands`](src/commands) | Command parsing, normalization, and resolution |
| [`src/execution`](src/execution) | Risk analysis, execution planning, limits, and the execution engine |
| [`src/security`](src/security) | Policy decisions (allow / confirm / deny) |
| [`src/ssh`](src/ssh) | Connection pooling and host-key verification |
| [`src/tools`](src/tools) | MCP tool surface exposed to the LLM |

See [linux-mcp-context.md](linux-mcp-context.md) and [.github/copilot-instructions.md](.github/copilot-instructions.md) for the full design contract.

## Requirements

- Rust (edition 2024 toolchain), or
- Docker + Docker Compose

## Setup

1. **Server inventory** — copy the example and fill in real values:
   ```sh
   cp config/servers.toml.example config/servers.toml
   ```
   Edit `hostname`, `username`, `port`, and `private_key` for each target server. This file is gitignored — never commit real values.

2. **SSH key** — place the private key referenced by `private_key` under `keys/` (also gitignored):
   ```sh
   keys/your-key
   ```

3. **Host-key trust store** — populate a `known_hosts` file the server will verify against. By default it looks at `~/.ssh/known_hosts`; override with:
   ```sh
   export LINUX_MCP_KNOWN_HOSTS=/path/to/known_hosts
   ```

4. **Run**:
   ```sh
   cargo build --release
   cargo run
   ```
   or with Docker:
   ```sh
   docker compose up --build
   ```

The server speaks MCP over stdio, so it's typically launched by an MCP-compatible client rather than run standalone.

## Security notes

- `keys/`, `.ssh/`, `ssh_trust/known_hosts`, and `config/servers.toml` are gitignored because they contain private keys, host trust data, and real server inventory. Only `config/servers.toml.example` is committed.
- Host keys are verified against a `known_hosts` file before any connection is trusted; unknown hosts are rejected, not silently trusted.
- Command execution always goes through the capability -> resolve -> analyze -> policy -> plan -> engine pipeline; nothing bypasses security policy on the intended path.

## License

MIT — see [LICENSE](LICENSE).
