# Spec 01 design acceptance matrix

| Requirement | Boundary | Verification |
|---|---|---|
| Owner and installation isolation | Scoped predicates on every repository/trust/credential operation | Alice/Bob guessed-ID, duplicate-hostname, list and mutation tests |
| UUID IDs and revisions | `inventory_servers` primary key and row lock/update transaction | Stale update/delete/trust/credential conflicts |
| CRUD and pagination | `Inventory` repository with deterministic owner-scoped cursor | Restart persistence and pagination isolation tests |
| Write-only credentials | `CredentialInput` -> AES-GCM envelope; no secret response type | Database export, tamper, wrong-key, debug and response redaction tests |
| Key rotation and recovery | External `key_id`, versioned envelopes, transactional active-version swap | Rewrap success/failure and old-key retirement tests |
| Lease lifecycle | `AuthorizedTarget`-bound lease, max 60 seconds, deferred cleanup | Foreign/stale reference and expiry tests |
| Host trust | Observation separate from verified enrollment; owner/server/host/port scope | Unknown, observed, mismatch, changed-key and re-enrollment tests |
| Destination safety | DNS result and actual connection address evaluated against policy | IPv4/IPv6, alternate DNS, private/metadata range and port tests |
| Revision invalidation | Mutation and event append commit atomically | No event on rollback; missed-event revision fallback |
| Delete semantics | Disable immediately; cleanup after bounded legitimate leases | New resolutions fail while submitted operations may finish |
| Approval mode | Per-server field, default medium threshold, browser-only mutation | Default/toggle, agent-token refusal and stale approval invalidation |

Critical unresolved contract changes would block this step. The frozen contracts
already expose the required inventory, resolver, credential, trust, revision,
approval-mode, and invalidation interfaces, so implementation can proceed without
changing Spec 00.
