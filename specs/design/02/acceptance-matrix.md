# Spec 02 acceptance matrix (design → implementing step → proof)

Covers [Spec 02](../../02-identity-and-authorization.md) requirements and acceptance, mapped to plan steps. "Real" means a disposable Keycloak (or the 02-02 test issuer for unit-level key tests) and real HTTP/crypto; mock-only evidence does not satisfy a real gate.

| Requirement or acceptance | Design reference | Step | Proof |
|---|---|---|---|
| Dashboard login/logout, HTTP-only secure cookies, CSRF, expiry | flows.md Flow A | 02-03 | browser tests: cookie flags, CSRF failures, fixation, expiry |
| Signup policy; self-hosted first admin, public signup off | flows.md mapping | 02-06 | bootstrap cannot be reused on an initialized installation |
| Stable subject → local user; email never the key | flows.md subject mapping | 02-03 | duplicate email/different subject, email change, same subject new email |
| MCP OAuth discovery appropriate to the current spec | README, flows.md Flow B | 02-04 | metadata served from configured base URL; 401/403 challenges |
| Validate issuer, audience/resource, signature, lifetime, scopes | flows.md step 5 | 02-04 | wrong issuer/audience/algorithm/`kid`, expired, malformed, key rotation, insufficient scope (real signed tokens) |
| Trusted `Principal` only from verified authority | contract `Principal::from_verified` | 02-04, 02-07 | invalid tokens never construct a principal; crate has no other construction path |
| Cookies do not authorize agents; tokens do not authorize browser | flows.md Flow A/B | 02-07 | cross-substitution tests both directions |
| Never take identity from server ID, URL, tool argument or MCP session ID | revocation-and-sessions.md | 02-07 | fixture MCP consumer with hostile arguments and session IDs |
| Scopes distinguish read / inspect / mutate; still subject to ownership | flows.md step 7 | 02-04 | tool-to-scope table tests; `approvals:decide` never on agent tokens |
| Bind MCP sessions to subject; reject cross-user reuse; revalidate on reconnect | revocation-and-sessions.md | 02-07 | covers 2025-11-25 sessions and the sessionless 2026-07-28 behavior |
| Delegated dashboard authority; no shared service token with browser-supplied user ID | flows.md Flow C | 02-05 | consent failure, refresh expiry, user switch, token never reaches browser |
| Browser sessions and delegated/agent tokens revoke independently | flows.md authority table | 02-06 | each action leaves the other two kinds working |
| Revoked users/tokens lose new-operation access within 60 s; restart preserves | revocation-and-sessions.md | 02-06 | controllable clock + real provider, multi-replica |
| Expired, wrong-audience, wrong-issuer, insufficient-scope, revoked, malformed fail | flows.md step 5 | 02-04, 02-08 | negative suite for Alice/Bob |
| User A cannot attach to B's MCP session | revocation-and-sessions.md | 02-07 | |
| Logout removes browser session; grants separately revocable | flows.md authority table | 02-03, 02-06 | |
| CSRF attempts fail | flows.md Flow A | 02-03 | |
| Two isolated installations reject each other's tokens | flows.md installation model | 02-08 | two realms/issuers/resources |
| Test issuer never accepted in production configuration | README, 02-02 plan | 02-02, 02-08 | production profile refuses mock issuer unless explicitly in a test environment |
| Named MCP-client compatibility | README client list | 04/07 gates | 02-08 records the exact assumptions only; real clients are not verified in Spec 02 |

## Explicit non-goals and limits for Spec 02

- No claim of end-to-end compatibility with any named client until 04/07 verify it.
- No custom OAuth protocol, token minting or authorization server.
- Revocation cannot cancel an already-submitted SSH operation.
- Provider migration between issuers is not automated in v1.
