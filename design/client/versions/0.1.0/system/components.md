# Components v0.1.0

| Component | Inputs / behavior | Required states |
|---|---|---|
| AppShell / Navigation | install, operator, selected owned target, capability map; skip link; aria-current | signed out, collapsed mobile menu, disconnected |
| TargetStrip | target ID/name/login/address/revision/trust/observation time; sticky; switching invalidates preflight | verified, observed-unverified, stale, disconnected |
| PageHeading / Notice / StatusBadge | heading plus task copy; text/icon status; alert only for newly introduced blocking errors | ready, loading, empty, unauthorized, unsupported, stale, refused, failed, unknown |
| DataTable | explicit column headers, bounded rows/cursor, filter label, row detail/action, count/time | no results, next/previous, partial, loading; internal scroll on mobile |
| FormField / SecretProvision | persistent label, help and associated error; required fields; no saved secret read/reveal; clear after simulated save | invalid, conflict, submitting, saved, write-only rotation |
| ExactIntent / ApprovalDialog | immutable target/revision/action/args/risk/expiry; focus trap/return, explicit human action | preview, approval required, expired, running; switching target clears preview |
| OperationResult / Verification / AuditRow | submission ID, bounded output, exit status separately from observed evidence/time | refused, definite failure, unknown, verified, mismatch, unavailable; no blind retry |
| Tabs / Pagination / Progress | buttons/links with selected and disabled states; no hover-only control | active, disabled with reason, busy; announces changed result |
| LogView / DiffView | plain text only, monospace, byte/entry limits, truncation and continuation | empty, truncated, hostile text escaped, stale; own scroll region |
| MetricChart / EvidenceCard | units, observation window, table alternative; missing = gap; timezone explicit | available, gap, counter reset, unavailable, stale |
| CapabilityCard / ReadOnlyResource | adapter/tool version and explanation; backend enforces permission | available, unavailable, unknown schema, read-only |

Interaction contract: tab order follows reading order. Focus is 3px cyan with 3px offset, never removed. Buttons/inputs >=44px; links in dense text have 24px spacing or sufficient separation. Error fields use aria-invalid and aria-describedby. Dialog label and description are associated; Escape dismisses only the preview UI. Remote text is always textContent/escaped server text, never innerHTML from a target. No automatic motion; reduce-motion disables transitions. Timestamp text uses UTC in prototypes; production offers user timezone with explicit label. Controls unavailable due to policy/capability include a visible explanation; do not rely on color or tooltip alone.

Token JSON is authoritative; CSS mirrors it. Stitch explorations may differ slightly and are reviewed against local semantic requirements. Fonts use local system fallback; no third-party scripts/fonts required for the local preview. Proposed Leptos slots: ShellContext, TargetContext, PageToolbar, ResourceTable<T>, ExactIntentPanel, ApprovalDialog, OutcomePanel and EvidencePanel; wire DTOs are not decided here.
