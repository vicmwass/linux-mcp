# 08 — First management console

Implementation: [step-by-step plan and dependencies](plans/08-console-implementation.md). Follow the [coordination workflow](plans/README.md).

Status: proposed. Implements Phase 2 of the management-client plan after the secure foundation.

## Independent work packages

| Module owner | Server deliverable in resource-tools | Dashboard deliverable |
|---|---|---|
| Health | Structured overview query | Overview: distribution, uptime, reachability, health and collection time |
| Services | Service listing/detail and supported start/stop/restart | Searchable service table, failure details, action preview and verification |
| Logs | Bounded filtered log query | Logs: service/severity/time filters and incremental retrieval |
| Users adapter | Existing tool/result adapters | Users: inspect/create/lock/unlock/delete with accurate semantics |

Each module owns separate source files, DTO fixtures and tests. Server and UI implementations can proceed in parallel once their 00 contract extensions are frozen. The 04 owner registers tools; the 05 owner registers pages; modules do not edit shared routers concurrently.

## Contracts and behavior

- All DTOs include server ID/revision and collection timestamp. Specify units, optional/unsupported fields and explicit errors. Bound list sizes and use stable cursors where supported.
- Health reports observed resource data; lack of a response is unavailable, not a fabricated unhealthy measurement.
- Service contracts define supported init systems, exact service identifiers, observed state and permitted actions. Inputs are validated and safely quoted through core execution.
- Log contracts define time range, maximum bytes/entries, truncation, continuation and supported backend. Sensitive output is owner-scoped and not persisted by default. Escape rendered content.
- Mutation preflight produces the exact target/action intent for 06. After submission, inspect actual state and report verified, mismatch or verification-unavailable. Command exit success alone is not verified service state.
- Existing password-lock actions must explain that SSH keys and active sessions remain unaffected.
- No automatic raw-shell fallback for unsupported distributions or missing tools.

## Acceptance

On disposable supported Linux targets, a user can inspect overview, identify a failed service, inspect relevant logs, approve a supported service action and see its observed result. Refusal, unsupported capabilities, truncated logs, stale data and disconnect-after-submit are represented correctly. Two users cannot access each other's queries, cursors, logs or actions. Each module supplies mock-backed server/UI demos, contract fixtures and live target verification instructions.
