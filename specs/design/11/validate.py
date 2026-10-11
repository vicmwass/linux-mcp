#!/usr/bin/env python3
"""Validate the Spec 11 workflow schemas, fixtures and lint rules (design-time evidence).

    python specs/design/11/validate.py

Standard library only. It implements the subset of JSON Schema 2020-12 the schemas use, plus
the semantic lint rules documented in model.md (templates L1-L9), state-machine.md (runs R1-R3)
and fleet.md (comparisons F1-F3). The production Rust implementation (11-03 onward) must
re-implement the same rules; these fixtures are the shared contract tests.

Exit status 0 only if every valid fixture passes with no findings and every invalid fixture
produces exactly its expected rule IDs.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
KNOWN = {
    "$schema", "$ref", "$defs", "title", "description", "type", "properties", "required",
    "additionalProperties", "items", "oneOf", "const", "enum", "minimum", "maximum",
    "minLength", "maxLength", "minItems", "maxItems", "pattern",
}


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


# ------------------------------------------------------------------ schema validator
def type_ok(kind, value):
    return {
        "object": isinstance(value, dict),
        "array": isinstance(value, list),
        "string": isinstance(value, str),
        "boolean": isinstance(value, bool),
        "null": value is None,
        "integer": isinstance(value, int) and not isinstance(value, bool),
        "number": isinstance(value, (int, float)) and not isinstance(value, bool),
    }[kind]


def check(root, schema, value, at="$"):
    """Returns a list of violations (empty when valid)."""
    if schema is True:
        return []
    errors: list[str] = []
    for key in schema:
        assert key in KNOWN, f"unsupported schema keyword {key!r} at {at}"
    if "$ref" in schema:
        node = root
        for part in schema["$ref"].removeprefix("#/").split("/"):
            node = node[part]
        errors += check(root, node, value, at)
    if "type" in schema:
        kinds = schema["type"] if isinstance(schema["type"], list) else [schema["type"]]
        if not any(type_ok(k, value) for k in kinds):
            return errors + [f"{at}: expected {kinds}, got {type(value).__name__}"]
    if "const" in schema and schema["const"] != value:
        errors.append(f"{at}: expected const {schema['const']!r}")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{at}: {value!r} not in enum")
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            errors.append(f"{at}: below minimum {schema['minimum']}")
        if "maximum" in schema and value > schema["maximum"]:
            errors.append(f"{at}: above maximum {schema['maximum']}")
    if isinstance(value, str):
        if "minLength" in schema and len(value) < schema["minLength"]:
            errors.append(f"{at}: shorter than {schema['minLength']}")
        if "maxLength" in schema and len(value) > schema["maxLength"]:
            errors.append(f"{at}: longer than {schema['maxLength']}")
        if "pattern" in schema and not re.search(schema["pattern"], value):
            errors.append(f"{at}: does not match pattern")
    if isinstance(value, dict):
        props = schema.get("properties", {})
        for name in schema.get("required", []):
            if name not in value:
                errors.append(f"{at}: missing required {name!r}")
        extra = schema.get("additionalProperties", True)
        for name, member in value.items():
            if name in props:
                errors += check(root, props[name], member, f"{at}.{name}")
            elif extra is False:
                errors.append(f"{at}: unexpected property {name!r}")
            elif isinstance(extra, dict):
                errors += check(root, extra, member, f"{at}.{name}")
    if isinstance(value, list):
        if "minItems" in schema and len(value) < schema["minItems"]:
            errors.append(f"{at}: fewer than {schema['minItems']} items")
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            errors.append(f"{at}: more than {schema['maxItems']} items")
        if "items" in schema:
            for i, item in enumerate(value):
                errors += check(root, schema["items"], item, f"{at}[{i}]")
    if "oneOf" in schema:
        matching = sum(1 for s in schema["oneOf"] if not check(root, s, value, at))
        if matching != 1:
            errors.append(f"{at}: matches {matching} of oneOf (expected 1)")
    return errors


# ------------------------------------------------------------------ template lint
RAW_NAMES = re.compile(r"(^|_)(command|cmd|script|shell|exec|bash|sh)(_|$)")
ALLOW = load(HERE / "tool-allowlist.json")
TOOLS = {t["id"]: t for t in ALLOW["tools"]}


def lint_template(t, schema):
    """Returns a set of rule IDs; empty means the template is acceptable."""
    if check(schema, schema, t):
        return {"L1"}
    found: set[str] = set()
    steps = t["steps"]
    ids = [s["id"] for s in steps]
    if len(set(ids)) != len(ids):
        found.add("L2")
    index = {s["id"]: i for i, s in enumerate(steps)}
    params = {p["name"]: p for p in t["parameters"]}

    # L5: parameter names/types cannot smuggle raw shell; defaults inside range.
    for name, p in params.items():
        if RAW_NAMES.search(name):
            found.add("L5")
        if p["type"] == "integer" and "default" in p:
            lo, hi = p.get("minimum", -(2**31)), p.get("maximum", 2**31)
            if not (isinstance(p["default"], int) and lo <= p["default"] <= hi):
                found.add("L9")

    # L7: step budget covers the steps.
    if len(steps) > t["budgets"]["max_steps"]:
        found.add("L7")

    mutating = [s for s in steps if s["kind"] == "mutate"]
    # L8: read-only integrity and mutate/verify pairing.
    if mutating and (t["read_only"] or t["kind"] != "maintenance"):
        found.add("L8")
    if not mutating and not t["read_only"]:
        found.add("L8")
    if t["kind"] != "maintenance" and not t["read_only"]:
        found.add("L8")

    def binding_ok(owner_i, name, b, tool, allow_observation):
        """Checks one argument binding; adds rule IDs."""
        if "param" in b and b["param"] not in params:
            found.add("L9")
        if "observation" in b:
            o = b["observation"]
            if o["step"] not in index:
                found.add("L9")
            elif index[o["step"]] >= owner_i:
                found.add("L2")
            spec = TOOLS.get(tool, {}).get("args", {}).get(name, {}) if tool else {}
            if not allow_observation or (tool and spec.get("observable_validator") != o["validator"]):
                found.add("L6")

    for i, s in enumerate(steps):
        kind = s["kind"]
        tool = s.get("tool", {}).get("id") if kind in ("inspect", "mutate", "verify") else None
        if tool is not None:
            if tool in ALLOW["denied_tools"]:
                found.add("L4")
                continue  # nothing else is meaningful for a raw-shell step
            spec = TOOLS.get(tool)
            if spec is None or spec["schema_version"] != s["tool"]["schema_version"]:
                found.add("L3")
            else:
                if (kind == "mutate") != spec["mutating"]:
                    found.add("L3")
                for name in s["arguments"]:
                    if name not in spec["args"]:
                        found.add("L3")
                for name, a in spec["args"].items():
                    if a.get("required") and name not in s["arguments"]:
                        found.add("L3")
                for name, a in spec["args"].items():
                    b = s["arguments"].get(name)
                    if b and "literal_values" in a and "literal" in b and b["literal"] not in a["literal_values"]:
                        found.add("L3")
                    if b and "literal_values" in a and "param" in b:
                        found.add("L3")  # closed value sets are literals; observations are L6
        if kind in ("inspect", "mutate", "verify", "suggest"):
            key = "parameters" if kind == "suggest" else "arguments"
            for name, b in s[key].items():
                binding_ok(i, name, b, tool, allow_observation=True)
        if kind == "suggest" and not re.fullmatch(r"[a-z][a-z0-9-]{2,63}", s["template_ref"]):
            found.add("L9")
        if kind in ("compare", "checkpoint"):
            ref = s["from_step"] if kind == "compare" else s["condition"]["from_step"]
            if ref not in index:
                found.add("L9")
            elif index[ref] >= i:
                found.add("L2")
        if kind == "mutate":
            v = s["verify_step"]
            if v not in index or steps[index[v]]["kind"] != "verify" or index[v] <= i or steps[index[v]]["after_step"] != s["id"]:
                found.add("L8")
        if kind == "verify":
            if s["after_step"] not in index or index[s["after_step"]] >= i:
                found.add("L2")
            if s["expect"]["from_step"] not in index or index[s["expect"]["from_step"]] > i:
                found.add("L2")
    return found


# ------------------------------------------------------------------ run and fleet lint
def lint_run(r, schema):
    if check(schema, schema, r):
        return {"R1"}
    found: set[str] = set()
    steps = r["steps"]
    dispatched: dict[tuple, int] = {}
    for s in steps:
        post_submit = s["state"] in {"dispatching", "submitted", "unknown", "reconciled"} or s["submission"] in {"submission_pending", "submitted", "completed"}
        if post_submit and s["operation_id"] is None:
            found.add("R2")
        if s["state"] == "succeeded" and s["submission"] == "completed" and s["operation_id"] is None:
            found.add("R2")
        if s["operation_id"] is not None:
            key = (s["step_id"], s["target_server_id"])
            dispatched[key] = dispatched.get(key, 0) + 1
    if any(n > 1 for n in dispatched.values()):
        found.add("R3")
    if r["state"] == "unknown_outcome" and not any(s["state"] == "unknown" for s in steps):
        found.add("R4")
    if r["state"] == "blocked" and r["state_reason"] not in {"authorization_lost", "revision_changed", "target_disabled"}:
        found.add("R5")
    return found


def lint_fleet(f, schema):
    if check(schema, schema, f):
        return {"F0"}
    found: set[str] = set()
    rows = f["rows"]
    collected = [x for x in rows if x["status"] == "collected"]
    if f["complete"] != (len(collected) == len(rows) and f["partial_reason"] is None):
        found.add("F1")
    for x in rows:
        if x["status"] != "collected" and x["values"] is not None:
            found.add("F2")  # a gap must be null/absent, never a fabricated value (e.g. 0)
        if x["status"] == "collected" and x["values"] is None:
            found.add("F2")
    for a in f["aggregates"]:
        if a["included_count"] > len(collected) or a["requested_count"] != f["requested_count"]:
            found.add("F3")
    if f["included_count"] != len(collected):
        found.add("F3")
    return found


# ------------------------------------------------------------------ driver
def main() -> int:
    failures: list[str] = []
    template_schema = load(HERE / "schemas" / "workflow-template.schema.json")
    run_schema = load(HERE / "schemas" / "workflow-run.schema.json")
    fleet_schema = load(HERE / "schemas" / "fleet-comparison.schema.json")
    counts = {"valid": 0, "invalid": 0}

    def expect_valid(path, found):
        counts["valid"] += 1
        if found:
            failures.append(f"{path.relative_to(HERE)}: unexpected findings {sorted(found)}")

    def expect_invalid(path, found, expected):
        counts["invalid"] += 1
        if set(expected) != found:
            failures.append(f"{path.relative_to(HERE)}: expected {sorted(expected)}, got {sorted(found)}")

    for p in sorted((HERE / "fixtures" / "templates").glob("*.json")):
        expect_valid(p, lint_template(load(p), template_schema))
    for p in sorted((HERE / "fixtures" / "invalid-templates").glob("*.json")):
        d = load(p)
        expect_invalid(p, lint_template(d["template"], template_schema), d["expect_lint"])
    for p in sorted((HERE / "fixtures" / "runs").glob("*.json")):
        expect_valid(p, lint_run(load(p), run_schema))
    for p in sorted((HERE / "fixtures" / "invalid-runs").glob("*.json")):
        d = load(p)
        expect_invalid(p, lint_run(d["run"], run_schema), d["expect_lint"])
    for p in sorted((HERE / "fixtures" / "fleet").glob("*.json")):
        expect_valid(p, lint_fleet(load(p), fleet_schema))
    for p in sorted((HERE / "fixtures" / "invalid-fleet").glob("*.json")):
        d = load(p)
        expect_invalid(p, lint_fleet(d["fleet"], fleet_schema), d["expect_lint"])

    # Every rule ID documented must be exercised by at least one invalid fixture.
    exercised = set()
    for sub in ("invalid-templates", "invalid-runs", "invalid-fleet"):
        for p in (HERE / "fixtures" / sub).glob("*.json"):
            exercised |= set(load(p)["expect_lint"])
    for rule in ["L1", "L2", "L3", "L4", "L5", "L6", "L7", "L8", "L9", "R1", "R2", "R3", "R4", "R5", "F1", "F2", "F3"]:
        if rule not in exercised:
            failures.append(f"rule {rule} has no invalid fixture")

    for message in failures:
        print("FAIL", message)
    print(f"{counts['valid']} valid and {counts['invalid']} invalid fixtures checked; {len(failures)} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
