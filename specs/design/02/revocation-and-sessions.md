# Spec 02 revocation, sessions and MCP-revision handling

## MCP sessions across revisions

| Revision | Sessions | Our rule |
|---|---|---|
| 2025-11-25 | `Mcp-Session-Id` optional on Streamable HTTP; authorization still required on every HTTP request | Authority comes from the bearer token on every request. If Spec 04 issues a session ID, it is an opaque cache key bound at creation to `(issuer, sub, azp)`; a request whose token subject differs is rejected (`forbidden`, session closed). A session never raises or extends authority. |
| 2026-07-28 | Removed (changelog verified): no `Mcp-Session-Id`, no `initialize`; per-request version/capabilities in `_meta`; no stream resumption, so a retry is a new request and the token must be valid then | No session state exists. Identity is re-derived per request; cross-call state uses server-minted handles owned by the principal (Spec 04), never identity. |

Consequence for Spec 02's original requirement ("bind MCP sessions to the subject; reject cross-user reuse"): it stays true for 2025-11-25 transports and becomes vacuous (by construction) for 2026-07-28. Tests (02-07) must cover both: cross-user session reuse rejected where sessions exist, and identical behavior when no session exists. A session ID, URL parameter, tool argument or server ID is never an identity source.

## Revocation design (target ≤ 60 s for new operations)

What can end authority: user disabled; browser session logout/expiry; delegated-grant revocation; agent-grant revocation; Keycloak session invalidation; admin action through our `Identity::revoke`.

Mechanisms, layered:

1. **Short access tokens** (≤ 5 min) bound the worst case for anything not otherwise detected.
2. **Local revocation list** in the identity database (`revoked_sessions`, `revoked_users`, keyed by `jti`/session ref and user): written by `Identity::revoke`, by the disabled-user flag, and by the Keycloak notification path below. Checked on every request through a cache with ≤ 5 s TTL and a database notification (or short poll) to invalidate other replicas.
3. **Keycloak events**: OIDC back-channel logout endpoint (signed logout token validated like an ID token) records the session/subject as revoked; admin events for user disable/delete are consumed where the realm is configured to emit them.
4. **Introspection backstop** (RFC 7662) with a per-`jti` cache TTL of 30 s: a token not confirmed active within the last 30 s is re-introspected before use. A revocation done only at Keycloak is therefore visible within ≤ 30 s plus skew.
5. **Fail closed**: if introspection and the revocation store are both unreachable and a token's last confirmation is older than 60 s, the request fails with `unavailable` (HTTP 503) and nothing is submitted. A stale allow is never served.

Worst case: 30 s introspection cache + 5 s revocation cache + clock skew (≤ 30 s) < 60 s from revocation to the first rejected **new** operation. An operation already submitted over SSH may complete; revocation cannot recall it and the audit record keeps the outcome. Restart preserves revocations (they live in the database). Tests in 02-06 must exercise the bound with a controllable clock and a real Keycloak (or the disposable provider from 02-02) including multi-replica behavior.

## Logout versus grant revocation

- Browser logout ends the browser session and its held delegated tokens (Flow C) but leaves external agent grants alone.
- Revoking an agent grant ends only that client's tokens for that user; the browser session and other clients continue.
- Disabling the user ends everything.
- Independent revocation is verified by tests that perform each action and assert the other two kinds are unaffected.

## Replay and theft hygiene

- Bearer tokens, session cookies, CSRF values, refresh tokens and authorization codes never appear in logs, errors, audit or URLs; `BearerToken`/`BrowserCredentials` already redact `Debug`.
- Refresh tokens (delegated) are stored encrypted with a key outside the database, like Spec 01 secrets.
- Authorization codes are redeemed only after `state` and `iss` checks; mismatched `iss` responses are not acted on or displayed.
- JWKS cache: bounded size/age, refresh on unknown `kid` at most once per 30 s, never accepts keys from the token itself (`jku`, `x5u`, embedded `jwk` headers rejected).
- Clock source is injected (deterministic in tests).
