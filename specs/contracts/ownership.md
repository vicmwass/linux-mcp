# Spec 00 file ownership map

Authoritative path assignments for the coordinator resources in `specs/plans/00-contracts.steps.json`. A worker edits only paths owned by a resource its claimed step reserves. Paths not listed here belong to nobody in Spec 00 and must not be edited by Spec 00 steps (in particular `src/**` of the root `linux-mcp` package: no runtime change).

| Resource | Paths | Steps |
|---|---|---|
| `contracts-design` | `specs/contracts/decisions.md`, `specs/contracts/ownership.md`, `specs/contracts/current-surface.md`, `specs/contracts/acceptance.md`, `specs/contracts/HANDOFF.md` | 00-01, 00-12 |
| `architecture-doc` | `linux-mcp-architecture-explanation.md` (local, Git-ignored) | 00-01, 00-02, 00-10, 00-11, 00-12 |
| `workspace-manifests` | root `Cargo.toml` (workspace table only), `Cargo.lock`, `crates/contracts/Cargo.toml` | 00-02, 00-10 |
| `contracts-registry` | `crates/contracts/src/lib.rs`, `crates/contracts/src/public/mod.rs`, `crates/contracts/src/backend/mod.rs`, `crates/contracts/src/test_support/mod.rs`, `crates/contracts/README.md` | 00-02, 00-10, 00-12 |
| `contracts-public-models` | `crates/contracts/src/public/{ids,inventory,errors,pagination}.rs`, `crates/contracts/tests/public_models.rs` | 00-03 |
| `contracts-backend-models` | `crates/contracts/src/backend/{identity,target,credentials}.rs`, `crates/contracts/tests/backend_boundaries.rs` | 00-04 |
| `contracts-operation-models` | `crates/contracts/src/public/operations.rs`, `crates/contracts/src/canonical.rs`, `crates/contracts/src/backend/operation_authority.rs`, `crates/contracts/tests/operations.rs`, `fixtures/operations/` | 00-05 |
| `contracts-management-api` | `crates/contracts/src/public/api.rs`, `crates/contracts/tests/management_api.rs`, `schemas/management/`, `fixtures/management/` | 00-06 |
| `contracts-service-traits` | `crates/contracts/src/backend/services.rs`, `crates/contracts/tests/service_traits.rs` | 00-07 |
| `contracts-mcp-fixtures` | `crates/contracts/src/public/mcp.rs`, `crates/contracts/tests/mcp_fixtures.rs`, `schemas/mcp/`, `fixtures/mcp/`, `specs/contracts/mcp-compatibility.md` | 00-08 |
| `contracts-mocks` | `crates/contracts/src/test_support/` except `mod.rs`, `crates/contracts/tests/mocks.rs`, `fixtures/scenarios/` | 00-09 |
| `contracts-schema-tests` | `crates/contracts/tests/schema_compat.rs` | 00-10 |
| `contracts-consumer-probes` | `crates/contracts/examples/`, `tests/consumer-probes/`, `specs/contracts/validation.md` | 00-11 |

`schemas/` and `fixtures/` are under `crates/contracts/` (that is, `crates/contracts/schemas/…`, `crates/contracts/fixtures/…`) so the crate is self-contained; the short forms above are relative to the crate root. `tests/consumer-probes/` is likewise `crates/contracts/tests/consumer-probes/`.

## Placeholders and registration

Step 00-02 creates every module file above as a compilable placeholder and registers it in the `mod.rs` files, so module owners never need to edit registry files. If a module needs a new file, it uses a child module declared inside its own file (`#[path]` is not used). Re-export changes in `lib.rs` are requested from 00-10.

## Feature boundaries

- Browser/dashboard client code depends on `linux-mcp-contracts` with `default-features = false, features = ["public"]` and therefore cannot reach `backend`/`test_support`. No browser-side type references secrets or `Principal`.
- `backend` modules are `#[cfg(feature = "backend")]` at the `mod` declaration in `lib.rs`; `test_support` is `#[cfg(feature = "test-support")]`.
- Exact defaults are validated in 00-02 (builds for: no default features + `public`; default; `backend`; `test-support`).
