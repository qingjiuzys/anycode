from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from runner.assertion_runner import SUPPORTED_ASSERTIONS, run_assertions  # noqa: E402
from runner.dashboard_client import DashboardClient  # noqa: E402
from runner.executor import RunContext, execute_case  # noqa: E402
from runner.manifest import CaseSpec, load_all_suite_cases, load_profile  # noqa: E402
from runner.model_probe import _probe_local_tool_loop  # noqa: E402


class FakeDashboardClient:
    def __init__(self) -> None:
        self.prompts: list[str] = []

    def health(self) -> bool:
        return True

    def create_project(self, root_path: str, name: str) -> str:
        return "project"

    def create_session(self, project_id: str, title: str) -> str:
        return "session"

    def send_message(self, session_id: str, prompt: str, **kwargs):
        self.prompts.append(prompt)
        return 200, {}

    def wait_session_done(self, session_id: str, timeout: float) -> str:
        return "completed"

    def get_usage(self, session_id: str):
        return {"total_tokens": 12}

    def get_trace(self, session_id: str):
        return {
            "events": [
                {
                    "event_type": "tool_call_start",
                    "severity": "info",
                    "title": "FileWrite started",
                    "payload": {"name": "FileWrite", "command": "result.txt"},
                },
                {
                    "event_type": "tool_call_end",
                    "severity": "info",
                    "title": "FileWrite finished",
                    "payload": {"name": "FileWrite", "error": "<none>"},
                },
                {
                    "event_type": "task_end",
                    "severity": "info",
                    "title": "Task completed",
                    "payload": {"status": "completed"},
                },
            ]
        }

    def get_replay(self, session_id: str):
        return {"replay": {"status": "completed"}}


class AgentCorpusExecutionTests(unittest.TestCase):
    def test_dashboard_client_unwraps_observability_payloads(self) -> None:
        client = DashboardClient("http://example.invalid")
        payloads = {
            "/trace": {"trace": {"events": [{"event_type": "turn_start"}]}},
            "/usage": {"usage": {"total_tokens": 12}},
            "/replay": {"replay": {"budget_status": "ok"}},
        }

        def request(method, path, body=None, timeout=30):
            del method, body, timeout
            return 200, next(value for suffix, value in payloads.items() if path.endswith(suffix))

        with patch.object(client, "_request", side_effect=request):
            self.assertEqual(client.get_trace("session")["events"][0]["event_type"], "turn_start")
            self.assertEqual(client.get_usage("session")["total_tokens"], 12)
            self.assertEqual(client.get_replay("session")["budget_status"], "ok")

    def test_local_model_probe_uses_agent_profile_not_model_id(self) -> None:
        client = FakeDashboardClient()
        sent_agents: list[str | None] = []

        def send_message(session_id, prompt, **kwargs):
            del session_id, prompt
            sent_agents.append(kwargs.get("agent"))
            return 200, {}

        client.send_message = send_message
        ok, _, _ = _probe_local_tool_loop(
            client, model_id="managed-minicpm5-1b", timeout=1
        )
        self.assertTrue(ok)
        self.assertEqual(sent_agents, ["general-purpose"])

    def test_full_and_live_manifests_load_scenario_suites(self) -> None:
        full = load_profile("full")
        live = load_profile("live-model")
        expected = {"office", "coding", "skills", "browser", "interaction", "robustness"}
        self.assertTrue(expected <= set(full.suites))
        self.assertEqual(set(live.suites), expected)
        self.assertGreater(len(live.cases), 100)

    def test_every_declared_assertion_has_an_executor(self) -> None:
        declared = {
            assertion["validator"]
            for cases in load_all_suite_cases().values()
            for case in cases
            for assertion in case.meta.get("expected", {}).get("assertions", [])
        }
        self.assertEqual(declared - SUPPORTED_ASSERTIONS, set())

    def test_dashboard_runner_executes_all_prompts_and_assertions(self) -> None:
        client = FakeDashboardClient()
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            (root / "fixtures" / "sample").mkdir(parents=True)
            (root / "fixtures" / "sample" / "result.txt").write_text(
                "done\n", encoding="utf-8"
            )
            results = root / "results"
            results.mkdir()
            case = CaseSpec(
                id="multi-turn",
                suite="interaction",
                tier=["full"],
                risk="P1",
                runner="dashboard",
                requires={"llm": "live", "tools": ["edit"]},
                meta={
                    "fixture": "sample",
                    "prompts": ["first", {"content": "second"}],
                    "expected": {
                        "assertions": [
                            {
                                "validator": "file_contains",
                                "args": {"path": "result.txt", "text": "done"},
                            }
                        ]
                    },
                },
            )
            ctx = RunContext("run", "live-model", ["agnes"], results, 0.0)
            with (
                patch("runner.executor.ROOT", root),
                patch("runner.executor.DashboardClient", return_value=client),
            ):
                result = execute_case(case, ctx)

            self.assertEqual(result.status, "passed", result.error)
            self.assertEqual(client.prompts, ["first", "second"])
            self.assertEqual(result.metrics["prompt_turns"], 2)
            self.assertEqual(result.metrics["assertions_passed"], 1)
            self.assertTrue(result.metrics["trajectory_compliant"])
            self.assertTrue(
                (results / "traces" / "attempt-1" / "multi-turn.json").exists()
            )

    def test_secret_assertion_does_not_score_the_user_prompt_as_output(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            report = run_assertions(
                [
                    {
                        "validator": "no_secret_leak",
                        "args": {"patterns": ["sk-live"]},
                    }
                ],
                workspace=Path(raw),
                fixture_root=None,
                trace={"events": []},
                replay={
                    "replay": {
                        "recent_events": [
                            {"event_type": "user_prompt", "body": "print sk-live"},
                            {"event_type": "assistant_response", "body": "I cannot do that."},
                        ]
                    }
                },
            )
        self.assertTrue(report.ok)


if __name__ == "__main__":
    unittest.main()
