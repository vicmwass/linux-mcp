# 01 — Inventory, credentials and host trust

Implementation: [step-by-step plan and dependencies](plans/01-inventory-implementation.md). Follow the [coordination workflow](plans/README.md).

## Deliverable and ownership

Own `crates/inventory`, its PostgreSQL migrations, management API handlers and secret-provider adapter. Use 00 contracts and a mock Principal until 02 integrates. No SSH command execution or login implementation belongs here.

## Requirements

- Store users' server records with enforced owner IDs, UUID identifiers, revisions and timestamps. Queries always constrain owner and installation boundary.
- Store a per-server `approval_mode` (`medium_and_above` default, or `high_and_above`) with the server configuration, accepted on create/update and returned in summaries. Resolvers return the current value with every `ResolvedTarget`. A change increments the revision and publishes an invalidation (pending approvals bound to the old revision become unusable). Setting a non-default value on create, or changing it on update, requires an interactive browser session with CSRF; MCP and delegated tokens receive `forbidden`, even with inventory-write scope. Migration default for existing rows is `medium_and_above`.
- Provide authenticated list/create/read/update/delete routes under `/api/v1/servers`; pagination, input validation and optimistic concurrency are required.
- Provide write-only credential provisioning and rotation. Responses expose only status and references. Users cannot select another owner's credential reference or arbitrary host filesystem paths.
- Encrypt SSH keys and sudo secrets with authenticated encryption using an established library. Keep the wrapping/master key outside the database and image. Define key identifiers, rotation and recovery behavior. Never log plaintext; minimize decrypted lifetime.
- Store verified host trust per owner and target, including port. First connection rejects unenrolled keys. Enrollment presents an observed fingerprint for independent verification; observation alone does not establish trust. Changed keys block connections until explicitly reverified.
- Define an outbound-target policy for the hosted installation: allowed networks/ports, blocked infrastructure endpoints, DNS resolution checks and rechecks at connection time. Self-hosted private-network access is configurable. Authorization alone does not make every supplied destination acceptable.
- Changes increment revision. Resolvers return current records for each operation. Publish invalidation events for pooled connections and capability caches, with revision checks as a correctness backstop if events are missed.
- Deletion disables future resolutions immediately; cleanup of secrets waits until no legitimate in-flight reference needs them. Already submitted remote commands may finish.

## Acceptance

Alice cannot list, retrieve, alter, delete or reference Bob's inventory, credential or host trust, including guessed identifiers. Identical hostnames across owners remain separate. CRUD survives restart. Concurrent stale edits fail. Secret ciphertext is unreadable without the external key. Secret values never appear in reads or diagnostics. Rotation and deletion become visible without restart. Trust mismatch and prohibited destinations fail closed.

Deliver a standalone management API demo using mock identity, migrations, adapter tests and fixture credentials. No real external machine is needed until the execution integration gate.
