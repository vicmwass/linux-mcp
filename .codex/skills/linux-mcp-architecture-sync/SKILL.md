---
name: linux-mcp-architecture-sync
description: Keep the linux-mcp architecture explanation synchronized with project edits. Use whenever changing source, Docker, MCP catalog, configuration, security policy, or remote-access behavior in this repository.
metadata:
  short-description: Update linux-mcp architecture documentation after edits
---

# linux-mcp architecture synchronization

When editing this repository, update `linux-mcp-architecture-explanation.md` in the project root during the same task.

Document the final implemented behavior, including:

- MCP tools and their catalog entries.
- The request path through parsing, resolution, analysis, policy, planning, execution, and SSH.
- The distinction between classified administrative commands and structured inspection commands.
- SSH identity and host-key behavior.
- SFTP and FTPS support, if implemented; do not claim support that is only planned.
- Docker image, gateway, and client configuration changes.
- Validation performed and any known limitations.

Read the existing architecture explanation before editing it. Preserve useful content, update stale sections, and avoid recording speculative designs as implemented behavior. Keep the document concise and technically accurate.
