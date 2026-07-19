#!/usr/bin/env python3
"""Run daily conversation scenario matrix against local cloud workbench.

All deliverables land under test/out/<run-id>/ (JSON + artifacts/), never the repo root.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = os.environ.get("ANYCODE_E2E_BASE", "http://127.0.0.1:43181").rstrip("/")
DEFAULT_TIMEOUT = int(os.environ.get("SCENARIO_TIMEOUT_S", "300"))
RUN_ID = os.environ.get(
    "EVAL_RUN_ID", f"daily-conversation-eval-{date.today().isoformat()}"
)
RUN_DIR = ROOT / "test" / "out" / RUN_ID
ARTIFACT_REL = Path("test") / "out" / RUN_ID / "artifacts"
ARTIFACT_DIR = ROOT / ARTIFACT_REL
OUT = RUN_DIR / "daily-conversation-eval.json"


@dataclass
class Scenario:
    id: str
    agent: str
    skills: list[str]
    prompt: str
    timeout_s: int = DEFAULT_TIMEOUT
    expect_tools: list[str] = field(default_factory=list)
    expect_artifacts_min: int = 0
    # Paths relative to repo ROOT (under test/out/.../artifacts).
    expect_paths: list[str] = field(default_factory=list)


def _p(*parts: str) -> str:
    """Repo-relative artifact path for prompts / expect_paths."""
    return str(ARTIFACT_REL.joinpath(*parts)).replace("\\", "/")


def build_scenarios() -> list[Scenario]:
    art = _p()
    return [
        Scenario(
            id="coding",
            agent="general-purpose",
            skills=[],
            prompt=(
                "Stay inside docs/ops only. Use Glob with path=docs/ops and pattern=*.md, "
                "then Grep once in docs/ops for 'eval'. Reply with a 3-line summary."
            ),
            expect_tools=["Glob", "Grep"],
            timeout_s=180,
        ),
        Scenario(
            id="ppt",
            agent="office-writer",
            skills=["office-pptx"],
            prompt=(
                f"Create a minimal 8-slide .pptx about anyCode daily brief. "
                f"Save ONLY to {_p('ppt', 'daily-brief.pptx')} "
                f"(do not write to repo root). Use the office-pptx skill."
            ),
            timeout_s=420,
            expect_artifacts_min=1,
            expect_paths=[_p("ppt", "daily-brief.pptx")],
        ),
        Scenario(
            id="pdf",
            agent="office-writer",
            skills=["md-to-pdf"],
            prompt=(
                f"Write {_p('pdf', 'brief.md')} with 3 bullets grounded in real anyCode "
                f"docs (not invented metrics), then use md-to-pdf to produce "
                f"{_p('pdf', 'brief.pdf')}. Do not write to repo root."
            ),
            timeout_s=300,
            expect_artifacts_min=1,
            expect_paths=[_p("pdf", "brief.pdf")],
        ),
        Scenario(
            id="image",
            agent="general-purpose",
            skills=[],
            prompt=(
                f"Generate a simple 16:9 product icon using GenerateImage. "
                f"If generation succeeds, save under {art}/image/ and reply with the path. "
                f"If it fails, clearly state the error — do not claim success."
            ),
            timeout_s=240,
        ),
        Scenario(
            id="video",
            agent="general-purpose",
            skills=["video-script"],
            prompt=(
                f"Create {_p('video', 'script.md')} with 4 shots for a 60s explainer. "
                f"Try GenerateImage for one storyboard under {_p('video', 'assets')}/. "
                f"If image gen fails, mark asset status as FAILED (never invent paths). "
                f"Do not write to repo-root video/."
            ),
            timeout_s=420,
            expect_paths=[_p("video", "script.md")],
        ),
        Scenario(
            id="skill-trace",
            agent="office-writer",
            skills=["doc-summary"],
            prompt=(
                "Using the doc-summary skill, Read docs/ops/scenario-eval.md and write "
                f"{_p('skill-trace', 'document-summary.md')} with Purpose / Key points / "
                "Risks / Actions. Cite the source path. Do not write to repo-root reports/."
            ),
            timeout_s=240,
            expect_artifacts_min=1,
            expect_paths=[_p("skill-trace", "document-summary.md")],
        ),
        Scenario(
            id="daily-brief",
            agent="office-writer",
            skills=["daily-brief"],
            prompt=(
                f"Using the daily-brief skill and WebSearch, write {_p('daily-brief', 'brief-eval.md')} "
                "about AI coding agents. Include focus, 3 priorities, one risk, and a Sources "
                "section with real URLs obtained from tools. Mark offline draft if search fails. "
                "Do not write to repo root."
            ),
            timeout_s=300,
            expect_artifacts_min=1,
            expect_paths=[_p("daily-brief", "brief-eval.md")],
        ),
    ]


SCENARIOS = build_scenarios()


def req(method: str, path: str, body: dict | None = None, timeout: int = 30) -> dict:
    url = f"{BASE}{path}"
    data = json.dumps(body).encode() if body is not None else None
    headers = {"Content-Type": "application/json"} if body is not None else {}
    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=timeout) as resp:
            raw = resp.read().decode()
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        raw = e.read().decode()
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            return {"error": raw, "status": e.code}
    except urllib.error.URLError as e:
        return {"error": str(e)}


def ensure_workbench() -> None:
    # Only manage the standalone workbench port; never touch the desktop app on :43180.
    if BASE.rstrip("/").endswith(":43180"):
        health = req("GET", "/api/health")
        if not health.get("ok"):
            raise RuntimeError(f"desktop workbench not healthy at {BASE}")
        return
    script = ROOT / "scripts" / "start-local-workbench.sh"
    status = subprocess.run([str(script), "status"], capture_output=True, text=True)
    if status.returncode != 0:
        subprocess.run([str(script), "restart"], check=True)


def pick_project() -> str:
    data = req("GET", "/api/projects?limit=30")
    projects = data.get("projects") or []
    for name in ("anycode", "e2e-delivery"):
        for p in projects:
            if p.get("name", "").lower() == name:
                return p["id"]
    if not projects:
        raise RuntimeError(f"no projects from {BASE}/api/projects — is workbench up?")
    return projects[0]["id"]


def approve_pending_for_session(session_id: str) -> int:
    """Desktop/web chat ignores shell ANYCODE_IGNORE_APPROVAL; auto-allow for eval."""
    data = req("GET", "/api/security/approvals/pending?limit=50")
    pending = data.get("pending") or []
    n = 0
    for item in pending:
        if item.get("session_id") != session_id:
            continue
        aid = item.get("approval_id")
        if not aid:
            continue
        resp = req(
            "POST",
            f"/api/security/approvals/{aid}/respond",
            {"decision": "allow_all_session"},
        )
        if resp.get("ok") or resp.get("decision"):
            n += 1
    return n


def transcript_text(session_id: str) -> str:
    data = req("GET", f"/api/sessions/{session_id}/transcript")
    blocks = (data.get("transcript") or {}).get("blocks") or []
    parts: list[str] = []
    for b in blocks:
        t = b.get("block_type") or b.get("type")
        if t == "tool_cluster":
            for step in b.get("steps") or []:
                parts.append(str(step.get("tool_name") or step.get("label") or ""))
        elif t == "tool_call":
            parts.append(str((b.get("meta") or {}).get("name") or ""))
        else:
            parts.append(str(b.get("body") or b.get("text") or ""))
    return "\n".join(parts)


def run_scenario(project_id: str, sc: Scenario) -> dict:
    started = time.time()
    start = req(
        "POST",
        f"/api/projects/{project_id}/conversations/start",
        {
            "prompt": sc.prompt,
            "agent": sc.agent,
            "skills": sc.skills,
            "recycle_session": False,
        },
    )
    sid = (start.get("session") or {}).get("id") or start.get("session_id")
    if not sid:
        return {
            "id": sc.id,
            "status": "start_failed",
            "error": start,
            "duration_s": round(time.time() - started, 1),
        }

    deadline = time.time() + sc.timeout_s
    status = "running"
    while time.time() < deadline:
        approved = approve_pending_for_session(sid)
        if approved:
            print(f"    auto-approved {approved} pending tool(s)", flush=True)
        sess = req("GET", f"/api/sessions/{sid}")
        status = (sess.get("session") or {}).get("status", "")
        if status in {"completed", "failed", "cancelled"}:
            break
        time.sleep(2)

    sess = req("GET", f"/api/sessions/{sid}")
    session = sess.get("session") or {}
    text = transcript_text(sid)
    tools_seen = [t for t in sc.expect_tools if t.lower() in text.lower()]
    req("POST", f"/api/sessions/{sid}/scan-artifacts")
    arts = req("GET", f"/api/sessions/{sid}/artifacts")
    artifacts = arts.get("artifacts") or arts.get("items") or []
    artifact_count = len(artifacts) if isinstance(artifacts, list) else 0
    paths_found = [p for p in sc.expect_paths if (ROOT / p).is_file()]
    # Session scan can miss fresh files; filesystem paths are a hard delivery check.
    delivery_count = max(artifact_count, len(paths_found))
    if sc.expect_artifacts_min > 0:
        artifact_ok = delivery_count >= sc.expect_artifacts_min
    elif sc.expect_paths:
        artifact_ok = len(paths_found) >= len(sc.expect_paths)
    else:
        artifact_ok = True
    tools_ok = all(t in tools_seen for t in sc.expect_tools) if sc.expect_tools else True
    hard_pass = status == "completed" and artifact_ok and tools_ok
    quality_flags: list[str] = []
    low = text.lower()
    if sc.id == "image":
        if "402" in low or "payment required" in low or "subscription_not_found" in low:
            quality_flags.append("image_gen_payment_or_subscription_failure")
        elif "generateimage" in low and not paths_found:
            # succeeded in transcript terms but no file under expect/art dir
            if "failed" in low or "error" in low:
                quality_flags.append("image_gen_failed")
    if sc.id == "video":
        # Catch hallucinated checkmarks for missing local assets.
        for line in text.splitlines():
            if "✅" in line or "✓" in line:
                for token in line.replace("`", " ").split():
                    if token.endswith((".png", ".jpg", ".jpeg", ".webp")):
                        cand = token.lstrip("./")
                        if not (ROOT / cand).is_file() and not (
                            ARTIFACT_DIR / "video" / "assets" / Path(cand).name
                        ).is_file():
                            quality_flags.append(f"claimed_missing_asset:{cand}")
    quality_pass = hard_pass and not quality_flags
    return {
        "id": sc.id,
        "session_id": sid,
        "status": status,
        "duration_s": round(time.time() - started, 1),
        "block_reason": session.get("block_reason") or session.get("summary"),
        "tools_seen": tools_seen,
        "tools_expected": sc.expect_tools,
        "artifact_count": artifact_count,
        "paths_found": paths_found,
        "artifacts_ok": artifact_ok,
        "expect_artifacts_min": sc.expect_artifacts_min,
        "transcript_excerpt": text[:500],
        "pass": hard_pass,
        "quality_pass": quality_pass,
        "quality_flags": quality_flags,
    }


def main() -> int:
    ensure_workbench()
    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)
    for sub in ("ppt", "pdf", "image", "video/assets", "skill-trace", "daily-brief"):
        (ARTIFACT_DIR / sub).mkdir(parents=True, exist_ok=True)

    health = req("GET", "/api/health")
    gw = req("POST", "/api/cloud/gateway-test")
    project_id = pick_project()
    results = {
        "base": BASE,
        "run_id": RUN_ID,
        "run_dir": str(RUN_DIR.relative_to(ROOT)),
        "artifact_dir": str(ARTIFACT_REL),
        "health": health,
        "gateway_test": {"ok": gw.get("ok"), "status": gw.get("status")},
        "project_id": project_id,
        "scenarios": [],
    }
    only = os.environ.get("SCENARIO_ONLY", "").strip()
    for sc in SCENARIOS:
        if only and sc.id != only:
            continue
        print(f"==> {sc.id}", flush=True)
        results["scenarios"].append(run_scenario(project_id, sc))
        print(json.dumps(results["scenarios"][-1], indent=2), flush=True)

    passed = sum(1 for r in results["scenarios"] if r.get("pass"))
    quality_passed = sum(1 for r in results["scenarios"] if r.get("quality_pass"))
    results["summary"] = {
        "passed": passed,
        "quality_passed": quality_passed,
        "total": len(results["scenarios"]),
        "pass_rate": passed / max(len(results["scenarios"]), 1),
        "quality_pass_rate": quality_passed / max(len(results["scenarios"]), 1),
        "note": "pass = session+artifacts/tools; quality_pass adds anti-hallucination gates (image/video)",
    }
    RUN_DIR.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(results, indent=2) + "\n")
    # Compat symlink/copy pointer for older docs
    compat = ROOT / "test" / "out" / "daily-conversation-eval.json"
    compat.write_text(OUT.read_text())
    print("wrote", OUT)
    print("summary", results["summary"])
    return 0 if passed == len(results["scenarios"]) else 1


if __name__ == "__main__":
    sys.exit(main())
