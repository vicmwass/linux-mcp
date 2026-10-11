# Spec 05 dashboard page and adapter architecture

Step 05-01 freezes the dashboard boundaries that later crate and page steps consume. It is an architecture record; it does not create the dashboard crate or claim that a provider is wired.

## Runtime split

Use Leptos with server-side rendering and browser hydration, hosted by Axum. The server build owns authentication, management API calls and the backend MCP client; the browser build owns rendered interaction state and never owns SSH, credentials or provider tokens. Leptos documents SSR as HTML rendered on the server followed by WASM hydration, and recommends the official Axum integration for new applications. `leptos_axum` provides the Axum integration and currently tracks Axum 0.8/Leptos 0.8. The exact locked versions and feature flags (`ssr` on the server, `hydrate` in the browser) must be verified before 05-02 edits the workspace manifest. References: [Leptos SSR and hydration](https://book.leptos.dev/getting_started/index.html), [Leptos SSR integrations](https://book.leptos.dev/ssr/index.html), and [`leptos_axum` documentation](https://docs.rs/crate/leptos_axum/latest).

The intended build produces one native dashboard server and a WASM browser bundle. Server functions or typed backend adapters may cross the boundary, but their inputs are frozen DTOs and their responses contain no secret material. The browser cannot call SSH libraries, read secret files, select an owner, or bypass the MCP service with a Linux-execution REST endpoint.

## Page and route ownership

The shared shell consumes design baseline 0.1.0: login/account state, persistent target selector, navigation, search/filter patterns, timestamps, connection state and operation-result presentation. Feature pages own only their route and typed adapter calls:

| Area | Initial routes | Adapter boundary | Disconnected behavior |
| --- | --- | --- | --- |
| Account | `/login`, `/account` | Identity/session adapter | Login remains available; expired session clears user caches |
| Inventory | `/servers`, `/servers/new`, `/servers/:id/edit` | Management API inventory DTOs | Show cached owned metadata and disable writes |
| Setup | `/installations`, `/installations/:id/grant` | Identity/delegated-authority adapter | Show grant status and revocation, never secret snippets |
| Server detail | `/servers/:id/overview`, `/capabilities`, `/users` | Owner-scoped MCP discovery/read adapter | Show last-known timestamp and unavailable actions |
| Operations | `/servers/:id/operations`, `/approvals/:id`, `/audit` | Prepare/submit, governance and audit contracts | Preserve exact intent/outcome; no blind mutation retry |

Resource modules from Specs 08/09 and monitoring/workflow modules from Specs 10/11 register into the shell through explicit route/component extension points. Unsupported tools are hidden or disabled after discovery rather than rendered as generic command or file fallbacks.

## Adapter and cache rules

The server adapter sequence is: authenticate the browser request into `Principal`; authorize the requested resource; call management API or MCP discovery; validate the pinned schema/version; map the response into a page DTO; and attach collection time plus stale/disconnected state. Every call independently checks ownership and current server revision. Cache keys include principal/installation, server ID, revision and query; logout, account change, token revocation and revision invalidation clear affected entries. A browser session cookie is never accepted as an MCP bearer token.

The dashboard may submit a write-only credential or trust enrollment form to the management API. Secret fields are excluded from hydration, cache, analytics, logs, errors and rendered setup snippets; the form clears after the request. Host key enrollment displays observed versus verified fingerprints and never silently promotes an observation to trust.

## Feature discovery and error presentation

Discovery determines whether a page action is available. Unknown or mismatched schema versions disable only the affected action with a useful explanation. The shell presents disconnected/unavailable, validation/authentication failure, policy refusal, failed-before-submission and unknown-after-submission as distinct states. Remote stdout/stderr/logs are untrusted text and are escaped before rendering. A successful command is not presented as proof of resulting system state; a follow-up observation carries its own timestamp.

The core pages require neither AI nor WebMCP. Agents can receive an MCP grant through the setup page, but only the MCP service and governance contracts can authorize operations. Approval shows the exact target, revision, resolved action, arguments, risk and expiry; `confirm: true` cannot approve an operation and the per-server approval mode is not editable through an agent path.

## Validation gates for 05-02

Before adding manifests or crate code, verify the chosen Leptos/Axum versions offline where possible, confirm separate server/WASM feature graphs, and build a fixture API/MCP server. Acceptance requires that pages render with MCP unavailable, fixture identity cannot enter production settings, and a browser bundle contains no stored-credential markers. Accessibility and responsive checks consume the published 0.1.0 design baseline; later design revisions require an explicit adoption/review record.
