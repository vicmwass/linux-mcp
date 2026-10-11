# Spec 01 storage schema and transaction design

## Installation and ownership boundary

Every inventory row carries both `installation_id` and `owner_user_id`. These
values are opaque UUIDs supplied by the authenticated backend. Foreign keys and
composite predicates use both values; a server identifier alone is never a
repository authorization input.

Repository methods receive a trusted principal context and generate predicates of
the form:

```sql
WHERE installation_id = $installation_id
  AND owner_user_id = $owner_user_id
  AND server_id = $server_id
```

The same predicate is used for reads, updates, deletes, credential binding, trust
records, and invalidation publication. A missing row and a row outside that scope
map to the same public `not_found` error.

## Tables

### `inventory_servers`

| Column | Rule |
|---|---|
| `server_id` | UUID primary key |
| `installation_id`, `owner_user_id` | UUID, not null; composite ownership key |
| `display_name`, `hostname`, `port`, `username`, `environment` | validated contract fields |
| `approval_mode` | `medium_and_above` or `high_and_above`, default `medium_and_above` |
| `revision` | positive `bigint`, starts at 1 |
| `enabled` | not null; deletion sets false |
| `created_at`, `updated_at` | UTC timestamps |

Constraints include positive revision, valid port range, and a unique
`(installation_id, owner_user_id, server_id)` key. Hostnames are not globally
unique: two owners may intentionally manage the same hostname.

Indexes support owner-scoped stable pagination by `(updated_at, server_id)`,
lookup by `(installation_id, owner_user_id, server_id)`, and enabled resolution.

### `inventory_credentials`

One row represents a credential slot bound to one server. It stores:

- opaque `credential_id`, installation, owner, and server identifiers;
- credential kind and enabled state;
- ciphertext envelope, nonce, authentication tag, and external `key_id`;
- envelope format version and creation/rotation timestamps;
- the server revision at which the binding became current.

The row contains no plaintext and no host filesystem path. A unique constraint on
`(installation_id, owner_user_id, server_id, kind)` prevents ambiguous active
credential selection. Historical ciphertext rows may be retained as disabled
versions for recovery/audit policy, but only the current enabled version resolves.

### `inventory_host_trust`

Trust is scoped to installation, owner, server, hostname, and port. It stores the
observed fingerprint, verified fingerprint (when independently enrolled), status,
key type, observation and verification timestamps, and the server revision of the
last change. `observed` never permits a connection; `mismatch` blocks until a new
fingerprint is explicitly enrolled.

### `inventory_invalidation_events`

Each target-affecting commit appends an event containing installation, owner,
server, new revision, reason, and creation time. A monotonically increasing event
ID supports polling/replay by cache consumers. Events do not contain credentials.

The event is an invalidation hint, not the authorization mechanism: resolvers
always reload and compare the current server revision.

## Mutation transactions

1. Begin a PostgreSQL transaction.
2. Lock the scoped server row with `FOR UPDATE`; if absent, return sanitized
   `not_found`.
3. Compare `expected_revision` for updates, deletes, credential changes, and trust
   enrollment/revocation.
4. Apply the mutation and increment `revision` exactly once.
5. Append the invalidation event with the new revision.
6. Commit before notifying in-process sinks.

A notification is never emitted for an uncommitted mutation. If a sink misses an
event, the next resolver revision check remains authoritative. Deletion sets
`enabled = false` in the same transaction, so new resolutions stop immediately.

## Read and pagination rules

List queries filter ownership before applying the cursor. Cursors encode the last
`updated_at` and `server_id` pair, are opaque on the wire, and are rejected when
malformed. Ordering is deterministic and does not expose another owner's rows.
Responses contain credential status/references only, never secret material or
ciphertext.
