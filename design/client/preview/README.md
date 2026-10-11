# Local preview

From repository root: `python design/client/build_preview.py`, then `python -m http.server 8765 --bind 127.0.0.1 --directory design/client`. Open http://127.0.0.1:8765/preview/index.html. No dependencies or backend. The HTTP server is loopback only.

Page-family workers add one JSON file in their owned screens directory; each record has id, title, description, columns, rows, notes and optional kind. The build script assembles these into the gallery. Workers do not edit shared CSS/renderer. No remote values are interpolated as HTML. View-state selector and dialogs demonstrate simulated outcomes; no real operation occurs. Production logic will be Rust.
