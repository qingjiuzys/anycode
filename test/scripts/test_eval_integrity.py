from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


evalplus_runner = load_module("run_agnes_evalplus", ROOT / "test/scripts/run_agnes_evalplus.py")
complex_verify = load_module(
    "complex_verify",
    ROOT / "test/e2e-delivery-chain/shared/utils/complex_verify.py",
)


class EvalPlusResumeTests(unittest.TestCase):
    def test_empty_and_http_error_samples_are_not_resumed(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "samples.jsonl"
            records = [
                {"task_id": "HumanEval/0", "solution": "def f():\n    return 1"},
                {"task_id": "HumanEval/1", "solution": "  "},
                {"task_id": "HumanEval/2", "solution": "HTTP error 502 Bad Gateway"},
            ]
            path.write_text(
                "".join(json.dumps(record) + "\n" for record in records),
                encoding="utf-8",
            )
            stats = evalplus_runner.clean_resume_samples(path)
            self.assertEqual(stats, {"valid": 1, "empty": 1, "http_error": 1, "malformed": 0})
            remaining = [json.loads(line) for line in path.read_text().splitlines()]
            self.assertEqual([record["task_id"] for record in remaining], ["HumanEval/0"])
            self.assertTrue(path.with_suffix(".invalid.jsonl").is_file())
            stats_path = path.with_suffix(".generation-stats.json")
            self.assertEqual(
                evalplus_runner.update_failure_stats(stats_path, stats),
                {"empty": 1, "http_error": 1, "malformed": 0},
            )
            self.assertEqual(
                evalplus_runner.update_failure_stats(stats_path, {"http_error": 1}),
                {"empty": 1, "http_error": 2, "malformed": 0},
            )


class ComplexVerifierTruthTests(unittest.TestCase):
    def test_manifest_claims_must_match_live_truth(self) -> None:
        manifest = {
            "eval_run_id": "run-a",
            "eval_profile": "agnes",
            "cargo_tests_passed": True,
            "git_head": "abcdef1",
        }
        self.assertEqual(
            complex_verify.verify_manifest_runtime_truth(
                manifest,
                cargo_passed=True,
                live_git_head="abcdef1234567890",
                run_id="run-a",
                profile="agnes",
            ),
            [],
        )
        findings = complex_verify.verify_manifest_runtime_truth(
            manifest,
            cargo_passed=False,
            live_git_head="9999999999999999",
            run_id="run-b",
            profile="local-1b",
        )
        self.assertEqual(
            {finding.code for finding in findings},
            {
                "manifest_run_id",
                "manifest_profile",
                "manifest_cargo_truth",
                "manifest_git_truth",
            },
        )
        self.assertTrue(all(finding.severity == "P0" for finding in findings))

    def test_artifact_hash_and_run_ownership_are_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            workspace = Path(tmp)
            artifacts = []
            for relative in complex_verify.V2["required_artifacts"]:
                path = workspace / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(relative.encode())
                artifacts.append(
                    {
                        "path": relative,
                        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                        "fresh": True,
                    }
                )
            owner_path = workspace / "owner.json"
            owner_path.write_text(
                json.dumps(
                    {
                        "evalRunId": "run-a",
                        "modelProfile": "agnes",
                        "sessionId": "session-a",
                        "workspace": str(workspace),
                        "artifacts": artifacts,
                    }
                )
            )
            findings, _ = complex_verify.verify_ownership(
                workspace, owner_path, "run-a", "agnes", "session-a"
            )
            self.assertEqual(findings, [])
            (workspace / complex_verify.V2["required_artifacts"][0]).write_bytes(b"polluted")
            findings, _ = complex_verify.verify_ownership(
                workspace, owner_path, "run-a", "agnes", "session-a"
            )
            self.assertIn("artifact_owner_mismatch", {finding.code for finding in findings})


if __name__ == "__main__":
    unittest.main()
