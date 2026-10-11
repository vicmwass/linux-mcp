# Spec 02 — Identity and authorization implementation

Source: [02](../02-identity-and-authorization.md). Registry: `02-identity.steps.json`. Follow [shared rules](IMPLEMENTATION-RULES.md). Own `crates/identity/`, `specs/design/02/` and namespaced identity migrations. Checks: format, `cargo test -p linux-mcp-identity`, disposable provider/database integration, browser/session security and real token validation. No live user secrets in fixtures.

## 02-01 — Select provider and authorization flows

Dependencies: `00-contracts/00-12`. Resources: `identity-design`.

1. Verify current MCP OAuth/discovery requirements and compatibility with named target clients using official sources.
2. Choose a maintained self-hostable provider and Rust integration; define dashboard login, remote-agent authorization and backend user-delegated MCP flow.
3. Document issuer/audience/scopes, stable subject mapping, session ownership, logout versus grant revocation and the 60-second revocation target.

Acceptance: runnable provider choice and exact flows cover hosted and independent installations; no custom OAuth protocol or shared global token impersonation. Unsupported client needs are explicit blockers/limitations.

## 02-02 — Bootstrap identity storage and test issuer

Dependencies: `02-01`. Resources: `workspace-manifests`, `migration-registry`, `identity-bootstrap`.

1. Add crate/config, namespaced account/session/grant mappings and external provider configuration.
2. Build disposable provider/test-issuer setup, deterministic clock and public/private key fixtures with no real credentials.
3. Add module registration placeholders and feature boundaries for production versus test adapters.

Acceptance: migrations, key discovery setup and crate checks pass; production rejects the mock/test issuer unless explicitly configured for a test environment.

## 02-03 — Implement browser sessions and account mapping

Dependencies: `02-02`. Resources: `identity-browser`.

1. Implement login/callback/logout and provider-subject to local-user mapping in `src/browser/`.
2. Enforce secure HTTP-only cookies, CSRF/state/redirect validation, session fixation protection and expiry.
3. Test duplicate email/different issuer subjects, logout, cookie flags and malicious callback/CSRF attempts.

Acceptance: browser auth tests pass; email cannot transfer ownership; session data never leaks through URLs/logs.

## 02-04 — Implement MCP token verification and discovery

Dependencies: `02-02`. Resources: `identity-mcp`.

1. Implement issuer/signature/key rotation, audience/resource, lifetime and scope validation in `src/mcp/`.
2. Provide provider/resource metadata and challenges through contract adapters, with restrictive production defaults.
3. Add wrong-issuer/audience/algorithm, malformed/expired token, key-rotation and insufficient-scope tests.

Acceptance: invalid authority cannot construct Principal; metadata matches actual deployment endpoint; authorization is checked independently of MCP session IDs.

## 02-05 — Implement delegated dashboard authority

Dependencies: `02-03`, `02-04`. Resources: `identity-delegation`.

1. Implement the provider-supported user-delegated flow selected in 01 for dashboard-to-MCP calls.
2. Secure token storage/refresh and bind grants to owner, resource, scope and installation.
3. Test consent failure, refresh expiry, cross-account browser changes and token leakage boundaries.

Acceptance: delegated MCP tokens carry the correct user authority; browser-supplied IDs and service-wide tokens cannot impersonate users.

## 02-06 — Implement revocation and bootstrap policy

Dependencies: `02-03`, `02-04`. Resources: `identity-lifecycle`.

1. Implement bounded token/session revocation checks, cache invalidation and disabled-user behavior in `src/lifecycle/`.
2. Add self-hosted first-admin setup and configurable hosted signup policy; public signup off by default for self-hosting.
3. Test multi-instance cache behavior, logout independence and access loss within 60 seconds.

Acceptance: no new operation beyond the documented revocation bound; restart preserves revocations and bootstrap cannot be reused to take over an initialized installation.

## 02-07 — Compose adapters and auth middleware

Dependencies: `02-05`, `02-06`. Resources: `identity-exports`, `shared-routing`.

1. Export Identity adapter and trusted request context; wire HTTP middleware without owning MCP handlers or dashboard pages.
2. Check long-lived request/reconnect/session subject binding with fixture MCP consumers.
3. Verify cookie and bearer paths cannot substitute for one another or bypass scope checks.

Acceptance: consumer probes use real provider-issued tokens; identity crate contains no server inventory ownership shortcuts.

## 02-08 — Validate two installations and publish handoff

Dependencies: `02-07`. Resources: `identity-integration`.

1. Run provider/browser tests and negative-token suite for Alice/Bob on two isolated installation configurations.
2. Verify cross-installation token rejection, independent revocation and secure production configuration.
3. Publish `specs/handoffs/02.md` with provider setup, delegated-flow examples, exact tested client assumptions and limits.

Acceptance: Spec 02 identity criteria pass; end-to-end real MCP-client compatibility remains a 04 gate so this plan cannot deadlock on 04.
