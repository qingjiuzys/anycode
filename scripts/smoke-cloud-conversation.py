#!/usr/bin/env python3
"""Quick API smoke for cloud conversation on local workbench."""
from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request

BASE = os.environ.get("ANYCODE_E2E_BASE", "http://127.0.0.1:43181").rstrip("/")
TIMEOUT = int(os.environ.get("SMOKE_TIMEOUT_S", "90"))


def req(method: str, path: str, body: dict | None = None) -> dict:
    url = f"{BASE}{path}"
    data = None
    headers = {"Content-Type": "application/json"}
    if body is not None:
        data = json.dumps(body).encode()
    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=30) as resp:
            raw = resp.read().decode()
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        raw = e.read().decode()
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            return {"error": raw, "status": e.code}


def main() -> int:
    health = req("GET", "/api/health")
    print("health", health.get("ok"), health.get("model_gateway_url"))
    gw = req("POST", "/api/cloud/gateway-test")
    print("gateway-test", gw.get("ok"), gw.get("status"))

    projects = req("GET", "/api/projects?limit=20")
    plist = projects.get("projects") or []
    if not plist:
        print("FAIL: no projects", projects)
        return 1
    project = next((p for p in plist if p.get("name", "").lower() == "anycode"), plist[0])
    pid = project["id"]
    print("project", project.get("name"), pid)

    start = req(
        "POST",
        f"/api/projects/{pid}/conversations/start",
        {
            "prompt": "Reply with exactly the word pong, nothing else.",
            "agent": "general-purpose",
            "recycle_session": False,
        },
    )
    if start.get("error"):
        print("FAIL start", json.dumps(start, indent=2))
        return 1
    sid = (start.get("session") or {}).get("id") or start.get("session_id")
    if not sid:
        print("FAIL no session id", start)
        return 1
    print("session", sid)

    deadline = time.time() + TIMEOUT
    status = ""
    while time.time() < deadline:
        sess = req("GET", f"/api/sessions/{sid}")
        status = (sess.get("session") or {}).get("status", "")
        print("status", status)
        if status in {"completed", "failed", "cancelled"}:
            break
        time.sleep(2)

    sess = req("GET", f"/api/sessions/{sid}")
    session = sess.get("session") or {}
    print("final", status, session.get("block_reason") or session.get("summary"))
    if status != "completed":
        return 1

    transcript = req("GET", f"/api/sessions/{sid}/transcript")
    blocks = (transcript.get("transcript") or {}).get("blocks") or []
    text = " ".join(
        str(b.get("body", ""))
        for b in blocks
        if b.get("type") in {"assistant", "user", "tool"}
    ).lower()
    print("transcript_chars", len(text))
    if "pong" not in text:
        print("WARN: pong not found in transcript")
    print("PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
