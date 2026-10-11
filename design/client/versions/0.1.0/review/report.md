# Prototype review — v0.1.0

2026-10-11. Tools: CUA in-app browser, Python JSON/contrast/build checks. Synthetic data only; no SSH/MCP/auth service was exercised.

Responsive: 27 screens × 1440/834/390/320px = 108 DOM layout measurements. One h1 each and no outer horizontal overflow in final responsive.json. Initial 320px host fingerprint overflow is retained in responsive-initial.json and corrected by long-identifier wrapping. Rechecked 14 console/monitoring screens at 320px after UTF-8 fixture correction. Representative desktop/tablet/mobile screenshots were visually inspected; DOM checks do not imply exhaustive visual inspection of every state/device.

Contrast: check_tokens.py passed 24 color pairs and semantic CSS/token consistency. Body/status text >=7.17:1 on all specified surfaces; primary button text 11.59:1; border 3.45:1 against surface. Focus cyan 11.53:1; visible focus checked via keyboard. Reduced-motion rule is present; the preview has no animation. Production/device/assistive-technology checks remain required.

Interaction checks: first invalid field receives focus; port 0 rejected, port 22 accepted; valid simulated save clears all six fields including new secret. Dialog shows exact target/revision/action/args/risk/expiry; Escape returns to trigger. Mobile menu opens with focus, marks background inert and closes by Escape. Target switch invalidates preview and removes prior target observations; unverified target cannot act. Login/unauthorized hides remote context. Unknown-after-submit disables mutation and offers read-only reconciliation. Storage remains read-only even in approval state. Hostile log tags appear as text; zero script nodes in log content. Table search/no-results and bounded scroll regions are implemented; finite synthetic pages disable pagination at end.

Screenshots: overview-desktop.jpg, approval-desktop.jpg, unknown-desktop.jpg, form-mobile.jpg, files-tablet.jpg, monitoring-mobile.jpg. Refer to actual files/checksums at publication; screenshots are samples rather than production snapshots.

Stitch: six screens and one design system generated in private project 3486587707575590516. Prompts/resource IDs/asset URLs in ../stitch.json; local reference index in ../preview/stitch/index.html. Original generated HTML export was attempted but failed (shell URLError and browser download timeout); no source export is claimed. Local preview is self-contained and independent of Stitch availability. Generated descriptions sometimes invent implementation details; findings.md records exclusions. Stitch visuals are reviewable references, not automatically approved application code.

No open critical design finding. Deferred items: real provider/contracts/capability wiring, backend authority/approval verification, screen-reader/device/zoom validation, original Stitch source archive. Publication means an agent-reviewed prototype baseline; no human approval or WCAG certification is implied.
