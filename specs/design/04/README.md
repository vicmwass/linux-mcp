# Spec 04 transport and adapter design

Step 04-01 freezes the assumptions that the later `crates/mcp-service` steps consume. This is a design record, not a transport implementation or an authentication claim.

## Selected transport

The repository currently pins `rmcp = 3.2.0` with `server` and `transport-io`; that configuration supports the existing stdio path only. The 3.2.0 feature catalogue exposes `transport-streamable-http-server` for the server transport and `auth` for OAuth 2.0 helpers. Before 04-02 changes the workspace manifest, the implementer must verify the exact locked API and add only the features required by the selected HTTP framework. Reference: [rmcp 3.2.0 feature and transport documentation](https://docs.rs/crate/rmcp/3.2.0).

The target endpoint is one POST-capable `/mcp` endpoint using Streamable HTTP. The current MCP draft requires `Origin` validation, authentication, `Accept: application/json, text/event-stream`, and a response that is either JSON or request-scoped SSE. The 2026-07-28 revision removes protocol-level `Mcp-Session-Id` sessions and the initialize handshake; application state must use an explicit server-minted handle. Legacy initialize/session behavior may be supported only as a separately documented compatibility mode. Reference: [MCP Streamable HTTP specification](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/draft/basic/transports/streamable-http.mdx).

## Request and response policy

Every POST is authenticated before dispatch and carries the verified `Principal` supplied by the identity adapter. The session identifier, if presented by an older client, is only a lookup key and never an authority or owner selector. Owner, server ID and credential references come from the authenticated context plus the frozen contracts; user-supplied owner fields are rejected.

Tool discovery is deterministic and contains no secrets. Inventory reads return only the principal's servers. Mutations use the versioned `prepare_operation` and `submit_operation` extension fixtures. `confirm: true` is intent, never human approval. A target's current `approval_mode` determines whether a medium-risk operation requires approval (`medium_and_above`, default) or only high/critical risk does (`high_and_above`); policy denials remain unconditional. Approval-required results contain a stable error code, operation/approval reference and dashboard URL, never a credential or bearer token.

The adapter maps contract outcomes as follows: refused policy/authorization stays `refused`; validation or dependency failure before dispatch is `failed_before_submission`; an accepted remote request whose result is unknown is `unknown_after_submission`; successful execution is `succeeded`. Internal provider, SQL, SSH and token-validation details are redacted behind the frozen structured error envelope and request ID.

## Limits and client test procedure

04-02 must make these limits configuration values with conservative defaults and tests: maximum body bytes, maximum JSON depth, maximum response bytes, maximum request duration, maximum SSE event size, and per-principal concurrent requests. The endpoint must reject invalid `Content-Type`, unsupported protocol versions, bad/missing required request metadata, invalid origins and oversized bodies before tool execution. Closing a request's SSE stream is cancellation; an already-submitted remote mutation must be recorded as unknown and never replayed.

The interoperability test records client name/version, protocol version, transport mode, authorization method, endpoint URL shape and every observed status/outcome. The minimum matrix is: initialize/discovery (or modern first POST), list servers for Alice and Bob, foreign ID by bypassed discovery, approval-required prepare/submit, expired/revoked token, reconnect after transport close, malformed origin/header/body, and an unknown-after-submit fault. Fixture tests are useful for 04-02 but cannot satisfy the real-client gate in 04-07.

## Deferred decisions

The exact HTTP framework, OAuth provider, TLS termination and state store remain implementation choices for 04-02/04-03. Changing frozen tool schemas or outcome meanings requires the contracts owner; adding a compatibility route must be explicit and must not weaken the modern `/mcp` authorization boundary.
