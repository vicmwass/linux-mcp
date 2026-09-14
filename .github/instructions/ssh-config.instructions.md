---
name: SSH and configuration safety
description: "Apply when editing server configuration, SSH connection management, host-key verification, capability detection, or execution limits."
applyTo: "src/ssh/**/*.rs,src/config/**/*.rs,src/capabilities/**/*.rs,config/**/*.toml"
---

# SSH and configuration safety

- Never log, return, or place private-key contents in MCP responses or diagnostics.
- Preserve host-key verification through `known_hosts`; do not add permissive fallback trust.
- Keep connection timeouts and execution limits explicit. Consider both remote lifetime and buffered output when changing execution behavior.
- Treat server IDs as configuration lookups, not arbitrary hostnames supplied by a caller.
- When adding a capability, update detection, parsing, caching/invalidation, resolver validation, and tests together.
- Keep secrets and environment-specific paths out of committed examples unless they are clearly placeholders.