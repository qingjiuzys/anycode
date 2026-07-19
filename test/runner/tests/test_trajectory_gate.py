from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from runner.trajectory_gate import evaluate_trajectory  # noqa: E402


def event(event_type: str, **payload: str) -> dict:
    return {
        "event_type": event_type,
        "severity": "info",
        "payload": payload,
        "title": event_type,
    }


class TrajectoryGateTests(unittest.TestCase):
    def test_accepts_structured_tool_trajectory(self) -> None:
        trace = {
            "events": [
                event("tool_call_start", name="Glob", command="*.rs"),
                event("tool_call_end", name="Glob", error="<none>"),
                event("task_end", status="completed"),
            ]
        }
        result = evaluate_trajectory(trace, required_tools=["search"])
        self.assertTrue(result.ok)
        self.assertEqual(result.metrics["tool_calls"], 1)

    def test_rejects_forbidden_repeated_and_budget_violations(self) -> None:
        call = event("tool_call_start", name="Bash", command="cat /etc/passwd")
        trace = {"events": [call, call, call, event("budget_exceeded")]}
        result = evaluate_trajectory(
            trace,
            required_tools=["shell"],
            policy={"forbidden_tools": ["Bash"], "max_identical_calls": 1},
        )
        self.assertFalse(result.ok)
        self.assertTrue(any("forbidden" in item for item in result.violations))
        self.assertTrue(any("repeated" in item for item in result.violations))
        self.assertTrue(any("budget" in item for item in result.violations))
        self.assertTrue(any("dangerous path" in item for item in result.violations))

    def test_distinguishes_expected_refusal_from_silent_no_tool(self) -> None:
        trace = {"events": [event("task_end", status="completed")]}
        self.assertFalse(evaluate_trajectory(trace, required_tools=["edit"]).ok)
        self.assertTrue(
            evaluate_trajectory(trace, required_tools=["edit"], allow_zero_tools=True).ok
        )

    def test_reads_index_only_budget_state_from_replay(self) -> None:
        trace = {"events": [event("tool_call_start", name="Glob", command="*.rs")]}
        replay = {"replay": {"budget_status": "exceeded", "recent_events": []}}
        result = evaluate_trajectory(trace, replay=replay, required_tools=["search"])
        self.assertFalse(result.ok)
        self.assertTrue(result.metrics["budget_exceeded"])


if __name__ == "__main__":
    unittest.main()
