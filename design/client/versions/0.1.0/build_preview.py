"""Assemble synthetic design fixtures; no production dependency."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
pages = [{"id":"components","title":"Component reference","description":"Slate Console v0.1 · synthetic fixtures · no remote execution", "columns":["Target","Status","Observed"],"rows":[["prod-web-01","Verified","2026-10-11 08:30 UTC"],["staging-api-02","Unverified host key","Not collected"]],"notes":["Text-labeled states, 44px controls, persistent target and bounded output.","Command exit status is separate from observed verification."]}]
for path in sorted((ROOT / "screens").glob("*/pages.json")):
    pages.extend(json.loads(path.read_text(encoding="utf-8")))
order = "inventory agent-setup operations audit overview capabilities users services logs processes packages files storage network jobs access monitoring schedules alerts troubleshoot fleet maintenance login server-form host-trust components".split()
pages.sort(key=lambda p: order.index(p["id"]) if p["id"] in order else len(order))
ids = [page["id"] for page in pages]
assert len(ids) == len(set(ids)), "Duplicate screen ID"
for page in pages:
    for key in ("id", "title", "description", "columns", "rows", "notes"):
        assert key in page, (page["id"], key)
    assert all(len(row) == len(page["columns"]) for row in page["rows"]), page["id"]
payload = json.dumps(pages, ensure_ascii=True).replace("<", "\\u003c")
(ROOT / "preview" / "screens.js").write_text("const PAGES = " + payload + ";\n", encoding="utf-8")
print(f"Built {len(pages)} synthetic screens; unique IDs and table shapes passed")
