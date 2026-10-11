# Spec 12 — Independent client design implementation

Source: [12](../12-client-page-design.md). Registry: `12-client-design.steps.json`. Follow [shared rules](IMPLEMENTATION-RULES.md) and [coordination](README.md). Status: steps 12-01–10 completed; 0.1.0 published and reviewable/reopenable. Live coordinator state remains authoritative.

Open design request (2026-10-11): add the per-server approval requirement control to the inventory add/edit form and approval screen; see `design/client/review-requests/approval-mode.md`. Integrate it through the revision workflow in [client-design-revisions.md](client-design-revisions.md) (supplemental revision before first publication, or a later revision); do not edit published or in-review baseline artifacts in place.

Own `design/client/`, `specs/implementation-evidence/12-*.md` and `specs/handoffs/12.md`. No prerequisite backend or contract build. Do not edit `crates/`, Cargo files, API schemas or shared routers. Use synthetic examples, a local preview, and the user-authorized dedicated Stitch project (2026-10-11). Keep remote screen references and generation prompts in the handoff; Stitch is design tooling, not a product runtime dependency. Standard checks: artifact/link/token consistency, rendered desktop/tablet/mobile inspection, critical keyboard/focus flows, measured contrast and screen/state coverage. No Rust test suite is needed for design-only assets. Inspect applicable design/browser tool instructions when actually producing/reviewing the preview.

Module ownership below allows parallel page families. Foundations own shared tokens/components. Screen workers reference shared assets and submit change requests rather than editing them. Each step records outputs, actual review results and limitations before completion. The final gate is a completed design deliverable, not a claim of backend implementation or human approval.

## 12-01 — Establish design brief and coverage map

Dependencies: none. Resources: `client-design-brief`.

1. Read product roadmap and specs 05/08–11, extracting screen families, tasks, safety copy and hosted/self-hosted context.
2. Write `design/client/brief.md` with visual direction, default theme, density, responsive/accessibility targets and explicit assumptions without waiting for backend schemas.
3. Write `design/client/coverage.md` listing screens, states, module ownership and acceptance evidence needed for freeze.

Acceptance: every Spec 12 screen family is assigned; no dependency on backend/agent/tool/provider availability is introduced; unresolved preferences have reasonable documented defaults.

## 12-02 — Define navigation and page wireframes

Dependencies: `12-01`. Resources: `client-design-navigation`.

1. Write information architecture and flows in `design/client/navigation/`: login, target selection, console navigation, agent setup and operations.
2. Produce annotated low-fidelity desktop/tablet/mobile wireframes with persistent target/install/user context and capability gating.
3. Walk through add-server, verify-host-key, inspect-service/logs, approve-action and unknown-outcome reconciliation using synthetic states.

Acceptance: route/screen map and flows cover requirements; designs never imply other users' machines are selectable or unavailable features execute via fallback.

## 12-03 — Define tokens, components and preview scaffold

Dependencies: `12-02`. Resources: `client-design-system`.

1. Create versioned tokens, CSS variables, component inventory and interaction specifications in `design/client/system/`.
2. Create a local preview shell/gallery in `design/client/preview/` with documented extension points and synthetic fixtures.
3. Define names/usage for status, forms, table/filter/pagination, approval/result, log/diff/chart and navigation components; measure initial contrast and focus behavior.

Acceptance: page workers have a concrete reusable visual reference; semantic tokens handle refused/failed/unknown distinctly, and shared assets have one owner. The scaffold has no Rust/backend dependency.

## 12-04 — Design account, inventory and agent setup screens

Dependencies: `12-03`. Resources: `client-design-inventory-pages`.

1. Build reviewable annotated mockups in `design/client/screens/inventory/` for login, inventory CRUD, write-only secrets, fingerprint enrollment and MCP connection/grants.
2. Include empty/loading/unauthorized/disconnected, revision conflict, invalid input and expired/revoked authorization states.
3. Review target/account context, form keyboard flow and mobile layout; reference shared tokens/components without editing them.

Acceptance: secret values are never displayed as saved credentials; observation is distinguished from verified trust; agent URL possession alone does not imply authorization.

## 12-05 — Design console, approval and operation screens

Dependencies: `12-03`. Resources: `client-design-console-pages`.

1. Build annotated Overview/Capabilities/Users/Services/Logs and approval/audit/result mockups in `design/client/screens/console/`.
2. Include exact-action preview, expiry, policy refusal, progress, truncation, verification mismatch/unavailable and unknown-after-submit reconciliation.
3. Review service/log investigation flow, protected-account warnings, timestamp/stale data and escaped hostile-looking output examples.

Acceptance: command success and verified state are visibly distinct; no blind retry/guaranteed rollback promise; target changes cannot leave a silently valid old approval preview.

## 12-06 — Design administration screen families

Dependencies: `12-03`. Resources: `client-design-administration-pages`.

1. Build representative annotated Processes/Packages/Files/Storage/Network/Jobs/Access layouts in `design/client/screens/administration/`.
2. Apply shared patterns while showing PID identity, package partial outcome, file diff/revision, read-only storage, connectivity risk, timezone/OS identity and access privilege impact.
3. Cover unsupported capability, stale resource, guarded action and unknown outcome for each family using a state matrix.

Acceptance: all seven module families are covered without implying undeclared mutation support; navigation/actions follow capability availability and existing safety boundaries.

## 12-07 — Design monitoring and workflow screens

Dependencies: `12-03`. Resources: `client-design-monitoring-workflow-pages`.

1. Build annotated metrics/history/schedules/alerts and troubleshooting/fleet/maintenance/progress layouts in `design/client/screens/monitoring-workflows/`.
2. Show units, gaps, sample times, grants, partial fleets, per-step human approval and restart/reconciliation states.
3. Keep optional AI/browser-agent extensions as clearly optional slots; test that deterministic navigation/tasks remain complete without them.

Acceptance: missing metrics cannot look like healthy zero; workflow label cannot approve dynamic steps; no optional-provider dependency blocks design completion.

## 12-08 — Assemble responsive preview and visual QA

Dependencies: `12-04`, `12-05`, `12-06`, `12-07`. Resources: `client-design-system`, `client-design-preview`, `client-design-review`.

1. Assemble all screen modules into the shared local preview with a screen/state index and representative transitions; preserve each module's ownership.
2. Render/review desktop, tablet and mobile widths, keyboard/focus/error flows, contrast, non-color status cues, reduced motion and bounded content.
3. Save screenshots/measurements/findings in `design/client/review/`; resolve shared-system issues and route page changes to owners before completion.

Acceptance: no critical coverage/safety/accessibility defect remains; review evidence identifies actual viewports/tools and simulated behavior, not a production-compliance claim.

## 12-09 — Prepare Rust component and page handoff

Dependencies: `12-08`. Resources: `client-design-handoff`.

1. Write `design/client/handoff.md`: token/component names, responsive/state behavior, proposed Leptos composition and screen-to-spec/step mapping.
2. Record data needs as provisional view models and map them to known requirements; unresolved wire/schema choices remain the contracts owner's responsibility.
3. Specify implementation checks for pixel/layout consistency, accessible interaction and future additive components; list accepted assumptions and limitations.

Acceptance: independent page agents can implement without inventing their own visual system; handoff does not require Leptos or backend code to be compiled here.

## 12-10 — Publish versioned design baseline and handoff

Dependencies: `12-09`. Resources: `client-design-handoff`, `client-design-release`.

1. Trace each Spec 12 requirement to screen/component/state and review evidence; resolve critical open findings.
2. Publish version/checksums, local preview instructions, `design/client/baselines.json` and `specs/handoffs/12.md`; distinguish reviewed design, simulated interactions and future production validation. Identify the implementation baseline without inventing human approval.
3. Verify all design step evidence and coordinator completions; publish the review/revision workflow and downstream impact/adoption instructions. Completion releases editing reservations; later changes use fresh tracked revision steps and preserve this baseline's evidence.

Acceptance: complete reviewable design package and developer handoff exist; only then may the dashboard bootstrap design gate unlock. No agent completion is represented as a user approval that was never given.
