---
name: linux-mcp-execution-trace
description: "Trace how a linux-mcp command moves from MCP input to remote SSH execution. Use when debugging command rejection, unexpected policy decisions, capability failures, timeouts, or incorrect remote execution."
---

# linux-mcp execution trace

Use a narrow trace instead of loading the whole repository.

## Read in order

1. `src/mcp/server.rs` for the tool entrypoint and response mapping.
2. `src/commands/parser.rs` and `src/commands/resolver.rs` for classification and capability validation.
3. `src/execution/analyzer.rs`, `src/security/policy.rs`, and `src/execution/plan.rs` for risk and authorization.
4. `src/execution/engine.rs`, `src/execution/limits.rs`, `src/ssh/manager.rs`, and `src/ssh/connection.rs` only if the request reaches execution.
5. `src/capabilities/detector.rs` and `src/capabilities/manager.rs` for target-specific failures.

## Diagnostic questions

- Did parsing produce `Unknown`?
- Did resolution reject a command absent from target capabilities?
- Did analysis raise the risk or mark the request destructive?
- Did policy return `Denied` or `RequiresConfirmation`?
- Did the plan reach the engine, and did the SSH timeout or remote exit code fail?

Record the first boundary where observed behavior diverges from the intended path. Fix that owner rather than adding compensating logic to the MCP tool.