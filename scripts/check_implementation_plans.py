"""Validate plan/registry parity and the catalog DAG without changing live state."""
import argparse
import json
from pathlib import Path
import re

from plan_coordinator import ROOT, load_plans


def main():
    directory = ROOT / "specs/plans"
    plans, _ = load_plans(argparse.Namespace(
        plan=directory / "00-contracts.steps.json", catalog=directory / "catalog.json", all=True))
    total = 0
    for pid, (plan, _, _) in plans.items():
        matches = [directory / plan["document"]] if plan.get("document") else list(directory.glob(pid.split("-", 1)[0] + "-*-implementation.md"))
        assert len(matches) == 1, f"Missing/ambiguous document for {pid}"
        document = matches[0].read_text(encoding="utf-8")
        sections = {m[1]: (m[2], m[3]) for m in re.finditer(
            r"^## (\d{2}-\d{2}) — ([^\n]+)\n(.*?)(?=^## |\Z)", document, re.M | re.S)}
        assert list(sections) == [s["id"] for s in plan["steps"]], f"Step list mismatch: {pid}"
        for step in plan["steps"]:
            title, body = sections[step["id"]]
            assert "Acceptance:" in body, step["id"]
            assert len(re.findall(r"^\d+\. ", body, re.M)) >= 3, step["id"]
            if pid.startswith("00-"):
                continue  # Existing detailed plan uses prose prerequisite descriptions.
            assert title == step["title"], step["id"]
            dependencies = re.search(r"Dependencies: (.*?)\. Resources:", body)[1]
            resources = re.search(r"Resources: (.*?)\.", body)[1]
            assert re.findall(r"`([^`]+)`", dependencies) == step["depends_on"], step["id"]
            assert re.findall(r"`([^`]+)`", resources) == step["resources"], step["id"]
            opt_in = re.search(r"Opt-in: `([^`]+)`", body)
            assert (opt_in[1] if opt_in else None) == step.get("opt_in"), step["id"]
        total += len(plan["steps"])
        print(f"{pid}: {len(plan['steps'])} steps verified")
    print(f"Verified {len(plans)} plans, {total} steps, matching documents and acyclic dependencies.")


if __name__ == "__main__":
    main()
