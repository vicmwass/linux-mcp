# Linux MCP console — design brief v0.1

Audience: individual Linux operators using a hosted account or an independently self-hosted installation. One owner per server; no shared/team inventory in this release. Tasks: connect a machine safely, inspect services and logs, approve an exact change, verify its result, connect an external agent, monitor and investigate bounded workflows.

Visual direction: a focused dark infrastructure console, slate surfaces, cyan primary actions, restrained amber attention and red failures. Inter/system sans for UI; system monospace for fingerprints, commands, paths and logs. Use a compact 232px navigation rail, generous page headings, 44px controls and readable 14–16px table/body text. Working name “Linux MCP”; no brand/logo assumption. One default dark theme; no light-mode commitment.

Always distinguish installation/account identity from remote target/login identity. Persistent target strip: `prod-web-01 · app@192.0.2.10 · revision 7`, trust badge, MCP connectivity and collected-at time. Documentation addresses and fake fingerprints only. No real inventory, credentials or output is sent to Stitch.

Desktop 1440px: sidebar, target strip, content with optional evidence panel. Tablet 834px: narrow collapsible rail and stacked evidence. Mobile 390px: menu button, sticky target strip, single column, explicit row-detail links, internally scrollable logs/tables. Also verify 320px reflow. Keyboard-first controls, visible focus, text status labels, reduced motion and WCAG 2.2 AA review target. This is a prototype target, not a compliance certification.

Safety language: observed key is unverified; URL alone grants nothing; secret provision/rotation is write-only. Submitted/exit-success/observed-verification are separate states. Disconnect after submit means “Outcome unknown — inspect before retry”. Target/revision change invalidates previews. Password lock leaves keys and sessions active. Storage starts read-only. No generic rollback/cancel promises. Capability absence disables the affected feature with a reason.

Stitch is authorized design tooling; local artifacts are the implementation source of truth. Prototype interactions are simulated and do not perform SSH, identity, approvals or MCP calls. Production client application logic remains Rust/Leptos. Default retention/sample limits are examples until contracts and policy owners choose actual values.

Sources: specs 05, 08, 09, 10, 11, 12 and LINUX-MANAGEMENT-CLIENT-PLAN.md. Unresolved provider, schemas, supported distributions and limits do not block design; page data needs are provisional.
