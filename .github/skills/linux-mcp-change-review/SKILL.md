---
name: linux-mcp-change-review
description: "Review a linux-mcp change for architecture, security, and regression risk. Use when reviewing a PR, checking a proposed fix, or validating changes to commands, execution, policy, capabilities, SSH, or MCP tools."
---

# linux-mcp change review

Review only the affected slice first. Load `linux-mcp-context.md` only when the local code does not answer a question.

## Workflow

1. Identify changed files and map each to its owner: parser/resolver, analyzer, policy, plan/engine, capabilities, SSH/config, or MCP tool.
2. Trace the request through `src/mcp/server.rs` and check that validation occurs before execution.
3. Check fail-closed behavior for unknown operations, missing capabilities, missing host keys, timeouts, and policy denials.
4. Check that errors do not leak keys, command secrets, or unnecessary host details.
5. Run the narrowest relevant tests, then `cargo check` and `cargo fmt --check` if the environment permits.

## Report

List findings first, ordered by severity, with file and line references. Separate confirmed defects from open questions and test gaps. Do not propose unrelated refactors.