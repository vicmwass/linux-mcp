# Rust dashboard design handoff — v0.1.0

Baseline candidate produced with Stitch and a local synthetic interactive prototype. Implementation baseline publication is recorded in baselines.json. This is design-only: no Rust page, live identity, SSH, database or MCP behavior is implemented. Agent completion is not human approval.

## Start here

Read brief.md, navigation/flows.md, system/tokens.json and system/components.md. Run `python design/client/build_preview.py`, then `python -m http.server 8765 --bind 127.0.0.1 --directory design/client`; open `/preview/index.html`. The page index and state selector demonstrate all 27 screens; review/screen-index.md links each directly. Six Stitch visual explorations are at https://stitch.withgoogle.com/projects/3486587707575590516, with prompts/design-system/screen IDs in stitch.json and links in preview/stitch/index.html.

Use local tokens and semantic/state annotations as requirements. Stitch informs visual hierarchy but its unsolicited crypto/mTLS/live-stream/CLI-builder claims are not accepted behavior. Generated HTML export was unavailable; its remote resource links are retained. The local preview is self-contained. Prototype JavaScript/Python is development-only; production application logic remains Rust/Leptos.

## Composition and ownership

Spec 05 owns shared `AppShell`, `TargetStrip`, `PageHeading`, `Notice`, `StatusBadge`, `FormField`, `SecretProvision`, `ResourceTable<T>`, `Pagination`, `LogView`, `DiffView`, `MetricChart`, `ExactIntentPanel`, `ApprovalDialog`, `OperationResult`, `VerificationPanel` and `AuditRow`. Page modules consume these slots and propose additive changes to their owner. Leptos server functions/backend clients fetch authenticated management APIs for inventory/identity/governance, and MCP for Linux operations. Browser code receives only public view data; never SSH/sudo saved secrets.

Token JSON v0.1.0 is authoritative. Translate CSS variables directly; use system font fallback, 4px spacing scale, 8px radius and >=44px controls. Breakpoints: <=640 single column/mobile drawer; 641–1024 narrow rail/stacked evidence; >1024 full rail/content/evidence. Tables/logs scroll internally, paragraphs/identifiers wrap, target strip remains sticky. Separate operator/install and remote identity. Read-only/unsupported capabilities never acquire mutation buttons from generic shared states.

| Screen IDs | Implementation owner / step |
|---|---|
| shell, components, target context | 05-02 |
| login / logout / expiry | 05-03 |
| inventory, server-form, host-trust | 05-04 |
| capabilities, users, operations | 05-05/06; Users extension 08-08 |
| agent-setup, approval, audit | 05-07 |
| overview | 08-03 |
| services | 08-05 |
| logs | 08-07 |
| processes / packages / files | 09-04 / 09-08 / 09-12 |
| storage / network / jobs / access | 09-16 / 09-20 / 09-24 / 09-28 |
| monitoring, schedules, alerts | 10-07 |
| troubleshoot, fleet, maintenance | 11-06 |

The coordinator still enforces each registered prerequisite. Publication of design does not bypass contracts, adapter or live-integration gates. No implementation work outside Spec 12 was performed here.

## Provisional data needs (not wire schemas)

- Shell: authenticated operator/install label, owned server choices, selected server ID/revision/login/address, host trust, connection/schema compatibility and collected time.
- Resources: typed bounded rows, stable identity, permitted actions with reasons, cursor/count/truncation, observation/source time and explicit missing fields. Module-specific process start identity, file revision, package version/outcome, schedule timezone/user and key/grant metadata.
- Intent: immutable target/revision, exact tool/action/arguments, requester, risk/privilege, expiry and backend operation/approval reference. Target/account change clears view intent; backend validates authority/revision and durable human approval again.
- Result: submission stage/ID/time, bounded stdout/stderr/exit, definite refusal/failure vs unknown, separate verification status/evidence/time. Unknown blocks replay; inspection precedes a fresh intent. Workflow state binds every resolved step and persists ambiguity.
- Monitoring: metric/unit/interval/window, per-sample observation/missing/reset, history revision, collection grant/expiry/status, quota hints, alert threshold/window/breach/recovery/unavailable. Fleet reports per-target results/times and partial counts.

Contracts owners choose actual DTOs, IDs, provider protocols and retention/budgets. Synthetic strings, limits and timestamps are examples; do not invent alternate wire contracts from these JSON fixtures. Confirm supported capability/schema versions before enabling a feature.

## Acceptance for Rust implementers

Pin baseline `0.1.0` and manifest checksum. Compare shell/layout/tokens at 1440, 834, 390 and 320px; verify page reflow plus internal table/log scroll, keyboard focus/skip/error/dialog/menu flow, non-color states and accessible chart alternative. Re-run contrast checks after any theme change. Production tests must additionally cover real authorization/isolation/cache clearing, secret redaction, trust enrollment, target/revision invalidation, approval expiry, disconnect-after-submit reconciliation, bounded escaping and actual post-change verification. Use disposable targets only for live mutation tests.

Review report: review/report.md; measurements: review/responsive.json and contrast.json; findings: review/findings.md. Prototype limitations include simulated transitions/finite pagination, no actual provider/SSH/MCP, no assistive-technology/device certification and unavailable original Stitch source exports. No unresolved critical design finding is accepted.

## Review and adoption

All steps can be reviewed/reopened with the coordinator; preserve previous evidence and revalidate dependent acceptance. For a nonblocking redesign, follow specs/plans/client-design-revisions.md and create a versioned draft/adoption plan. Keep published v0.1.0 snapshot and checksums. Implementers stay pinned until adoption is explicitly assigned; drafts do not alter active workers' files or dependencies. Publishing a design baseline never invents human approval.
