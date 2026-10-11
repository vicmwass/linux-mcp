# Spec 03 regression baseline

Captured 2026-10-11 from the working tree at commit `31fb043` (plus uncommitted planning/contracts files). Extraction (03-02) must keep every item below passing and unchanged in meaning.

## Commands

```bash
cargo test --offline --locked -p linux-mcp          # 51 tests, 0 failed (baseline)
cargo build --offline --locked                       # root binary builds
python scripts/verify_sudo.py                        # live: confirmation refusal, UID 0 elevation, ordinary UID, password-isolated stdin, user listing (two hosts)
python scripts/verify_sudo_failures.py               # live: missing/empty/multiline/incorrect password, no silent fallback
python scripts/verify_user_lifecycle.py              # live: create/disable/enable/delete on a development host
```

The three scripts launch the real stdio server against configured hosts and never read password files themselves; they are optional live smoke checks and run only against disposable, explicitly configured machines (the final 03 gate), never as part of default tests.

## Unit-test inventory (51)

| Module | Tests | What it protects |
|---|---|---|
| `commands::normalize` | 9 | command normalization rules |
| `commands::parser` | 8 | parsing, operation classification, shell-construct rejection |
| `commands::resolver` | 7 | capability-aware resolution (package manager, init system, user commands) |
| `execution::analyzer` | 6 | risk/operation aggregation |
| `security::policy` | 11 | denial rules, confirmation rules, capability validation, catastrophic delete detection |
| `ssh::sudo` | 5 | sudo stdin framing, metacharacter and whitespace preservation, no-secret errors, `sudo -n` without a secret, missing secret fails closed |
| `tools::users` | 5 | session-check interpretation and user-tool validation |

These move with their modules in 03-02 without edits. New multi-user tests are added beside them, not instead of them.

## Behaviors to preserve (and where they are checked)

| Behavior | Source | Check |
|---|---|---|
| Unknown/critical operations and catastrophic deletes are denied; confirmation cannot override | `security::policy` | policy unit tests |
| High risk and sudo-needing commands require confirmation (legacy) | `security::policy` | policy unit tests; legacy adapter fixture |
| Sudo password never an argument or MCP parameter; stdin only; `sudo -k -S -p ''` with `sh -c` and `</dev/null` wrapper; missing configured file fails, no fallback | `ssh::sudo` | sudo unit tests + `verify_sudo_failures.py` |
| `password lock` wording: disable/enable do not revoke SSH keys or end sessions | `tools::users` messages, docs | message fixture |
| Host keys must match `known_hosts`; missing/untrusted rejected | `ssh::connection` | legacy live smoke; fake-transport test for the new fingerprint check |
| Retry once only before exec (closed connection, channel-open failure); never after submission | `ssh::manager` | fake-transport tests (new) |
| Timeout covers connect + queue + exec; remote outcome "may be unknown" | `ssh::manager` | fake-transport timeout test (new) |
| Result text formats and refusal prefixes | `mcp::server`, `tools::users` | `crates/contracts/fixtures/mcp/legacy` (captured 2026-10-11) and the text classifier tests |

## Expected outcomes to record before extraction

| Scenario | Legacy text prefix | Typed outcome after extraction |
|---|---|---|
| Unknown server | `Failed to detect server capabilities:` / `Failed to get user:` | legacy adapter: same text; multi-user: resolver `not_found` |
| Unparseable/unsupported command | `Command rejected:` | `Refused { UnsupportedOperation }` |
| Policy denial | `This operation is not permitted.` / `Security policy rejected the command:` | `Refused { PolicyDenied | CapabilityMissing }` |
| Needs confirmation | `This operation requires explicit confirmation.` | legacy: same; multi-user: `RequiresApproval` |
| `delete_user` without confirm | `Failed to delete user:\nUser deletion requires explicit confirmation. Set confirm=true.` | legacy: same |
| Non-zero exit | `Exit code: N …` | `Succeeded { exit_status: Code(N) }` |
| Timeout | `Command execution failed:` … "remote outcome may be unknown" | `UnknownAfterSubmission { Timeout }` (or `FailedBeforeSubmission` if before exec) |
| Disconnect after exec | `Command execution failed:` + error | `UnknownAfterSubmission { Disconnected }` (**changed typing**, same text) |

## Test strategy for the new library

- A fake `Transport` (scripted connect/auth/host-key/exec results, controllable clock, attempt log) drives executor tests without SSH: submission-boundary matrix, retry rule, output truncation, per-user concurrency, pool isolation for identical hostnames, invalidation, foreign target never connects, disconnect-after-submit never retried.
- Live SSH only at the final gate against designated disposable targets.
- The contracts crate's `test-support` mocks are used only as dev-dependency scenarios for the adapter layer.
