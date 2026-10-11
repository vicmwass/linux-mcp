# Screen and state coverage

Each family needs loading, empty, ready, invalid/error, unauthorized, unsupported, disconnected and stale states where applicable. Guarded actions additionally need preview, approval-required/expired, running, refused, failed-before-submit, unknown-after-submit and verified/mismatch/unavailable. Screens are synthetic and capability-driven. State annotations accompany the visual references.

| Family / screen IDs | Step owner | Product source | Evidence required |
|---|---|---|---|
| login, inventory, server-form, host-trust, agent-setup | 12-04 | 05 | write-only secrets, conflict, expiry/revocation, isolated owner/install context, mobile form |
| overview, capabilities, users, services, logs, approval, operations, audit | 12-05 | 05/08/06 | timestamps, bounded hostile-looking text, exact intent, verification and unknown outcome |
| processes, packages, files, storage, network, jobs, access | 12-06 | 09 | PID/start identity, partial packages, file diff/revision, read-only storage, connectivity risk, timezone, privileges |
| monitoring, schedules, alerts, troubleshoot, fleet, maintenance | 12-07 | 10/11 | metric gaps/units, grant expiry, partial fleet, per-step approval and reconciliation |

Navigation/wireframes: 12-02. Tokens and reusable components: 12-03. Cross-family responsive/accessibility review: 12-08. Rust handoff: 12-09. Baseline traceability and publication: 12-10. No backend dependency is introduced; publication needs all design steps and actual prototype checks. User approval is separate from agent publication. Review/reopen remains available for all steps.
