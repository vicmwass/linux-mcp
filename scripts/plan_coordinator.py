"""Local shared-checkout step claims. Standard library only; see specs/plans/README.md."""
import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import secrets
import sys
import tempfile
import time
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]


def now():
    return datetime.now(timezone.utc).isoformat()


@contextlib.contextmanager
def transaction(directory):
    directory.mkdir(parents=True, exist_ok=True)
    lock = open(directory / "transaction.lock", "a+b")
    if os.name == "nt":
        import msvcrt
        lock.seek(0, 2)
        if lock.tell() == 0:
            lock.write(b"0")
            lock.flush()
        while True:
            lock.seek(0)
            try:
                msvcrt.locking(lock.fileno(), msvcrt.LK_NBLCK, 1)
                break
            except OSError:
                time.sleep(0.05)
    else:
        import fcntl
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
    try:
        yield
    finally:
        if os.name == "nt":
            lock.seek(0)
            msvcrt.locking(lock.fileno(), msvcrt.LK_UNLCK, 1)
        else:
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
        lock.close()


def save(path, state):
    fd, name = tempfile.mkstemp(dir=path.parent, prefix="state-", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as output:
            json.dump(state, output, indent=2)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def qualified(plan_id, step_id):
    return step_id if "/" in step_id else f"{plan_id}/{step_id}"


def validate_registry(plan):
    ids = [s["id"] for s in plan["steps"]]
    if not plan.get("plan_id") or len(set(ids)) != len(ids) or not ids:
        raise ValueError("Invalid plan ID or duplicate/empty steps")
    visited = set()
    pending = list(plan["steps"])
    while pending:
        ready = [s for s in pending if {d for d in s["depends_on"] if "/" not in d} <= visited]
        if not ready:
            raise ValueError("Missing dependency or dependency cycle")
        for step in ready:
            if not step.get("resources") or len(set(step["resources"])) != len(step["resources"]):
                raise ValueError("Steps require unique nonempty resources")
            visited.add(step["id"])
            pending.remove(step)


def load_plans(args):
    selected_path = args.plan.resolve()
    catalog = args.catalog or selected_path.parent / "catalog.json"
    paths = [selected_path]
    if catalog.exists():
        config = json.loads(catalog.read_text(encoding="utf-8"))
        if config.get("version") != 1:
            raise ValueError("Unsupported catalog version")
        catalog_paths = [(catalog.parent / p).resolve() for p in config["plans"]]
        if args.all or selected_path in catalog_paths:
            paths = catalog_paths
    elif args.all:
        raise ValueError("--all requires a plan catalog")
    plans = {}
    for path in paths:
        raw = path.read_bytes()
        plan = json.loads(raw)
        validate_registry(plan)
        if plan["plan_id"] in plans:
            raise ValueError("Duplicate plan ID in catalog")
        plans[plan["plan_id"]] = (plan, hashlib.sha256(raw).hexdigest(), path)
    selected = next((pid for pid, (_, _, path) in plans.items() if path == selected_path), None)
    if not args.all and selected is None:
        raise ValueError("Selected plan missing from catalog")
    graph = {qualified(pid, s["id"]): {qualified(pid, d) for d in s["depends_on"]}
             for pid, (p, _, _) in plans.items() for s in p["steps"]}
    remaining, visited = dict(graph), set()
    while remaining:
        ready = [key for key, deps in remaining.items() if deps <= visited]
        if not ready:
            raise ValueError("Missing cross-plan dependency or catalog cycle")
        for key in ready:
            visited.add(key)
            remaining.pop(key)
    return plans, selected


def run(args):
    plans, selected = load_plans(args)
    with transaction(args.state_dir):
        path = args.state_dir / "state.json"
        state = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {"version": 1, "plans": {}}
        if state.get("version") != 1 or not isinstance(state.get("plans"), dict):
            raise ValueError("Invalid state; stop workers and reconcile")
        for pid, (plan, digest, _) in plans.items():
            entry = state["plans"].get(pid)
            if entry is None:
                entry = {"registry_sha256": digest, "steps": {
                    s["id"]: {"status": "pending", "history": [], "resources": s["resources"]}
                    for s in plan["steps"]}}
                state["plans"][pid] = entry
            if entry["registry_sha256"] != digest:
                raise ValueError(f"Registry changed ({pid}); stop workers and explicitly migrate state")
        specs = [(pid, s) for pid, (p, _, _) in plans.items()
                 if args.all or pid == selected for s in p["steps"]]

        def lookup(pid, sid):
            target_plan, target_step = qualified(pid, sid).split("/", 1)
            return state["plans"][target_plan]["steps"][target_step]
        occupied = {resource: f'{pid}/{sid}'
                    for pid, p in state["plans"].items()
                    for sid, s in p["steps"].items() if s["status"] == "claimed"
                    for resource in s["resources"]}

        def eligibility(pid, spec):
            record = lookup(pid, spec["id"])
            waiting = [qualified(pid, d) for d in spec["depends_on"] if lookup(pid, d)["status"] != "completed"]
            conflicts = {r: occupied[r] for r in spec["resources"] if r in occupied}
            enabled = not spec.get("opt_in") or spec["opt_in"] in args.enable
            return record["status"] == "pending" and not waiting and not conflicts and enabled, waiting, conflicts

        if args.command == "status":
            report = []
            for pid, spec in specs:
                record = lookup(pid, spec["id"])
                eligible, waiting, conflicts = eligibility(pid, spec)
                stale = record["status"] == "claimed" and (
                    datetime.now(timezone.utc) - datetime.fromisoformat(record["heartbeat_at"])
                ).total_seconds() > 600
                report.append({"plan": pid, **spec, **record, "eligible": eligible, "waiting_for": waiting,
                               "opt_in_disabled": bool(spec.get("opt_in") and spec["opt_in"] not in args.enable),
                               "resource_conflicts": conflicts, "stale_heartbeat_warning": stale})
            result = {"plan": "all" if args.all else selected, "state_dir": str(args.state_dir.resolve()), "steps": report}
        elif args.command == "claim":
            if any(s.get("owner") == args.owner and s["status"] == "claimed"
                   for p in state["plans"].values() for s in p["steps"].values()):
                raise ValueError("Owner already has a claim; resume or release it first")
            candidates = [(pid, s) for pid, s in specs if not args.step or args.step in (s["id"], qualified(pid, s["id"]))]
            chosen = next(((pid, s) for pid, s in candidates if eligibility(pid, s)[0]), None)
            if chosen is None:
                raise ValueError("No eligible step; inspect status for claims, blockers and dependencies")
            pid, spec = chosen
            record = lookup(pid, spec["id"])
            record.setdefault("attempt", 1)
            record.update(status="claimed", owner=args.owner, token=secrets.token_hex(24),
                          claimed_at=now(), heartbeat_at=now(), note="", resources=list(spec["resources"]))
            record["history"].append({"event": "claimed", "at": now(), "owner": args.owner})
            result = {"plan": pid, "step": spec["id"], **record}
        else:
            matches = [(pid, s) for pid, s in specs if args.step in (s["id"], qualified(pid, s["id"]))]
            if len(matches) != 1:
                raise ValueError("Unknown step")
            pid, spec = matches[0]
            record = lookup(pid, spec["id"])
            if args.command == "review":
                record.setdefault("reviews", []).append({"at": now(), "reviewer": args.reviewer,
                                                          "note": args.reason})
            elif args.command == "reopen":
                if record["status"] != "completed":
                    raise ValueError("Only a completed step can be reopened; use release/unblock for unfinished work")
                source = qualified(pid, spec["id"])
                affected = {source}
                graph = {qualified(p, s["id"]): {qualified(p, d) for d in s["depends_on"]}
                         for p, (definition, _, _) in plans.items() for s in definition["steps"]}
                while True:
                    expanded = affected | {key for key, deps in graph.items() if deps & affected}
                    if expanded == affected:
                        break
                    affected = expanded
                active = [key for key in affected if lookup(*key.split("/", 1))["status"] == "claimed"]
                if active:
                    raise ValueError("Reopen refused; affected workers must hand off first: " + ", ".join(sorted(active)))
                reset = []
                for key in sorted(affected):
                    target = lookup(*key.split("/", 1))
                    if target["status"] != "completed":
                        continue
                    target.setdefault("previous_completions", []).append({
                        "attempt": target.get("attempt", 1), "completed_at": target.get("completed_at"),
                        "evidence": target.get("evidence"), "owner": target.get("last_owner")})
                    target.update(status="pending", attempt=target.get("attempt", 1) + 1,
                                  rework_reason=args.reason, invalidated_by=source)
                    target.pop("completed_at", None)
                    target.pop("evidence", None)
                    target["history"].append({"event": "reopened" if key == source else "invalidated",
                                              "at": now(), "source": source, "reason": args.reason})
                    reset.append(key)
                record["reopened_steps"] = reset
            elif args.command == "unblock":
                if record["status"] != "blocked":
                    raise ValueError("Step is not blocked")
                record["status"] = "pending"
            else:
                if record["status"] != "claimed" or not secrets.compare_digest(record.get("token", ""), args.token):
                    raise ValueError("Claim missing or token mismatch; stop work")
                if args.command == "heartbeat":
                    record.update(heartbeat_at=now(), note=args.note or record.get("note", ""))
                elif args.command == "reserve-doc":
                    own_key = qualified(pid, spec["id"])
                    if "architecture-doc" in occupied and occupied["architecture-doc"] != own_key:
                        raise ValueError("architecture-doc is reserved; keep your claim and retry after its owner releases it")
                    if "architecture-doc" not in record["resources"]:
                        record["resources"].append("architecture-doc")
                    record["heartbeat_at"] = now()
                else:
                    if args.command == "complete":
                        if any(lookup(pid, d)["status"] != "completed" for d in spec["depends_on"]):
                            raise ValueError("Dependencies not completed")
                        record.update(status="completed", completed_at=now(), evidence=args.evidence)
                    elif args.command == "block":
                        record.update(status="blocked", reason=args.reason, evidence=args.evidence)
                    else:
                        record["status"] = "pending"
                    record["last_owner"] = record.pop("owner")
                    record.pop("token")
            record["history"].append({"event": args.command, "at": now(),
                                      "reason": getattr(args, "reason", None),
                                      "evidence": getattr(args, "evidence", None),
                                      "note": getattr(args, "note", None)})
            result = {"plan": pid, "step": spec["id"], **record}
        save(path, state)
        return result


def nonempty(value):
    if not value.strip():
        raise argparse.ArgumentTypeError("Must not be empty")
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", type=Path, default=ROOT / "specs/plans/00-contracts.steps.json")
    parser.add_argument("--state-dir", type=Path, default=ROOT / "specs/plans/.coord")
    parser.add_argument("--catalog", type=Path, help="Defaults to catalog.json alongside selected plan")
    parser.add_argument("--all", action="store_true", help="Schedule/report all catalog plans (only within authorized scope)")
    parser.add_argument("--enable", action="append", default=[], choices=("webmcp", "assistant"), help="Explicitly select optional package (repeatable)")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("status")
    claim = commands.add_parser("claim")
    claim.add_argument("--owner", required=True, type=nonempty)
    claim.add_argument("--step")
    for command in ("heartbeat", "reserve-doc", "complete", "block", "release", "recover", "unblock", "review", "reopen"):
        sub = commands.add_parser(command)
        sub.add_argument("--step", required=True)
        if command not in ("unblock", "review", "reopen"):
            sub.add_argument("--token", required=True, type=nonempty)
        if command == "review":
            sub.add_argument("--reviewer", required=True, type=nonempty)
        if command == "heartbeat":
            sub.add_argument("--note", type=nonempty)
        if command in ("block", "release", "recover", "unblock", "review", "reopen"):
            sub.add_argument("--reason", required=True, type=nonempty)
        if command in ("complete", "block"):
            sub.add_argument("--evidence", required=True, type=nonempty)
    try:
        print(json.dumps(run(parser.parse_args()), indent=2))
    except (ValueError, KeyError, TypeError, OSError) as error:
        print(f"Coordinator refused: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
