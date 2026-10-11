# Spec 02 design — identity provider and authorization (step 02-01)

Status: design, 2026-10-11. Scope: provider choice, authorization flows, claims, subject mapping, session and revocation model. No product code. Companion documents: [flows.md](flows.md), [revocation-and-sessions.md](revocation-and-sessions.md), [acceptance-matrix.md](acceptance-matrix.md). Inputs: [Spec 02](../../02-identity-and-authorization.md), frozen contracts ([HANDOFF](../../contracts/HANDOFF.md), `Principal`, `VerifiedIdentity`, `Identity`, `Scope`, `AuthKind`).

## Decision summary

| Question | Decision |
|---|---|
| Authorization server | **Keycloak** (one realm per installation), self-hosted, packaged by Spec 07. No custom OAuth protocol and no custom authorization server. |
| Resource server | `crates/identity` validates access tokens for the MCP service (Spec 04) as an OAuth 2.1 resource server and publishes RFC 9728 protected-resource metadata. |
| Dashboard login | OIDC authorization code + PKCE with the dashboard backend as a confidential client (`openidconnect` crate). Browser holds only an opaque session cookie. |
| Remote agents | Standard MCP OAuth directly between the MCP client and Keycloak; our service only validates tokens. |
| Dashboard → MCP authority | User-delegated access tokens obtained by the dashboard backend through its own authorization-code grant for the MCP resource. A service token plus a browser-supplied user ID is never used. |
| MCP revisions | Target **2025-11-25** and design to be stateless-compatible with **2026-07-28** (see below). |
| Revocation bound | ≤ 60 s for new operations by short introspection cache + local revocation list; fail closed on staleness. |

### Why this matters now: the MCP revision moved

The official 2026-07-28 revision (changelog verified) removes protocol-level sessions and the `Mcp-Session-Id` header, removes the `initialize` handshake, and requires authorization on every request; it deprecates Dynamic Client Registration in favor of Client ID Metadata Documents and adds RFC 9207 `iss` validation. Spec 02's "bind MCP sessions to the subject" requirement is therefore redefined (see [revocation-and-sessions.md](revocation-and-sessions.md)): identity is derived from the bearer token on every request, and any session a 2025-11-25 transport creates is only a cache key bound to (issuer, subject, client), never authority. The rmcp version in `Cargo.lock` (3.2.0) defaults to 2025-11-25; support for 2026-07-28 in rmcp is **not verified** and is a 04-01 check.

## Provider evaluation

Requirements taken from the MCP authorization specification (2025-11-25 and 2026-07-28): authorization server with OAuth 2.1 behavior and PKCE S256, RFC 8414 or OIDC discovery with `code_challenge_methods_supported`, RFC 8707 resource indicators or an equivalent audience mechanism, a client-registration path (CIMD, pre-registration or DCR), exact redirect-URI validation, short-lived tokens with refresh rotation for public clients; plus Spec 02: self-hostable, maintained, OIDC login for the dashboard, revocation, and one realm/issuer per installation.

| Candidate | Evidence | Verdict |
|---|---|---|
| **Keycloak** | Its own [MCP guide](https://www.keycloak.org/securing-apps/mcp-authz-server) (for nightly 26.8.0) lists: OAuth 2.1 Supported; RFC 8414 Supported; RFC 9207 Supported; RFC 7591 DCR Supported; **RFC 8707 Experimental** (`--features=resource-indicators`); **CIMD Experimental** (`--features=cimd`); MCP 2025-03-26 Supported, 2025-06-18 / 2025-11-25 / 2026-07-28 Experimental. Stable audience path: scope + `Audience` mapper with the MCP URL as included custom audience. Also provides the OIDC login the dashboard needs. | **Chosen.** One product covers dashboard login, agent authorization and revocation. Caveats below. |
| Ory Hydra | Only a third-party table ([Zuplo](https://zuplo.com/learn/mcp/compatibility/identity-providers)) was found: resource parameter yes, CIMD no, DCR off by default; not confirmed from Ory documentation. Hydra is an OAuth/OIDC server only: login, consent and user management UI would have to be built by us. | Rejected for scope: builds an identity UI we would own. Could be revisited. |
| ZITADEL | Third-party table (same source) reports DCR and RFC 8707 not supported; not confirmed from ZITADEL documentation. | Rejected for the MCP agent path; unverified. |
| Custom authorization server (for example on `oxide-auth`) | Spec 02 prohibits a custom OAuth protocol; maintenance and security burden. | Rejected. |

Not exhaustively evaluated: Authentik, Dex, Logto and others. The decision is a documented, reversible default, and the resource-server code depends only on standards (RFC 9068-style JWT access tokens, RFC 8414 / OIDC discovery, RFC 7662 introspection), so another compliant server can replace Keycloak without changing `crates/identity` contracts.

### Caveats and conditions on the Keycloak choice

1. **Experimental features are off by default.** The baseline works without `resource-indicators` and `cimd`: clients register with DCR (supported) or are pre-registered, and audience binding uses the scope→audience mapper. Our resource server validates that the token `aud` contains its canonical resource URI regardless of how Keycloak set it, so enabling RFC 8707 later changes only Keycloak configuration. Enabling an experimental feature in production requires an explicit installation setting and a recorded risk acceptance, because the guide warns they "may introduce breaking changes".
2. **Version pinning.** The guide documents the 26.8.0 nightly; the released version that Spec 07 pins must be verified against Keycloak release notes before packaging. This document does not name a released version.
3. **Not a Rust component.** The provider is a separately deployed dependency (Java). The all-Rust requirement applies to our dashboard, MCP service and identity crate; Spec 07 must package and operate Keycloak (PostgreSQL, TLS, realm import, upgrade path). If that is unacceptable the fallback is pre-selected Hydra-style composition, with the UI cost above.
4. **Client compatibility is provider-guide documentation, not our verification.** See the client list below.

### Rust-side libraries (checked on crates.io, 2026-10-11)

| Crate | Use | Observation |
|---|---|---|
| `jsonwebtoken` 11.1.0 (updated 2026-09-16, MIT) | access-token signature, `exp`, `aud`, `iss` validation with an explicit algorithm allow-list | actively maintained |
| `openidconnect` 4.0.1 (updated 2025-07-06, MIT, ramosbugs) | dashboard OIDC code flow, state/nonce/PKCE, discovery | widely used, slower cadence; acceptable, isolated behind our own `browser` module so it can be replaced by `oauth2` plus custom ID-token checks |
| `rmcp` 3.5.1 on crates.io (updated 2026-10-05); lockfile has 3.2.0 | MCP transport in Spec 04; exposes an `auth` feature | support for the 2026-07-28 revision not verified here |

Exact versions are chosen and locked in 02-02 with `cargo` metadata, not from this table.

## Target MCP clients (named, with verification status)

The Keycloak guide's client-specific notes concern its experimental CIMD path. Our own end-to-end verification of any client is a **04/07 gate**, not completed here.

| Client | What the provider guide says | Our status |
|---|---|---|
| MCP Inspector (test harness) | needs CORS and an anonymous client-registration policy on the registration endpoint | planned harness for 02/04 |
| Claude Code | CIMD with localhost callbacks needs "Restrict same domain" off; shares the `claude.ai` client_id domain with Claude Desktop | unverified |
| Claude Desktop / claude.ai connectors | needs "Accept Public Client with Confidential-only Grant Types" on | unverified |
| VS Code | localhost callbacks; same setting as Claude Code | unverified |
| ChatGPT | **not supported** with Keycloak CIMD (unparsed `token_endpoint_auth_methods_supported`); needs a custom client-policy executor. DCR path untested. | explicit limitation |

## Sources (verified 2026-10-11)

- MCP authorization, revision 2025-11-25: <https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization>
- MCP authorization, latest (2026-07-28): <https://modelcontextprotocol.io/specification/latest/basic/authorization>
- MCP 2026-07-28 changelog: <https://modelcontextprotocol.io/specification/2026-07-28/changelog>
- Keycloak MCP guide: <https://www.keycloak.org/securing-apps/mcp-authz-server>
- Third-party comparison (unconfirmed, used only as a lead): <https://zuplo.com/learn/mcp/compatibility/identity-providers>
- crates.io API pages for `jsonwebtoken`, `openidconnect`, `rmcp`.

## Open items and blockers

- Verify rmcp's supported MCP revisions and its `auth` feature before 04-01; if rmcp lags, 04 may need a thin layer, not a fork.
- Confirm Keycloak released version and stability of RFC 8707/CIMD at packaging time (07).
- Verify Keycloak's standard token exchange as an optional alternative to the delegated flow (02-05).
- Real-client verification (named above) belongs to 04/07; 02-08 hands off the exact assumptions.
- Product-owner awareness: Keycloak is non-Rust and adds an operated service. This is a design decision, not a blocker; reversal cost is bounded by the standards-only resource-server design.
