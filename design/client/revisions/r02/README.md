# Linux MCP client design refresh r02

This is a draft revision of the published `0.1.0` client design. The prior
baseline remains available at `../../versions/0.1.0/`.

## Review findings

The Spec 12 screens and the private Stitch project
[`3486587707575590516`](https://stitch.withgoogle.com/projects/3486587707575590516)
establish a useful operator-console grammar:

- A persistent left navigation rail and a sticky target/connection strip keep
  install, operator, target, revision, and trust context visible.
- Page headings pair task-oriented copy with one primary action. Secondary
  actions stay close to the relevant card or table.
- Cards group one decision or evidence unit; metric cards are compact and
  always label units and observation time.
- Status chips use an icon and text, not color alone. Verified, refused,
  failed, and unknown have distinct semantic treatments.
- Tables, logs, and diffs use bounded scrolling and explicit truncation or
  continuation states.
- Approval is a focused dialog with immutable target, revision, action,
  arguments, risk, and expiry; it is never represented as a generic “Run”
  button.
- Tabs and pagination are real controls with selected/disabled states, while
  mobile collapses the rail without changing the reading order.

The Stitch project currently contains the published Slate Console design
system (`Inter`, dark mode, cyan primary, eight-pixel rounding). A requested
variant generation timed out, so this refresh is based on the accessible
project metadata and existing screens rather than treating a partial generated
variant as authoritative.

## r02 visual direction

The draft keeps the safety vocabulary and interaction contract while making the
workspace feel more current:

- ink background with cool slate surfaces and softer border hierarchy;
- 12px cards and dialogs with restrained depth instead of flat boxes;
- brighter type hierarchy and a more legible display scale;
- cyan action controls, lime verified state, amber unknown state, and rose
  failure state;
- stronger active-navigation and segmented-control affordances;
- unchanged 44px control minimums, visible focus ring, reduced-motion behavior,
  and synthetic-only data.

Open [`preview/index.html`](preview/index.html) to review the isolated draft
preview. This folder deliberately copies the baseline shell so its CSS can be
compared without changing the published implementation baseline.
