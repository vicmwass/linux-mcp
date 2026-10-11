# 02 — Identity and authorization

Implementation: [step-by-step plan and dependencies](plans/02-identity-implementation.md). Follow the [coordination workflow](plans/README.md).

## Deliverable and ownership

Own `crates/identity`, identity-provider integration, dashboard sessions and MCP token verification. Build independently against contract mocks. Select a maintained standards-based provider before implementation; do not create a custom OAuth protocol.

## Requirements

- Support dashboard login/logout, secure HTTP-only cookies, CSRF protection and expiration. Hosted signup policy is configurable; self-hosted setup bootstraps an administrator and disables public signup by default.
- Map stable provider subjects to installation-local user IDs; email alone is not the ownership key.
- Remote MCP uses OAuth authorization and discovery appropriate to the current specification. Validate issuer, intended resource/audience, signature, lifetime and scopes. Verify compatibility with explicitly named target MCP clients during implementation.
- Produce a trusted Principal for each request. Dashboard cookies do not automatically authorize an external agent. Never accept identity from a server ID, URL parameter, tool argument or MCP session ID.
- Scopes distinguish inventory read, inspection and mutation. All scopes remain subject to per-server ownership and execution policy.
- Bind MCP sessions to the authenticated subject; reject cross-user session reuse. Revalidate authorization on requests, including reconnects and long-lived session operations. MCP revision 2026-07-28 removes protocol sessions, so identity is always derived from the bearer token per request and a session ID is only a cache key bound to the subject where a transport still has one; see [design/02](design/02/README.md).
- Dashboard backend forwards user authority through an explicitly supported delegated flow. A shared service token plus a browser-supplied user ID is prohibited. Browser sessions and delegated agent tokens have independent revocation behavior.
- Define bounded revocation latency and test it. Initial target: revoked users/tokens lose new-operation access within 60 seconds. Already submitted SSH operations cannot necessarily be canceled.

## Acceptance

Expired, wrong-audience, wrong-issuer, insufficient-scope, revoked and malformed credentials fail. User A cannot attach to B's MCP session. Logout removes the browser session; agent grants can be separately revoked. CSRF attempts fail. Two isolated self-hosted installations reject each other's tokens. Demo includes a test issuer and verified authorization flow; mock tokens are never accepted in production configuration.
