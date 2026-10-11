# Spec 00 validation record (step 00-11)

Recorded 2026-10-11 (re-run after the approval-mode change in 00-12) on Windows 11, rustc 1.95.0, cargo 1.95.0, `--offline --locked` unless noted. All commands run from the repository root.

Two kinds of evidence are kept apart on purpose:

- **Real checks** — the compiler, linker, test harness and cargo actually ran. They prove types, features, schemas, dependency graphs and builds.
- **Simulated behavior** — the in-memory `World` adapters (`test_support`) acting out the contract. They prove consumers can use the contract and that its documented outcomes are expressible. They do **not** prove production isolation: no database, OAuth provider, SSH server or real secret store was involved.

## Real checks

| Check | Command | Result |
|---|---|---|
| Formatting (contracts only; root app was never rustfmt-clean) | `cargo fmt -p linux-mcp-contracts -- --check` | pass |
| Lints | `cargo clippy --offline --locked --all-features --all-targets -p linux-mcp-contracts` | 0 warnings |
| Public-only build and tests | `cargo test -p linux-mcp-contracts --no-default-features --features public` | 37 passed |
| Default features | `cargo test -p linux-mcp-contracts` | 50 passed |
| Backend feature | `cargo test -p linux-mcp-contracts --features backend` | 81 passed |
| All features (incl. test-support, examples, doctests) | `cargo test -p linux-mcp-contracts --all-features` | 122 passed, 0 failed |
| Root application regression | `cargo test -p linux-mcp` | 51 passed (same as the 00-01 baseline) |
| Browser build, public only | `cargo build -p linux-mcp-contracts --target wasm32-unknown-unknown --no-default-features --features public` | pass |
| Browser build, public + schema | `cargo build -p linux-mcp-contracts --target wasm32-unknown-unknown` | pass |
| Standalone public consumer crate, host | test `public_consumer_builds_on_host` (`cargo check`) | pass |
| Standalone public consumer crate, wasm32 | test `public_consumer_builds_for_wasm32` (`cargo check --target wasm32-unknown-unknown`) | pass |
| Backend types unreachable from public builds | `backend_boundaries::public_build_cannot_import_backend_modules` | pass |
| Dependency graph | `schema_compat::dependency_graph_has_no_backend_leakage_or_reverse_dependencies` | pass: public tree has no async-trait, zeroize, schemars, getrandom, tokio, rmcp, russh; no reverse dependencies on the contracts crate |
| Schemas and fixtures agree | `schema_compat` (9 tests) | pass |
| Compile-fail boundaries | doctests: `Principal`/`VerifiedIdentity` cannot be deserialized or defaulted; `CredentialMaterial` not Clone/Serialize; `AuthorizedTarget` and `ExecutionAuthority` not Clone/Serialize; IDs cannot be mixed or converted | pass |
| Runnable examples | `cargo run -p linux-mcp-contracts --example {inventory_client,identity_flow,executor_flow,governance_flow,mcp_adapter} --features test-support`; `--example dashboard_public --no-default-features --features public` | all run and print the expected flow |

**wasm32 tooling.** `wasm32-unknown-unknown` was not installed. With the user's explicit approval it was added with `rustup target add wasm32-unknown-unknown`. The first wasm build failed offline because `uuid`'s default `std` feature carries weak `wasm-bindgen?` references that force cargo to fetch the wasm-bindgen tree on every wasm32 resolve. `crates/contracts/Cargo.toml` therefore sets `uuid` to `default-features = false, features = ["serde"]` (the code needs no uuid `std`). This one-line manifest change was made during 00-11 although the manifest belongs to `workspace-manifests`; no other worker held that resource. The wasm builds above were run after it. The wasm probe fails, rather than skips, if the target is missing.

## Simulated behavior (consumer probes, mock adapters)

`crates/contracts/tests/consumer-probes/main.rs` (13 tests, all passing) and `tests/mocks.rs` (28 tests) drive mock consumers — inventory, identity, executor, an MCP adapter, governance and a dashboard view model — through the documented call sequence:

| Scenario | Observed contract outcome |
|---|---|
| Alice/Bob isolation, including guessed IDs | foreign ID is byte-identical to an absent ID (`not_found`); no remote attempt; no side effects on Bob's server |
| Same hostname, different owners | distinct credential IDs, host fingerprints and cache keys per attempt; attempt log shows each owner's own credential |
| Revoked identity | existing contexts, fresh token validation and service calls all fail `unauthenticated`; other users unaffected; revocation after approval blocks execution |
| Stale target | revision change after approval invalidates the approval; old target cannot lease credentials; pooled connection evicted |
| Changed arguments | an approval cannot be consumed by a different intent (`binding_mismatch`); the failed attempt does not burn it |
| Expired approval | `expired`, nothing sent |
| Reused approval / unknown outcome | `consumed` after dispatch, including after `unknown_after_submission`; attempt log shows a single dispatch |
| Concurrent consume | 16 threads, exactly one dispatch |
| Secret redaction | no fake secret marker, token or session value appears in results, errors, audit, `Debug` output or serialized summaries; secret-named arguments are rejected |
| Policy refusal | returned as an outcome (`refused`), distinct from a protocol error; creates no approval |
| `confirm: true` | never produces execution authority; mutations still return `approval_required` |
| Per-server approval mode (D11) | default `medium_and_above`: medium and high need approval; after an owner browser session sets `high_and_above`, medium runs and high still needs approval; denials unchanged; Bob's server unaffected; an MCP token with inventory-write scope gets `forbidden` when changing or creating with a relaxed mode; changing the mode invalidates an approved-but-unused approval |
| Timeout before / disconnect after submission | `failed_before_submission` (nothing sent) vs `unknown_after_submission` (never retried) |

## What this does not show

- Production isolation, durability or atomicity (the mock uses one mutex; a real service needs database transactions).
- OAuth/OIDC validation, revocation latency (the 60 s target is untested), CSRF in a real browser.
- Any live SSH behavior, host-key handling by `russh`, or the real command classifier. Policy in the mock is a stand-in.
- Live MCP transport interoperability (no transport exists).
- The exact approval semantics decision (decisions.md Q2) still needs product-owner confirmation before the 00-12 freeze.
- No external machine was contacted and no real secret was used.
