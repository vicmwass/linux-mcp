# Design request: per-server approval requirement

Status: **open** (requested 2026-10-11). Raised by the user while approving Spec 00's approval semantics. Not yet designed; no published or in-review baseline contains it. This is feedback in the sense of [client-design-revisions.md](../../../specs/plans/client-design-revisions.md): integrate it as a supplemental revision before first publication of 0.1.0, or as a later revision. Do not edit baseline artifacts in place, and do not mark this addressed without recorded review evidence.

## Requirement (source of truth: Spec 00 decisions D9/D11)

Each server has an approval requirement, set together with its other configuration:

| Option | Label | Effect |
|---|---|---|
| `medium_and_above` | Medium risk and above (default, recommended) | Medium, high and critical operations need human approval |
| `high_and_above` | High risk and above | Only high-risk operations need approval; medium-risk operations (create user, restart service, install package) run without it |

Rules the design must make visible:

- The default is "Medium risk and above"; the weaker option is an explicit choice with a warning.
- Policy denials are never affected, and neither option treats a UI click or `confirm: true` as approval.
- Only a person in the interactive dashboard session can change it. Agents and MCP/delegated tokens cannot. Outside an interactive session the control is read-only with an "interactive session required" explanation.
- Saving a change bumps the server revision and invalidates pending approvals for that server; the form says so before saving.
- It is a per-server setting: changing one server never changes another.

## Affected screens and states

| Screen | Change |
|---|---|
| Inventory add/edit server form | New two-option control, examples for each option, weaker-option warning, invalidation note; validation and revision-conflict states |
| Inventory list/detail | Show the current requirement as a badge or summary line (text, not colour alone) |
| Approval panel | Show the requirement in effect for the target next to risk and expiry |
| Approval invalidated | Copy for "approval no longer valid: server settings changed" |
| Read-only context | Explanation when the session is not interactive |

Accessibility: radio group with accessible names and a described-by warning; keyboard order; non-colour cues for the weaker option.

## Acceptance for the revision

- Every affected screen above has desktop, tablet and mobile coverage with the states listed.
- Copy distinguishes policy refusal, approval required, approval invalidated and unknown outcome.
- Handoff maps the control to the contract field `approval_mode` (`ApprovalMode`, `medium_and_above` | `high_and_above`) and to Spec 05's form step.
- No human approval is claimed unless explicitly recorded.

## Out of scope

Rust implementation (Spec 05) and any change to the contract (owned by Spec 00).
