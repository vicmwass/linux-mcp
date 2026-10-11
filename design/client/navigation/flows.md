# Information architecture and walkthroughs

Global (installation/account scope): Login; Servers; Agent connection; Operations & audit; Monitoring; Alerts; Workflows. Remote scope under `/servers/:id`: Overview; Capabilities; Users; Services; Logs; Processes; Packages; Files; Storage; Network; Jobs; Access. Proposed routes are UI suggestions, not API contracts. The prototype's query links use screen IDs.

Account/install context is always separate from remote target. `alex@example.test · Demo self-hosted install` identifies the operator; `app@prod-web-01 · 192.0.2.10 · r7` identifies the machine. Hosted edition changes the install label, not owner visibility. Only authenticated owned inventory is selectable. No team selector. Logout clears account/target caches in production.

Capability discovery supplies navigation availability and action-specific reasons. Unknown schemas disable affected actions. Navigation does not invoke shell fallbacks. Inventory and historical pages remain usable on MCP loss; cached observations show time, live controls disable. URL possession never grants access; external agents authorize to the installation MCP endpoint independently.

## Walkthroughs

1. Add server: Servers → Add → hostname/port/login/environment → new write-only credential → Save → field cleared → unverified observed fingerprint. Errors remain associated with fields; a revision conflict offers reload/compare, never overwrite silently.
2. Trust: independently compare fingerprint via a separate trusted channel → record verification → fresh capability query. “Observed” alone never unlocks execution. Rotation requires renewed verification.
3. Service investigation: select owned target → Services → failed `nginx.service` → Logs filtered by service/time → bounded escaped output and continuation. Unsupported init/log backend shows reason.
4. Change: supported action → exact intent preview (owner/install/target/revision/service/action/args/risk/expiry) → human approval → submit once → running → exit result → independently observed verification. Target switch invalidates preview; expiry forces fresh preflight. Protected account actions explain specific impact.
5. Unknown: disconnect after submit → Outcome unknown → no retry button → read-only inspect/reconcile → verified/mismatch/unavailable → fresh preflight only after reconciliation. Never promise rollback or remote cancellation.
6. Agent setup: copy public endpoint → authorize agent → choose owned scope → inspect grant expiry → revoke. Expired/revoked grants need reauthorization; URL never contains saved secrets.

Keyboard: skip link → global navigation → target selector → page heading → filters → table row actions → next page. Dialog traps focus, Escape closes before submission, and returns focus to trigger; pending operations remain recorded after dismissal. Tablet collapses rail; mobile opens a labeled menu and stacks evidence below intent. Target strip stays visible when scrolling actions.
