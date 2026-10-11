# Spec 02 flows, claims and mapping

Contract anchors: `Identity` trait (`validate_mcp_token`, `authenticate_browser`, `revoke`, `revalidate`), `VerifiedIdentity` → `Principal::from_verified`, `AuthKind::{BrowserSession, McpToken, DelegatedToken}`, `Scope` (`inventory:read`, `inventory:write`, `credentials:write`, `servers:inspect`, `servers:mutate`, `approvals:request`, `approvals:decide`, `audit:read`), `SessionRef` (1–128 visible ASCII, never a bearer value). `crates/identity` is the only crate allowed to build `VerifiedIdentity`.

## Installation model

One **installation** = one `InstallationId` = one Keycloak realm (issuer) = one MCP resource URI. Hosted multi-user installations share the realm across users; independent self-hosted installations each run their own. Configuration (not request data) fixes `issuer`, `resource` (canonical MCP URI, no fragment, no trailing slash), JWKS URI (from discovery), allowed algorithms and clock skew. A token from another installation fails on issuer, audience and signature.

## Flow A — dashboard login (browser session)

1. Browser requests a protected page; the dashboard backend has no session → redirect to Keycloak authorization endpoint (backend is a **confidential client**; authorization code + PKCE S256; `state`, `nonce`; exact registered redirect URI; discovery via `/.well-known/openid-configuration`).
2. Callback: validate `state`, `iss` (RFC 9207 when present; reject mismatch before redeeming the code), exchange the code at the token endpoint, validate the ID token (issuer, audience = dashboard client, `nonce`, signature, `exp`).
3. Map `(issuer, sub)` to an installation-local `UserId` (below). Create a server-side session: opaque 256-bit random ID stored hashed; cookie `__Host-session`, `Secure`, `HttpOnly`, `SameSite=Lax`, `Path=/`, no `Domain`. Rotate the session ID at login (fixation). Idle timeout and absolute lifetime are configuration; defaults 30 min idle / 8 h absolute.
4. Unsafe methods require a CSRF token (synchronizer token bound to the session; `x-csrf-token` header). `authenticate_browser` validates cookie + CSRF and yields `Principal` with `AuthKind::BrowserSession` and the user's browser scopes (including `approvals:decide`).
5. Logout: delete the server-side session and any delegated tokens held for it (Flow C), clear the cookie, and optionally call Keycloak RP-initiated logout. Logout does **not** revoke external agent grants (Flow B).

The session cookie is never an MCP credential: `validate_mcp_token` rejects it, and `authenticate_browser` rejects bearer tokens (contract-tested in 00-09 mocks; real tests in 02-07).

## Flow B — remote agent authorization (MCP client ↔ Keycloak)

Our service is only the **resource server**.

1. Unauthenticated request to `/mcp` → `401` with `WWW-Authenticate: Bearer resource_metadata="https://<host>/.well-known/oauth-protected-resource/mcp", scope="<needed scopes>"`.
2. Protected-resource metadata (RFC 9728) at that URL and at the root fallback: `resource`, `authorization_servers` (the installation's issuer), `scopes_supported` (minimal baseline set), `bearer_methods_supported: ["header"]`. The metadata URL is derived from the configured public base URL, never from the `Host` header.
3. The client discovers the authorization server (RFC 8414, then OIDC discovery), registers or uses a pre-registered client, and runs authorization code + PKCE S256 with `resource=<canonical MCP URI>` where the server supports it. The user authenticates and consents at Keycloak.
4. The client sends `Authorization: Bearer` on **every** request (tokens never in the query string).
5. Validation in `crates/identity` (`validate_mcp_token`), all must pass or the call fails with `unauthenticated` (HTTP 401) and no `Principal` exists:
   - JWT access token (`typ` `at+jwt` where provided); algorithm from an allow-list (`RS256`/`ES256`/`EdDSA` as configured; `none` and HMAC rejected); `kid` selects a key from the JWKS cache; unknown `kid` triggers one rate-limited refresh;
   - `iss` equals the configured issuer exactly; `aud` contains the canonical resource URI exactly (no prefix or wildcard); `exp`/`nbf`/`iat` with bounded skew (default 30 s); maximum token age bound;
   - `sub` present; `scope` parsed with `Scope::parse` (unknown scopes dropped from authority but never grant anything); `azp`/`client_id` recorded;
   - not revoked and not disabled (see [revocation-and-sessions.md](revocation-and-sessions.md)).
6. Authority: `VerifiedIdentity { user_id: mapped, installation_id, scopes: granted ∩ known, session: SessionRef(from jti), auth_kind: McpToken, authenticated_at: now, expires_at: min(token exp, revocation horizon) }`.
7. Insufficient scope for a tool → `403` with `WWW-Authenticate: Bearer error="insufficient_scope", scope="<all scopes needed for this operation>", resource_metadata=…`. Scope sets per tool: `list_servers` → `inventory:read`; inspection and `get_*` tools → `servers:inspect`; mutating tools → `servers:mutate`; approval requests → `approvals:request`. `approvals:decide` is never issued to agent clients.
8. Per-server ownership and execution policy remain separate checks (Spec 03/06); scopes never grant machine access.

Token passthrough is prohibited: the MCP service never forwards a received token and never accepts a token not issued for its resource.

## Flow C — dashboard backend → MCP service (delegated authority)

Goal: the dashboard's Linux operations reach the MCP service as the logged-in user without a service-wide token or a browser-supplied user ID.

1. The dashboard backend is a Keycloak confidential client with its own client authentication (`private_key_jwt` preferred; client secret supported, held in the deployment secret store).
2. During or after login, the backend requests authorization for the MCP resource (`resource`/audience = canonical MCP URI, scopes limited to what the user's role allows, no `approvals:decide`). Consent is recorded by Keycloak.
3. Tokens are obtained by the backend (code exchange, refresh) and stored **server-side only**, encrypted, bound to `(installation, user, dashboard client, resource, scope set, browser session)`. The browser never sees them.
4. Each MCP call carries the user's delegated access token; the MCP service validates it exactly as in Flow B. The resulting `Principal` has `AuthKind::DelegatedToken`. Refresh before expiry; on refresh failure (revoked, consent withdrawn, user disabled) the delegated authority is dropped and the UI shows "reconnect".
5. A change of logged-in user in the same browser discards the old user's delegated tokens before any new call.
6. A token whose `azp` is the dashboard client is **only** valid as `DelegatedToken`; a token whose `azp` is an external client is **only** valid as `McpToken`. This prevents a stolen agent token from being presented as dashboard authority and the reverse.

Alternative to verify in 02-05 (not assumed): Keycloak standard token exchange (RFC 8693) from the browser session token to an MCP-audience token. If supported and stable it can replace step 2–3's separate grant. Either way no shared service identity impersonates users.

## Authority kinds and revocation independence

| Authority | Created by | Ends when | Independent of |
|---|---|---|---|
| Browser session (`BrowserSession`) | Flow A | logout, idle/absolute expiry, user disabled, admin revoke | agent grants |
| Delegated token (`DelegatedToken`) | Flow C | browser logout (held per session), refresh failure, user disabled, grant revoke | external agent grants |
| Agent grant (`McpToken`) | Flow B, per external client | user revokes grant (dashboard grants page, Keycloak), token expiry, user disabled | browser logout |

Disabling a user ends all three within the revocation bound.

## Subject mapping

Table (design, created in 02-02 under identity-owned namespaced migrations): `identity_accounts(installation_id, issuer, subject, user_id, display_email, email_verified, created_at, disabled_at)` with `UNIQUE(installation_id, issuer, subject)` and `UNIQUE(user_id)`. Rules:

- The ownership key is `(installation, issuer, subject)` → `UserId` (random v4), never the email. Two subjects with the same email are two users; an email change does not change `UserId`; an email is never used to link or take over an account.
- First sign-in creates a user only if signup policy allows it (hosted: configurable, default closed-by-invite until the operator opens it; self-hosted: public signup off, first-admin bootstrap only — see 02-06).
- Unknown `sub` on a token (agent grant for someone not provisioned) → `unauthenticated`; tokens do not create users.
- A new `issuer` is a new account namespace (migrating providers requires an explicit link operation by an administrator, out of scope for v1).
- `disabled_at` set → every authority for that user fails immediately on the next check.

## Claims and configuration expected from Keycloak

Access token: `iss`, `sub`, `aud` (includes the MCP resource URI), `exp`, `iat`, `jti`, `scope`, `azp`/`client_id`, `typ`. Client scopes named exactly as contract `Scope` strings. Access-token lifetime 5 min recommended; refresh-token rotation on for public clients (OAuth 2.1 requirement). Required discovery fields in authorization-server metadata: `issuer`, `authorization_endpoint`, `token_endpoint`, `jwks_uri`, `code_challenge_methods_supported` including `S256`; the resource server logs a start-up error if missing.
