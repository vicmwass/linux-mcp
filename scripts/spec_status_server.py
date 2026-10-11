"""Read-only loopback viewer for the shared checkout coordinator. Stdlib only."""
import argparse
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
from types import SimpleNamespace
from urllib.parse import urlsplit

from plan_coordinator import load_plans, qualified

ROOT = Path(__file__).resolve().parents[1]
PAGE = ROOT / "tools/spec-status/index.html"
STATUSES = {"pending", "claimed", "blocked", "completed"}


def snapshot(root=ROOT, state_dir=None):
    """Never call coordinator.run(): even its status command writes state."""
    plans, _ = load_plans(SimpleNamespace(
        plan=root / "specs/plans/00-contracts.steps.json",
        catalog=root / "specs/plans/catalog.json", all=True))
    state = json.loads(((state_dir or root / "specs/plans/.coord") / "state.json").read_bytes())
    if state.get("version") != 1 or not isinstance(state.get("plans"), dict):
        raise ValueError("Invalid coordinator state")
    records = {}
    for pid, (plan, digest, _) in plans.items():
        entry = state["plans"].get(pid)
        if not entry or entry.get("registry_sha256") != digest:
            raise ValueError(f"Missing state or registry drift: {pid}")
        for step in plan["steps"]:
            record = entry["steps"].get(step["id"])
            if not record or record.get("status") not in STATUSES:
                raise ValueError(f"Invalid or missing step: {pid}/{step['id']}")
            records[qualified(pid, step["id"])] = record
    # Include reservations from tooling plans too, without exposing their records.
    occupied = {r: qualified(pid, sid)
                for pid, entry in state["plans"].items()
                for sid, record in entry["steps"].items()
                if record["status"] == "claimed" for r in record["resources"]}
    now = datetime.now(timezone.utc)
    result = []
    for pid, (plan, _, _) in plans.items():
        steps = []
        for step in plan["steps"]:
            key = qualified(pid, step["id"])
            record = records[key]
            waiting = [qualified(pid, d) for d in step["depends_on"]
                       if records[qualified(pid, d)]["status"] != "completed"]
            conflicts = {r: occupied[r] for r in step["resources"]
                         if r in occupied and occupied[r] != key}
            age = None
            if record["status"] == "claimed":
                heartbeat = datetime.fromisoformat(record["heartbeat_at"])
                age = max(0, int((now - heartbeat).total_seconds()))
            # Explicit projection: never serialize a record or history wholesale.
            steps.append({
                "id": step["id"], "title": step["title"], "status": record["status"],
                "optional": bool(step.get("opt_in")), "package": step.get("opt_in"),
                "owner": record.get("owner") if record["status"] == "claimed" else None,
                "attempt": record.get("attempt", 1), "note": record.get("note", ""),
                "reason": record.get("reason") if record["status"] == "blocked" else None,
                "evidence": record.get("evidence") if record["status"] == "completed" else None,
                "heartbeat_age_seconds": age, "stale_heartbeat": age is not None and age > 600,
                "waiting_for": waiting, "resource_conflicts": conflicts,
                "eligible": record["status"] == "pending" and not waiting and not conflicts and not step.get("opt_in"),
                "last_activity": max((h["at"] for h in record.get("history", []) if h.get("at")), default=None),
            })
        required = [s for s in steps if not s["optional"]]
        done = sum(s["status"] == "completed" for s in required)
        status = ("completed" if done == len(required) else
                  "blocked" if any(s["status"] == "blocked" for s in required) else
                  "in progress" if any(s["status"] == "claimed" for s in required) or done else "pending")
        result.append({"id": pid, "status": status, "completed": done,
                       "total": len(required), "steps": steps,
                       "optional_total": len(steps) - len(required),
                       "optional_completed": sum(s["optional"] and s["status"] == "completed" for s in steps)})
    return {"updated_at": now.isoformat(), "plans": result}


def handler(root=ROOT, state_dir=None):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            route = urlsplit(self.path).path
            if route in ("/", "/index.html"):
                body, mime, code = PAGE.read_bytes(), "text/html; charset=utf-8", 200
            elif route == "/api/status":
                try:
                    payload, code = snapshot(root, state_dir), 200
                except (OSError, ValueError, KeyError, TypeError) as error:
                    payload, code = {"error": "Coordinator snapshot unavailable; retrying", "kind": type(error).__name__}, 503
                body, mime = json.dumps(payload).encode(), "application/json; charset=utf-8"
            else:
                body, mime, code = b"Not found", "text/plain", 404
            self.send_response(code)
            self.send_header("Content-Type", mime)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.send_header("Content-Security-Policy", "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'")
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_):
            pass
    return Handler


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8779)
    parser.add_argument("--state-dir", type=Path, help="Existing shared coordinator directory")
    args = parser.parse_args()
    server = ThreadingHTTPServer(("127.0.0.1", args.port), handler(state_dir=args.state_dir))
    print(f"Spec status: http://127.0.0.1:{server.server_port} (read-only)", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
