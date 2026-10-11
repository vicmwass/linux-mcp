# Spec 12 traceability — 0.1.0

| Requirement | Artifact | Review evidence |
|---|---|---|
| audience, context, visual direction, assumptions | brief.md, coverage.md | 12-01 evidence; all family ownership assigned |
| information architecture, target/capability flows, three wireframes | navigation/flows.md, wireframes.html | 12-02 walkthroughs; assembled context checks |
| semantic tokens, CSS, reusable components | system/tokens.json, components.md; preview/style.css | 24 measured contrast pairs, CSS consistency, keyboard review |
| account/agent/inventory/secrets/trust/conflict | screens/inventory/pages.json | 12-04 evidence; six-field form/port/secret-clearing checks |
| overview/capabilities/users/services/logs | screens/console/pages.json | 12-05 evidence + UTF-8 rework; escaped-output/context checks |
| exact approval, refusal, outcome/verification/audit | console annotations; preview/app.js shared states | dialog/target switch/unknown/read-only checks; approval and unknown screenshots |
| seven administration modules | screens/administration/pages.json, state-matrix.md | 12-06 evidence; file diff/revision; storage read-only |
| monitoring/history/schedules/alerts | screens/monitoring-workflows/pages.json, state-matrix.md | metric gap/table/units/time/grant annotations and mobile review |
| troubleshooting/fleet/maintenance/restart | same workflow artifacts | partial fleets and resolved-step approval/unknown reconciliation |
| screen/state index, responsive preview, screenshots | preview/index.html, review/screen-index.md, six review JPEGs | 108 final layout measurements across four widths; representative visual inspection |
| accessibility review and known limits | system/components.md; review/report.md, findings.md | focus/error/menu/contrast/text cues; production screen-reader/device review deferred |
| Rust component/data/page handoff | handoff.md | 12-09 mapping checked against registered steps; provisional schemas only |
| versioning, review/rework and adoption | baselines.json, versions/0.1.0/manifest.json; handoff.md | snapshot checksums and coordinator history; no invented human approval |
| Stitch user authorization and reproducibility | specs/12 scope note; stitch.json; preview/stitch/index.html | private project, six screen IDs and complete prompts; source export failure recorded |

All applicable read/mutation states are represented by shared review selector plus page-specific annotations. Missing capabilities do not imply raw-shell execution; unknown mutation never auto-replays; target context, trust, credentials and result verification follow the safety requirements. Optional AI/WebMCP is absent and does not gate the deterministic design.
