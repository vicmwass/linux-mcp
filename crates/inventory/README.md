# linux-mcp inventory

This crate owns Spec 01 persistence boundaries for owner-scoped servers,
credentials, host trust, invalidation events, and the outbound destination policy.
It does not execute SSH commands or authenticate users.

The crate uses PostgreSQL through SQLx. Migrations are namespaced under
`crates/inventory/migrations` and use monotonically ordered UTC-style version
numbers so future crates can add migrations without renaming this crate's files.
The migration runner is exposed through [`run_migrations`].

Current bootstrap scope:

- typed database configuration;
- compile-time embedded SQLx migrations;
- schema for servers, encrypted credential envelopes, host trust, and
  invalidation events;
- module placeholders for repository, credentials, trust, destination policy, and
  resolver work owned by later Spec 01 steps.

Set `DATABASE_URL` only in local or disposable test environments. The application
must provide the external key provider and destination-policy configuration; this
crate never reads private keys from arbitrary filesystem paths.
