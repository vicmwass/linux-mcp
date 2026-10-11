# 11 — Guided workflows and optional agent extensions

Implementation: [step-by-step plan and dependencies](plans/11-workflows-implementation.md). Follow the [coordination workflow](plans/README.md).

Status: proposed. Implements Phase 4 through independently selectable packages.

## Package A: deterministic guided workflows

Own `crates/workflows`, workflow definitions, operation tracking and dashboard workflow modules. Use MCP for every Linux interaction. Begin with read-only troubleshooting for failed services, resource pressure and disk usage, using tools from 08–10. Fleet comparisons compare owned targets with bounded concurrency and report partial failures and collection times.

Saved maintenance workflows define explicit targets, bounded steps, parameter schemas, per-step privileges, expected checks and stop conditions. Check current authorization at every step. Required mutation approval is bound to the resolved step; do not treat a broad workflow label as approval for dynamic commands. Persist progress and unknown outcomes; after restart reconcile by inspection before allowing continuation. Never automatically replay submitted mutations or promise generic rollback.

Acceptance: a fixture workflow produces evidence, proposes exact actions, pauses for approval, executes at most once and records verification. Revocation or target revision changes block subsequent steps. Partial fleet failures do not fabricate a complete result. Every result and persisted workflow is owner-scoped.

## Package B: optional WebMCP adapter

Separate opt-in feature exposing curated dashboard actions to compatible browser agents. Verify the current specification, browser support and Rust integration feasibility before selecting this package. It does not replace the backend MCP client or weaken server-side permissions, approval and audit. If browser integration needs generated bindings, document them; do not silently abandon the Rust application-logic requirement.

Acceptance: manual and browser-agent requests reach the same backend validation; unsupported browsers retain working core pages. Browser-agent calls cannot self-approve guarded mutations.

## Package C: optional embedded assistant

Separate model-provider integration and tool orchestrator. No LLM is required for Packages A/B or ordinary dashboard operation. Users explicitly configure provider/credentials and what remote output may leave the installation. Treat logs, files and tool output as untrusted data, not instructions. Apply bounded context, tool-call budgets, redaction and the existing approval boundary.

Acceptance: malicious output cannot change user identity, target ownership or approval requirements. Model outages leave deterministic features functional. Provider secrets never reach browser bundles. Self-hosted installations can disable this package without losing core functionality.
