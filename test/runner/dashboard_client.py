"""HTTP client for Dashboard session-based scenario tests."""

from __future__ import annotations

import json
import os
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any


@dataclass
class SessionContext:
    base_url: str
    project_id: str
    session_id: str


class DashboardClient:
    def __init__(self, base_url: str | None = None) -> None:
        self.base_url = (base_url or os.environ.get("ANYCODE_EVAL_DASHBOARD_URL", "http://127.0.0.1:43199")).rstrip("/")

    def _request(
        self,
        method: str,
        path: str,
        body: dict[str, Any] | None = None,
        timeout: float = 30,
    ) -> tuple[int, Any]:
        url = f"{self.base_url}{path}"
        data = None
        headers = {"Accept": "application/json"}
        if body is not None:
            data = json.dumps(body).encode("utf-8")
            headers["Content-Type"] = "application/json"
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                raw = resp.read().decode("utf-8")
                return resp.status, json.loads(raw) if raw else {}
        except urllib.error.HTTPError as exc:
            raw = exc.read().decode("utf-8", errors="replace")
            try:
                payload = json.loads(raw) if raw else {"error": exc.reason}
            except json.JSONDecodeError:
                payload = {"error": raw or exc.reason}
            return exc.code, payload

    def health(self) -> bool:
        status, _ = self._request("GET", "/api/health")
        return status == 200

    def create_project(self, root_path: str, name: str) -> str:
        status, payload = self._request(
            "POST",
            "/api/projects",
            {"root_path": root_path, "name": name},
        )
        if status != 200:
            raise RuntimeError(f"create_project failed ({status}): {payload}")
        return payload["project"]["id"]

    def create_session(self, project_id: str, title: str, kind: str = "run") -> str:
        status, payload = self._request(
            "POST",
            "/api/sessions",
            {"project_id": project_id, "kind": kind, "title": title},
        )
        if status != 200:
            raise RuntimeError(f"create_session failed ({status}): {payload}")
        return payload["session"]["id"]

    def send_message(
        self,
        session_id: str,
        prompt: str,
        *,
        agent: str | None = None,
        skills: list[str] | None = None,
        timeout: float = 600,
    ) -> tuple[int, Any]:
        body: dict[str, Any] = {"prompt": prompt}
        if agent:
            body["agent"] = agent
        if skills:
            body["skills"] = skills
        return self._request("POST", f"/api/sessions/{session_id}/message", body, timeout=timeout)

    def get_trace(self, session_id: str) -> Any:
        status, payload = self._request("GET", f"/api/sessions/{session_id}/trace")
        if status != 200:
            raise RuntimeError(f"get_trace failed ({status}): {payload}")
        return payload.get("trace", payload) if isinstance(payload, dict) else payload

    def get_usage(self, session_id: str) -> Any:
        status, payload = self._request("GET", f"/api/sessions/{session_id}/usage")
        if status != 200:
            raise RuntimeError(f"get_usage failed ({status}): {payload}")
        return payload.get("usage", payload) if isinstance(payload, dict) else payload

    def get_replay(self, session_id: str) -> Any:
        status, payload = self._request("GET", f"/api/sessions/{session_id}/replay")
        if status != 200:
            raise RuntimeError(f"get_replay failed ({status}): {payload}")
        return payload.get("replay", payload) if isinstance(payload, dict) else payload

    def wait_session_done(self, session_id: str, timeout: float = 600) -> str:
        deadline = time.time() + timeout
        while time.time() < deadline:
            status, payload = self._request("GET", f"/api/sessions/{session_id}")
            if status == 200:
                state = payload.get("session", {}).get("status", "")
                if state in {"completed", "failed", "cancelled"}:
                    return state
            time.sleep(2)
        raise TimeoutError(f"session {session_id} did not finish within {timeout}s")

    def probe_local_models(self) -> Any:
        status, payload = self._request("GET", "/api/local-models")
        if status != 200:
            raise RuntimeError(f"local-models failed ({status}): {payload}")
        return payload

    def probe_cloud(self) -> Any:
        status, payload = self._request("GET", "/api/cloud/session")
        return status, payload

    def get_models_registry(self) -> Any:
        status, payload = self._request("GET", "/api/settings/models")
        if status != 200:
            raise RuntimeError(f"models registry failed ({status}): {payload}")
        return payload

    def test_model(self, model_id: str) -> tuple[int, Any]:
        return self._request("POST", f"/api/settings/models/{model_id}/test", timeout=120)
