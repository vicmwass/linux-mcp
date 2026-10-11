"""Coordination tests run in isolated temporary stores, never live plan state."""
import concurrent.futures
import json
from pathlib import Path
import subprocess
import sys
import shutil
import uuid
import unittest

CLI = Path(__file__).with_name("plan_coordinator.py")


class CoordinatorTests(unittest.TestCase):
    def setUp(self):
        test_root = (CLI.parent.parent / "specs/plans/.coord").resolve()
        test_root.mkdir(parents=True, exist_ok=True)
        self.root = test_root / ("test-" + uuid.uuid4().hex)
        self.root.mkdir()
        # Validate the resolved recursive cleanup target stays in the test root.
        if self.root.resolve().parent != test_root:
            raise RuntimeError("Unsafe test cleanup path")
        self.addCleanup(shutil.rmtree, self.root)
        self.plan = self.root / "plan.json"
        self.write_plan("test", [
            {"id": "a", "title": "A", "depends_on": [], "resources": ["shared"]},
            {"id": "b", "title": "B", "depends_on": ["a"], "resources": ["b"]},
            {"id": "c", "title": "C", "depends_on": [], "resources": ["shared"]},
        ])

    def write_plan(self, name, steps, path=None):
        (path or self.plan).write_text(json.dumps({"plan_id": name, "version": 1, "steps": steps}), encoding="utf-8")

    def call(self, *args, plan=None, succeeds=True):
        result = subprocess.run([sys.executable, str(CLI), "--plan", str(plan or self.plan),
                                 "--state-dir", str(self.root / "state"), *args],
                                capture_output=True, text=True, timeout=15)
        if succeeds:
            self.assertEqual(result.returncode, 0, result.stderr)
            return json.loads(result.stdout)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        return result.stderr

    def test_dependencies_ownership_completion_and_next(self):
        self.call("claim", "--owner", "agent", "--step", "b", succeeds=False)
        claim = self.call("claim", "--owner", "agent")
        self.assertEqual(claim["step"], "a")
        self.call("claim", "--owner", "agent", succeeds=False)
        self.call("complete", "--step", "a", "--token", "wrong", "--evidence", "tested", succeeds=False)
        self.call("heartbeat", "--step", "a", "--token", claim["token"], "--note", "working")
        self.call("complete", "--step", "a", "--token", claim["token"], "--evidence", "checks passed")
        self.assertEqual(self.call("claim", "--owner", "agent")["step"], "b")
        status = self.call("status")
        self.assertEqual(status["steps"][0]["evidence"], "checks passed")

    def test_simultaneous_claims_and_resource_exclusion(self):
        def invoke(index):
            return subprocess.run([sys.executable, str(CLI), "--plan", str(self.plan),
                                   "--state-dir", str(self.root / "state"), "claim", "--owner", f"agent-{index}"],
                                  capture_output=True, text=True, timeout=15)
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
            results = list(pool.map(invoke, range(8)))
        winners = [r for r in results if r.returncode == 0]
        self.assertEqual(len(winners), 1)
        self.assertEqual(json.loads(winners[0].stdout)["step"], "a")
        self.assertEqual(sum(s["status"] == "claimed" for s in self.call("status")["steps"]), 1)

    def test_independent_steps_claim_in_parallel(self):
        self.write_plan("test", [
            {"id": "a", "title": "A", "depends_on": [], "resources": ["a"]},
            {"id": "b", "title": "B", "depends_on": [], "resources": ["b"]},
        ])
        self.assertEqual(self.call("claim", "--owner", "one")["step"], "a")
        self.assertEqual(self.call("claim", "--owner", "two")["step"], "b")

    def test_block_release_and_recovery(self):
        claim = self.call("claim", "--owner", "one")
        self.call("block", "--step", "a", "--token", claim["token"], "--reason", "missing input", "--evidence", "attempt recorded")
        self.call("unblock", "--step", "a", "--reason", "input restored")
        claim = self.call("claim", "--owner", "two")
        self.call("recover", "--step", "a", "--token", claim["token"], "--reason", "worker stopped and verified")
        new = self.call("claim", "--owner", "three")
        self.assertNotEqual(new["token"], claim["token"])
        self.call("heartbeat", "--step", "a", "--token", claim["token"], succeeds=False)
        self.call("release", "--step", "a", "--token", new["token"], "--reason", "handoff")
        self.assertEqual(self.call("status")["steps"][0]["status"], "pending")

    def test_resources_shared_across_plans(self):
        self.call("claim", "--owner", "one")
        second = self.root / "other.json"
        self.write_plan("other", [{"id": "x", "title": "X", "depends_on": [], "resources": ["shared"]}], second)
        self.call("claim", "--owner", "two", plan=second, succeeds=False)

    def test_registry_drift_corruption_and_cycles_fail_closed(self):
        self.call("status")
        self.plan.write_text(self.plan.read_text() + "\n")
        self.call("claim", "--owner", "one", succeeds=False)
        (self.root / "state/state.json").write_text("corrupt")
        self.call("status", succeeds=False)
        self.write_plan("cycle", [{"id": "a", "title": "A", "depends_on": ["a"], "resources": ["a"]}])
        self.call("status", succeeds=False)

    def catalog(self, dependency="test/a"):
        second = self.root / "other.json"
        self.write_plan("other", [{"id": "x", "title": "X", "depends_on": [dependency], "resources": ["x"]}], second)
        (self.root / "catalog.json").write_text(json.dumps({"version": 1, "plans": ["plan.json", "other.json"]}), encoding="utf-8")
        return second

    def test_cross_plan_dependency_blocks_until_completed(self):
        second = self.catalog()
        status = self.call("status", plan=second)
        self.assertEqual(status["steps"][0]["waiting_for"], ["test/a"])
        self.call("claim", "--owner", "two", plan=second, succeeds=False)
        claim = self.call("claim", "--owner", "one", "--step", "a")
        self.call("claim", "--owner", "two", plan=second, succeeds=False)
        self.call("complete", "--step", "a", "--token", claim["token"], "--evidence", "passed")
        self.assertEqual(self.call("claim", "--owner", "two", plan=second)["step"], "x")

    def test_missing_cross_plan_dependency_and_cycle_refused(self):
        second = self.catalog("missing/a")
        self.call("status", plan=second, succeeds=False)
        self.catalog()
        self.write_plan("test", [{"id": "a", "title": "A", "depends_on": ["other/x"], "resources": ["a"]}])
        self.call("status", plan=second, succeeds=False)

    def test_document_reservation_excludes_other_workers(self):
        claim = self.call("claim", "--owner", "one", "--step", "a")
        self.call("reserve-doc", "--step", "a", "--token", claim["token"])
        self.call("complete", "--step", "a", "--token", claim["token"], "--evidence", "passed")
        first = self.call("claim", "--owner", "two", "--step", "b")
        second = self.call("claim", "--owner", "three", "--step", "c")
        self.call("reserve-doc", "--step", "b", "--token", first["token"])
        self.call("reserve-doc", "--step", "c", "--token", second["token"], succeeds=False)

    def test_global_status_and_claim(self):
        self.catalog()
        status = self.call("--all", "status")
        self.assertEqual(len(status["steps"]), 4)
        claim = self.call("--all", "claim", "--owner", "one", "--step", "test/a")
        self.assertEqual(claim["plan"], "test")

    def test_optional_steps_require_selection(self):
        self.write_plan("test", [{"id": "a", "title": "A", "depends_on": [], "resources": ["a"], "opt_in": "webmcp"}])
        self.assertTrue(self.call("status")["steps"][0]["opt_in_disabled"])
        self.call("claim", "--owner", "one", succeeds=False)
        claim = self.call("--enable", "webmcp", "claim", "--owner", "one")
        self.assertEqual(claim["step"], "a")

    def test_claim_survives_catalog_extension(self):
        claim = self.call("claim", "--owner", "one")
        self.catalog()
        heartbeat = self.call("heartbeat", "--step", "a", "--token", claim["token"])
        self.assertEqual(heartbeat["owner"], "one")
        self.call("complete", "--step", "a", "--token", claim["token"], "--evidence", "passed")

    def finish(self, step, owner="worker", plan=None):
        claim = self.call("claim", "--owner", owner, "--step", step, plan=plan)
        self.call("complete", "--step", step, "--token", claim["token"], "--evidence", "original checks passed", plan=plan)
        return claim

    def test_review_and_reopen_preserve_history_and_cascade(self):
        second = self.catalog()
        original = self.finish("a")
        self.finish("b")
        self.finish("x", plan=second)
        self.call("review", "--step", "a", "--reviewer", "human", "--reason", "Revise behavior")
        reopened = self.call("reopen", "--step", "a", "--reason", "Review requires correction")
        self.assertEqual(reopened["status"], "pending")
        self.assertEqual(reopened["attempt"], 2)
        self.assertEqual(reopened["previous_completions"][0]["evidence"], "original checks passed")
        self.assertEqual(reopened["reviews"][0]["reviewer"], "human")
        self.assertEqual(set(reopened["reopened_steps"]), {"test/a", "test/b", "other/x"})
        self.call("claim", "--owner", "dependent", "--step", "x", plan=second, succeeds=False)
        self.call("complete", "--step", "a", "--token", original["token"], "--evidence", "stale", succeeds=False)
        self.finish("a")
        self.assertTrue(self.call("status", plan=second)["steps"][0]["eligible"])

    def test_reopen_refuses_active_dependent_without_changing_state(self):
        self.finish("a")
        claim = self.call("claim", "--owner", "dependent", "--step", "b")
        self.call("reopen", "--step", "a", "--reason", "Needs correction", succeeds=False)
        records = {s["id"]: s for s in self.call("status")["steps"]}
        self.assertEqual(records["a"]["status"], "completed")
        self.assertEqual(records["b"]["token"], claim["token"])
        self.call("review", "--step", "b", "--reviewer", "reviewer", "--reason", "Feedback while active")
        self.call("heartbeat", "--step", "b", "--token", claim["token"])

    def test_reopen_does_not_touch_unrelated_completed_step(self):
        self.finish("a")
        self.finish("c")
        self.call("reopen", "--step", "a", "--reason", "Change A only")
        records = {s["id"]: s for s in self.call("status")["steps"]}
        self.assertEqual(records["c"]["status"], "completed")


if __name__ == "__main__":
    unittest.main()
