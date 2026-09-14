---
name: Rust architecture boundaries
description: "Apply when editing Rust modules in linux-mcp, especially MCP tools, command resolution, execution, security, capabilities, or SSH."
applyTo: "src/**/*.rs"
---

# Rust architecture boundaries

- Identify the module that makes the decision before editing a forwarding or orchestration caller.
- Preserve the boundary between parsing (`src/commands`), capability detection (`src/capabilities`), authorization (`src/security`), approved plans and limits (`src/execution`), and transport (`src/ssh`).
- Keep raw LLM input out of the SSH layer. The SSH layer receives an approved, bounded request through the execution path.
- Use structured errors and results where callers need to distinguish detection failure, rejection, confirmation, timeout, and remote command failure.
- Add or update a focused unit test for classification, policy, capability parsing, or limit behavior when changing that behavior.