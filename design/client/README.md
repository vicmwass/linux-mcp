# Linux MCP client design

Baseline: **0.1.0**. Read [handoff](handoff.md), [review report](review/report.md), [screen index](review/screen-index.md) and [traceability](traceability.md). Publication state is authoritative in [baselines.json](baselines.json). No human approval is implied.

[Open the Stitch project](https://stitch.withgoogle.com/projects/3486587707575590516). Six generated visual references cover the component system, inventory, service investigation/approval, tablet Files, mobile Monitoring/workflows and desktop Overview. Resource IDs and complete prompts are in stitch.json. Original generated source downloads were unavailable.

Local interactive preview: from repository root run `python design/client/build_preview.py`, then `python -m http.server 8765 --bind 127.0.0.1 --directory design/client`. Open http://127.0.0.1:8765/preview/index.html. All 27 pages, shared state controls and synthetic interactions work without Stitch, backend services or remote machines. This prototype's Python/JavaScript does not change the Rust product requirement.

Published versions live in versions/. Preserve snapshots and manifests when reviewing/redoing; use the coordinator's review/reopen or supplemental design-revision workflow. Production implementers pin the published version until an adoption change is assigned.
