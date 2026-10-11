# 12 — Independent client page design

Status: version 0.1.0 is published under `design/client/`, with six Stitch references and 27 local screens. The baseline remains reviewable/reopenable; later changes use the revision workflow and do not imply human approval. Implementation: [step-by-step plan](plans/12-client-design-implementation.md). Follow the [coordination workflow](plans/README.md).

## Objective and boundaries

Define a coherent visual and interaction design for the Rust management dashboard before page implementation. This work has no dependency on Spec 00, any backend service, authentication provider, database or running Linux machine. Use synthetic data and documented product requirements; provisional UI view models are design examples, not frozen backend/wire contracts.

Own `design/client/` and the design handoff. Do not change Rust implementation, Cargo files, API contracts or deployment. Spec 05 implements the agreed visual system in Leptos; Specs 08–11 reuse it for feature pages. Browser-compatible HTML/CSS mockups may serve as local design artifacts without changing the Rust application-language requirement. On 2026-10-11 the user authorized Stitch for these page designs. Use a dedicated Stitch project with synthetic data, record its screen/design-system references, and retain a local reviewable package. Publishing a hosted application, generated raster assets unrelated to screen previews, or starting production UI implementation remains outside this spec.

## Required design outputs

- Product/design brief: audience, primary tasks, information density, terminology, visual direction and explicit assumptions. Default to a professional infrastructure console with clear status, readable logs and a persistent visible target; no branding preference blocks initial design work.
- Information architecture: screen inventory, navigation map, target selection, account/install context, page layouts and permission/capability visibility rules.
- Design tokens in a documented machine-readable format and CSS variables: semantic colors, typography, spacing, sizing, borders, radii, elevation, focus, motion and responsive breakpoints. Choose one fully specified default theme; alternate themes are optional, not promised.
- Reusable component specifications: shell/navigation, target selector, page heading, cards, searchable/paginated tables, forms, secret inputs, dialogs, tabs, badges, notices, empty states, loading/progress, charts, log/code views, diffs, operation results and approval panels.
- Reviewable synthetic-data mockups at desktop, tablet and mobile widths, including critical interaction states and representative navigation. Provide a local preview entry point, screen/state index and screenshots/evidence for review.
- Screen annotations for component usage, keyboard/focus behavior, validation, responsive layout and source data needs. UI copy distinguishes policy refusal, definite failure, unverified success and unknown remote outcome.
- Design review findings, resolved issues, artifact version/checksums and an implementation handoff with traceability to each planned page/module.

## Screen coverage

| Screen family | Required coverage |
|---|---|
| Account and agent connection | Login/logout, session expiry, self-hosted setup context, MCP URL/authorization/grants/revocation |
| Inventory | List/search/select, add/edit/delete, per-server approval requirement setting ("Medium risk and above" default / "High risk and above"), credential write-only provisioning/rotation, host-key observation and verification, revision conflict |
| Server console | Overview, capabilities, Users, Services and Logs |
| Operations | Exact-action preview, human approval, waiting/running state, refusal, bounded output, observed verification, unknown outcome and reconciliation |
| Administration | Processes, Packages, Files, Storage, Network, Scheduled Jobs and Access; capability-specific navigation and guarded actions |
| Monitoring | Metrics/history with gaps/units/times, schedules/grant state, alert rules and inbox |
| Workflows | Troubleshooting evidence, fleet partial results, saved maintenance, per-step approvals and resumable progress |

Later administration/monitoring/workflow pages need annotated representative layouts and exceptional states, not a separate polished mockup for every parameter combination. Optional embedded AI/WebMCP can have reserved extension points, but must not block the deterministic design freeze.

## State and safety requirements

Every page family defines loading, empty, ready, validation/error, unauthorized, unsupported capability, disconnected and stale-data states where applicable. Mutations also define preview, approval-required/expired, submitting/running, refused, failed-before-submit, unknown-after-submit and verified/mismatch/unavailable verification.

The per-server approval requirement is a two-option control in the add/edit server form: default "Medium risk and above", alternative "High risk and above". The design shows examples of operations that run without approval under each option, a clear warning for the weaker option, the effect that saving bumps the revision and invalidates pending approvals, and a read-only state with an "interactive session required" explanation when the page is not in a human browser session. The approval screen shows the requirement in effect. See the open request [`design/client/review-requests/approval-mode.md`](../design/client/review-requests/approval-mode.md); it is integrated through the revision workflow and is not claimed as designed until a baseline includes it.

Persist the visible target while reviewing actions; target changes invalidate stale previews. Approval shows the exact stored target/action/arguments and expiry. A UI click or `confirm: true` alone is not approval evidence. Do not claim generic rollback or remote cancellation guarantees. Read-only storage remains visually read-only. Password lock wording does not imply SSH-key revocation or session termination. First observed host key is not trusted until independently verified. Secret inputs are write-only with no stored-value reveal; mocks use synthetic placeholders only.

Remote content is rendered as untrusted text. Logs/charts/lists specify bounded visible data, truncation and pagination; unavailable observations are not healthy zeros. Layouts distinguish logged-in user/install identity from target machine/user identity.

## Review, revision and acceptance

Use WCAG 2.2 AA as the implementation review target: verify text/UI contrast, keyboard/focus flow, accessible names/error association, non-color status cues, reduced motion, reflow and usable touch targets in the prototype. This is a design/prototype review, not certification of the later production app. Record measured widths/contrast and known limitations rather than claiming untested compliance.

The handoff must include all required screen families, a component/state matrix, traceable acceptance evidence and a versioned local preview. Backend-dependent behavior is labeled simulated. Unresolved critical safety/accessibility/coverage defects block release of that baseline. Cosmetic preferences may be documented as assumptions and refined through versioned revisions; a user response is not required merely to start design work. Any explicitly requested approval later remains a separate human decision, never inferred from agent completion.

Completion publishes one reviewable design baseline; it does not permanently lock this spec, its mockups or visual decisions. Locks protect only an active editing step and are released on completion. A user can request review, changes or a full redesign at any time. Preserve the previous baseline and completed evidence, then track the new work as a revision with fresh claimable steps. Never erase completed history or pretend old validation covers changed designs.

Use the [revision workflow/template](plans/client-design-revisions.md). Track feedback as open/addressed/deferred findings with rationale and affected screens/components. Store revision artifacts separately and maintain `design/client/baselines.json` recording version, artifact path, review evidence, publication state and current implementation baseline. Initial states are draft, in-review, published and superseded; none means human approval unless explicitly recorded.

Implementation agents pin the published baseline they consume. A draft redesign does not silently invalidate their in-progress work. Publication of a revision includes an impact/adoption plan for existing components/pages and any required new tracked implementation steps. If the user explicitly requires implementation to wait for a review, record a blocker or additive dependency on still-pending affected steps; do not reset completed tasks or edit active workers' dependencies.

## Integration gate

`12-client-design/12-10` is the initial design completion gate. Spec 05's page-shell/bootstrap step cannot be claimed until it is completed; later page work consumes the published tokens/component specifications and records its chosen baseline version. Workflow pages also require the shared dashboard shell. Backend/contract work continues independently. New revisions get their own completion gates; old completed records remain valid evidence for their original version, not proof that a new draft is ready. Visual changes go through affected component/page owners without silently changing API contracts.
