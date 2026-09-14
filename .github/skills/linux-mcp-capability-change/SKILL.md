---
name: linux-mcp-capability-change
description: "Add or change Linux command, package-manager, init-system, or server capability support in linux-mcp. Use when extending detection, parsing, resolver validation, or target support."
---

# linux-mcp capability change

Keep capability support coherent across the complete path.

## Workflow

1. Read the relevant enum/model in `src/commands/operation.rs` or `src/capabilities/models.rs`.
2. Update remote detection and output parsing in `src/capabilities/detector.rs`.
3. Update cached access only when the capability shape or invalidation semantics change.
4. Update parser classification and resolver validation together.
5. Confirm analyzer risk, sudo, network, and destructive metadata match the operation.
6. Confirm security policy allowlist, confirmation, and critical-operation behavior.
7. Add focused tests for detection/parsing and command resolution, including unsupported-target behavior.

Do not add a second capability model in a tool module. Reuse `src/capabilities/` and treat existing capability-shaped helpers under `src/tools/` as migration-sensitive integration code.