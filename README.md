# linux-mcp

A Rust [Model Context Protocol](https://modelcontextprotocol.io/) server that lets an LLM safely administer remote Linux servers over SSH. The LLM supplies Linux knowledge and intent; Rust is the sole enforcement boundary for validation, security policy, and execution.

> **Status:** early-stage / experimental. Architecture and command coverage are still evolving.

## Why

The goal is structured Linux administration — not just package installs. Credentials are loaded by the server rather than supplied in MCP calls. The generic execution tool follows this path:

```text
MCP input
  -> input validation and argument quoting
  -> command parsing       (classify the operation)
  -> execution analysis     (classify risk, detect destructive operations)
  -> security policy        (Allowed / RequiresConfirmation / Denied)
  -> execution engine
  -> SSH
```

A policy refusal (`Denied`) is a normal, successful outcome — not an error.

Capability detection is exposed separately. The structured execution path does not currently use the older resolver or execution-plan modules. Dedicated user-management tools apply their own account validation and deletion safeguards.

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

## Password-based sudo

Add an optional `sudo_password_file` to each server inventory entry and mount that file read-only at the same container path. For example:

```toml
sudo_password_file = "/run/secrets/ubuntu-01-sudo"
```

`execute_command` uses `executable`, `args`, and `privilege: "sudo"` with `confirm: true`. User creation, deletion, locking and unlocking share the same sudo implementation. SSH login still uses the configured SSH key; this feature does not provide password authentication for SSH or nested SSH commands.

The server sends the password through SSH stdin to `sudo -k -S`, with a shell wrapper that gives the target command `/dev/null` as stdin. Passwords are not command arguments. The wrapper requires appropriate sudo permission and supports noninteractive commands without a TTY. Missing or invalid configured secrets fail closed. Omitting `sudo_password_file` selects `sudo -n` for existing passwordless sudo configurations.

Compose accepts `LINUX_MCP_MINT_SUDO_FILE` and `LINUX_MCP_UBUNTU_SUDO_FILE` overrides for the local password-file paths. Populate `ssh_trust/known_hosts` with verified host keys. For Docker MCP Gateway, update the profile snapshot as well as the catalog, and allow the project plus the exact secret paths in `MCP_GATEWAY_DOCKER_BIND_ALLOWED_PATHS` (semicolon-separated on Windows). Reconnect MCP clients after rebuilding the image or changing launcher configuration.

Validation commands:

```sh
cargo test --locked
python scripts/verify_sudo.py --connection linux-mcp
python scripts/verify_sudo.py --connection MCP_DOCKER
python scripts/verify_sudo_failures.py
```

The Python checks use the configured Codex stdio launchers, Docker and the existing server inventory. The failure checks mount an isolated inventory with synthetic secrets. An optional mutation check, `python scripts/verify_user_lifecycle.py --server ubuntu-01 --connection MCP_DOCKER`, creates and removes one temporary account without a usable password or home directory.

## Security notes

- `keys/`, `.ssh/`, `ssh_trust/known_hosts`, and `config/servers.toml` are gitignored because they contain private keys, host trust data, and real server inventory. Only `config/servers.toml.example` is committed.
- Host keys are verified against a `known_hosts` file before any connection is trusted; unknown hosts are rejected, not silently trusted.
- Structured execution validates input, classifies risk and enforces confirmation/refusal before execution. The classifier is not an OS sandbox for arbitrary interpreters.
- Closed SSH sessions reconnect before command submission. Commands are never automatically replayed after submission; a timeout or disconnect may leave their remote outcome unknown.

## License

MIT — see [LICENSE](LICENSE).

## Claude Code Desktop

The Code tab reads user-level MCP settings from `~/.claude.json`, shared with Claude Code CLI. The `linux-mcp` entry launches `linux-mcp:local` with read-only inventory, SSH key, host trust and sudo secret mounts. Restart Claude Code Desktop or start a fresh Code session to load it. The Chat tab uses the separate `claude_desktop` Docker profile.
