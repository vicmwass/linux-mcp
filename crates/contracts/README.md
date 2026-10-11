# linux-mcp-contracts

Shared Spec 00 contracts: versioned wire DTOs, errors, canonicalization, backend-only identity/credential/authority types, async service traits, JSON Schemas, fixtures and in-memory test doubles. Design decisions: [`specs/contracts/decisions.md`](../../specs/contracts/decisions.md). File ownership: [`specs/contracts/ownership.md`](../../specs/contracts/ownership.md).

This crate depends on no implementation crate (no tokio, rmcp, russh, SQL or HTTP). The root `linux-mcp` binary does not depend on it yet.

## Features

| Feature | Contents | Use from |
|---|---|---|
| `public` (default) | IDs, inventory, pagination, errors, operations, management API, MCP types, canonicalization | everyone, including browser builds |
| `schema` (default) | `schemars::JsonSchema` derives | servers, schema tests |
| `backend` | `Principal`, targets, credential leases, operation authority, service traits | server-side crates only |
| `test-support` | in-memory adapters and scenarios | tests and demos only; never production |

Browser/dashboard client code: `linux-mcp-contracts = { path = "../contracts", default-features = false, features = ["public"] }`.

## Commands (repository root)

```bash
cargo fmt -p linux-mcp-contracts -- --check
cargo clippy --offline --locked --all-features --all-targets -p linux-mcp-contracts
cargo test --offline --locked -p linux-mcp-contracts --no-default-features --features public
cargo test --offline --locked -p linux-mcp-contracts
cargo test --offline --locked -p linux-mcp-contracts --features backend
cargo test --offline --locked -p linux-mcp-contracts --all-features
cargo test --offline --locked -p linux-mcp
```

`cargo fmt --all -- --check` also checks the root application, whose files were not rustfmt-formatted before Spec 00; use the package-scoped form above for the contracts crate. Tests that spawn cargo (`public_build_cannot_import_backend_modules`, `dependency_graph_has_no_backend_leakage_or_reverse_dependencies`) are skipped with `LINUX_MCP_SKIP_PROBES=1`.

Regenerate committed schemas after an intended wire change: `LINUX_MCP_UPDATE_SCHEMAS=1 cargo test -p linux-mcp-contracts` (writes `schemas/management/` and `schemas/mcp/`), then review the diff and bump the relevant version constant in `src/lib.rs` if the change is breaking.

## Registration and versions (step 00-10, done)

- `src/lib.rs` declares `canonical`/`public` (feature `public`), `backend` (`backend`), `test_support` (`test-support`); re-exports `CANONICALIZATION_VERSION`; and provides `prelude` with curated browser-safe re-exports. Backend items are never re-exported.
- Pinned versions asserted by `tests/schema_compat.rs`: `CONTRACT_VERSION` 1.0.0 (equals the crate version), `MANAGEMENT_API_VERSION` v1, `MCP_EXTENSION_VERSION` linux-mcp/v1, `MCP_LEGACY_VERSION` linux-mcp/legacy-1, `CANONICALIZATION_VERSION` lmcp-canon-v1.
- `schema_compat` validates every fixture against the committed schemas with a small validator, requires the incompatible fixtures to fail the schemas too, and checks the published schema set is exactly the route table plus MCP extension types.
- Module owners that need new files declare child modules inside their own file; central registration changes go through the `contracts-registry` owner.
- `tests/consumer-probes/main.rs` is registered as the `consumer_probes` test (requires `test-support`).
