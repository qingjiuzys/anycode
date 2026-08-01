#!/usr/bin/env python3
"""Run skill bake-off cases against anycode-dashboard-serve + deepseek-v4-flash."""
from __future__ import annotations

import argparse
import json
import os
import shutil
import signal
import subprocess
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

BAKEOFF = Path(__file__).resolve().parents[1]
ROOT = BAKEOFF.parents[1]
CANDIDATES = BAKEOFF / "skills-candidates"
BASELINES = CANDIDATES / "_baselines"
PROMPTS = BAKEOFF / "fixtures" / "prompts.json"
CONFIG = Path.home() / ".anycode" / "config.json"


def req(method: str, url: str, body: dict | None = None, timeout: float = 60) -> tuple[int, dict]:
    data = None
    headers = {"Accept": "application/json"}
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=timeout) as resp:
            raw = resp.read().decode()
            return resp.status, (json.loads(raw) if raw else {})
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", errors="replace")
        try:
            payload = json.loads(raw) if raw else {"error": e.reason}
        except json.JSONDecodeError:
            payload = {"error": raw or e.reason}
        return e.code, payload
    except urllib.error.URLError as e:
        return 0, {"error": str(e)}


def link_skills_into_workspace(workspace: Path) -> None:
    """Copy bake-off candidates (+ baselines) into `<workspace>/skills`.

    Uses real directories (not symlinks) so older dashboard builds that skip
    symlink dirs still discover skills during validation.
    """
    dest = workspace / "skills"
    if dest.exists():
        shutil.rmtree(dest)
    dest.mkdir(parents=True)
    sources: list[Path] = []
    for child in sorted(CANDIDATES.iterdir()):
        if child.is_dir() and child.name != "_baselines" and (child / "SKILL.md").is_file():
            sources.append(child)
    if BASELINES.is_dir():
        for child in sorted(BASELINES.iterdir()):
            if child.is_dir() and (child / "SKILL.md").is_file():
                sources.append(child)
    for src in sources:
        shutil.copytree(
            src,
            dest / src.name,
            ignore=shutil.ignore_patterns(".git", "node_modules", "__pycache__", ".DS_Store"),
            dirs_exist_ok=True,
        )
    print(f"copied {len(sources)} skills -> {dest}", flush=True)


def _is_bakeoff_extra(path: str) -> bool:
    p = path.replace("\\", "/")
    return "eval/skill-bakeoff" in p or "skills-candidates" in p


def patch_extra_dirs(extra: list[str]) -> list:
    """Return previous non-bakeoff extra_dirs for restore."""
    cfg = json.loads(CONFIG.read_text(encoding="utf-8"))
    skills = cfg.setdefault("skills", {})
    raw_prev = skills.get("extra_dirs") or []
    # Ignore leftover bakeoff paths from a killed prior run.
    prev = [p for p in raw_prev if isinstance(p, str) and not _is_bakeoff_extra(p)]
    skills["enabled"] = True
    skills["extra_dirs"] = extra
    # Ensure active chat model is flash for this eval.
    models = cfg.setdefault("models", {})
    active = models.setdefault("active", {})
    active["chat"] = "deepseek-v4-flash"
    cfg["provider"] = "deepseek"
    cfg["model"] = "deepseek-v4-flash"
    CONFIG.write_text(json.dumps(cfg, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return prev


def restore_extra_dirs(prev: list) -> None:
    cfg = json.loads(CONFIG.read_text(encoding="utf-8"))
    skills = cfg.setdefault("skills", {})
    skills["extra_dirs"] = list(prev or [])
    CONFIG.write_text(json.dumps(cfg, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def start_dashboard(port: int, workspace: Path, log_path: Path) -> subprocess.Popen:
    bin_candidates = [
        ROOT / "target" / "release-local" / "anycode-dashboard-serve",
        ROOT / "target" / "release" / "anycode-dashboard-serve",
    ]
    bin_path = next((p for p in bin_candidates if p.is_file()), None)
    if bin_path is None:
        print("building anycode-dashboard-serve (release-local)…", flush=True)
        subprocess.run(
            [
                "cargo",
                "build",
                "--profile",
                "release-local",
                "-p",
                "anycode-dashboard",
                "--bin",
                "anycode-dashboard-serve",
            ],
            cwd=ROOT,
            check=True,
        )
        bin_path = ROOT / "target" / "release-local" / "anycode-dashboard-serve"

    db = Path(os.environ.get("TMPDIR", "/tmp")) / f"anycode-skill-bakeoff-{port}.db"
    for suffix in ("", "-wal", "-shm"):
        p = Path(str(db) + suffix) if suffix else db
        if p.exists():
            p.unlink()

    env = os.environ.copy()
    env.update(
        {
            "ANYCODE_DASHBOARD_DB": str(db),
            "ANYCODE_DASHBOARD_RECORD": "0",
            "ANYCODE_DASHBOARD_TEST_AUTH_BYPASS": "1",
            "ANYCODE_IGNORE_APPROVAL": "1",
            "ANYCODE_DASHBOARD_EMBEDDED_DESKTOP": "1",
        }
    )
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_f = open(log_path, "w", encoding="utf-8")
    proc = subprocess.Popen(
        [str(bin_path), "--host", "127.0.0.1", "--port", str(port), "--db", str(db)],
        cwd=str(workspace),
        env=env,
        stdout=log_f,
        stderr=subprocess.STDOUT,
    )
    base = f"http://127.0.0.1:{port}"
    for _ in range(90):
        code, _ = req("GET", f"{base}/api/health", timeout=2)
        if code == 200:
            return proc
        if proc.poll() is not None:
            break
        time.sleep(0.5)
    raise RuntimeError(f"dashboard failed to start; see {log_path}")


def stop_proc(proc: subprocess.Popen | None) -> None:
    if proc is None:
        return
    if proc.poll() is None:
        proc.send_signal(signal.SIGTERM)
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()


def approve_pending(base: str, session_id: str) -> int:
    code, data = req("GET", f"{base}/api/security/approvals/pending?limit=50")
    if code != 200:
        return 0
    n = 0
    for item in data.get("pending") or []:
        if item.get("session_id") != session_id:
            continue
        aid = item.get("approval_id")
        if not aid:
            continue
        req(
            "POST",
            f"{base}/api/security/approvals/{aid}/respond",
            {"decision": "allow_all_session"},
            timeout=15,
        )
        n += 1
    return n


def answer_questions(base: str, session_id: str) -> None:
    code, data = req(
        "GET",
        f"{base}/api/security/questions/pending?session_id={session_id}&limit=10",
    )
    if code != 200:
        return
    for q in data.get("pending") or []:
        qid = q.get("question_id")
        if not qid:
            continue
        options = q.get("options") or []
        label = "ok"
        if options and isinstance(options[0], dict):
            label = options[0].get("label") or options[0].get("id") or "ok"
        elif options and isinstance(options[0], str):
            label = options[0]
        req(
            "POST",
            f"{base}/api/security/questions/{qid}/respond",
            {"answers": [{"question_id": qid, "selected": [label]}]},
            timeout=15,
        )


def wait_done(base: str, session_id: str, timeout_s: float) -> str:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        approve_pending(base, session_id)
        answer_questions(base, session_id)
        code, payload = req("GET", f"{base}/api/sessions/{session_id}")
        if code == 200:
            state = (payload.get("session") or {}).get("status", "")
            if state in {"completed", "failed", "cancelled"}:
                return state
        time.sleep(2)
    req("POST", f"{base}/api/sessions/{session_id}/cancel", timeout=10)
    return "timeout"


def list_out_files(workspace: Path, case_id: str) -> list[str]:
    d = workspace / "out" / case_id
    if not d.is_dir():
        return []
    files = []
    for p in sorted(d.rglob("*")):
        if p.is_file():
            files.append(str(p.relative_to(workspace)))
    return files


def collect_transcript(base: str, session_id: str) -> dict:
    out: dict = {"session_id": session_id}
    for name, path in (
        ("transcript", f"/api/sessions/{session_id}/transcript"),
        ("trace", f"/api/sessions/{session_id}/trace"),
        ("usage", f"/api/sessions/{session_id}/usage"),
        ("session", f"/api/sessions/{session_id}"),
    ):
        code, payload = req("GET", f"{base}{path}")
        out[name] = payload if code == 200 else {"error": payload, "status": code}
    return out


def assistant_text_preview(transcript: dict, limit: int = 1200) -> str:
    blocks = ((transcript.get("transcript") or {}).get("transcript") or {}).get("blocks") or []
    chunks: list[str] = []
    if isinstance(blocks, list):
        for b in blocks:
            t = b.get("block_type") or b.get("type") or ""
            if "assistant" in str(t):
                body = b.get("body") or b.get("text") or ""
                if body:
                    chunks.append(str(body))
            elif t in {"tool_call", "tool_cluster"}:
                chunks.append(str(b.get("title") or t))
    text = "\n".join(chunks).strip()
    if not text:
        trace = transcript.get("trace")
        text = json.dumps(trace, ensure_ascii=False)[:limit] if trace else ""
    return text[:limit]


def score_heuristic(case: dict, files: list[str], status: str, preview: str) -> dict:
    """Lightweight auto scores 1-5; human SCORECARD overrides."""
    expect_dir = f"out/{case['id']}/"
    has_files = any(f.startswith(expect_dir) for f in files)
    skill = case["skill"]
    mentions_skill = skill in preview or skill.replace("bakeoff-", "") in preview.lower()
    scores = {
        "completion": 1,
        "instruction_follow": 1,
        "artifact_quality": 1,
        "skill_utility": 1,
        "notes": "",
    }
    if status == "completed":
        scores["completion"] = 4 if has_files else 2
    elif status == "failed":
        scores["completion"] = 1
    elif status == "timeout":
        scores["completion"] = 2 if has_files else 1

    if has_files:
        scores["instruction_follow"] = 4
        # size signal
        total = 0
        for f in files:
            try:
                total += (Path.cwd() / f).stat().st_size if False else 0
            except OSError:
                pass
        scores["artifact_quality"] = 3
        if any(f.endswith((".html", ".tsx", ".py", ".md", ".xlsx", ".png", ".gif", ".pdf", ".svg")) for f in files):
            scores["artifact_quality"] = 4
    else:
        scores["instruction_follow"] = 2 if status == "completed" else 1
        scores["artifact_quality"] = 1

    scores["skill_utility"] = 3 if has_files else 2
    if mentions_skill:
        scores["skill_utility"] = min(5, scores["skill_utility"] + 1)
    scores["auto_total"] = sum(scores[k] for k in ("completion", "instruction_follow", "artifact_quality", "skill_utility"))
    scores["notes"] = "auto-heuristic only; replace with human review"
    return scores


def run_case(base: str, project_id: str, workspace: Path, case: dict, run_dir: Path) -> dict:
    timeout_s = float(case.get("timeout_s") or 360)
    title = f"bakeoff-{case['id']}"
    code, payload = req(
        "POST",
        f"{base}/api/sessions",
        {"project_id": project_id, "kind": "run", "title": title},
    )
    if code != 200:
        print(f"[{case['id']}] ERROR create_session {code} {payload}", flush=True)
        return {"id": case["id"], "skill": case.get("skill"), "error": f"create_session {code} {payload}"}
    session_id = payload["session"]["id"]
    t0 = time.time()
    print(f"[{case['id']}] start skill={case['skill']} session={session_id}", flush=True)
    code, msg_payload = req(
        "POST",
        f"{base}/api/sessions/{session_id}/message",
        {
            "prompt": case["prompt"],
            "skills": [case["skill"]],
            "agent": case.get("agent") or "general-purpose",
        },
        timeout=60,
    )
    if code not in (200, 202):
        print(f"[{case['id']}] ERROR message {code} {msg_payload}", flush=True)
        return {
            "id": case["id"],
            "skill": case["skill"],
            "session_id": session_id,
            "error": f"message {code} {msg_payload}",
            "elapsed_s": round(time.time() - t0, 1),
        }
    status = wait_done(base, session_id, timeout_s)
    elapsed = round(time.time() - t0, 1)
    transcript = collect_transcript(base, session_id)
    files = list_out_files(workspace, case["id"])
    preview = assistant_text_preview(transcript)
    case_dir = run_dir / case["id"]
    case_dir.mkdir(parents=True, exist_ok=True)
    (case_dir / "transcript.json").write_text(
        json.dumps(transcript, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    (case_dir / "preview.txt").write_text(preview, encoding="utf-8")
    (case_dir / "files.json").write_text(json.dumps(files, indent=2), encoding="utf-8")
    # copy artifacts into run dir for browsing
    src_out = workspace / "out" / case["id"]
    if src_out.is_dir():
        dst_out = case_dir / "artifacts"
        if dst_out.exists():
            shutil.rmtree(dst_out)
        shutil.copytree(src_out, dst_out)
    scores = score_heuristic(case, files, status, preview)
    result = {
        "id": case["id"],
        "skill": case["skill"],
        "baseline": bool(case.get("baseline")),
        "session_id": session_id,
        "status": status,
        "elapsed_s": elapsed,
        "files": files,
        "scores": scores,
        "message_ack": msg_payload if isinstance(msg_payload, dict) else {},
    }
    (case_dir / "result.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print(
        f"[{case['id']}] status={status} files={len(files)} elapsed={elapsed}s auto={scores['auto_total']}/20",
        flush=True,
    )
    return result


def write_scorecard(run_dir: Path, results: list[dict], model: str) -> None:
    lines = [
        "# Skill Bake-off SCORECARD",
        "",
        f"- Run dir: `{run_dir}`",
        f"- Model: `{model}`",
        f"- Generated: {datetime.now(timezone.utc).isoformat()}",
        "",
        "## Legend",
        "",
        "Auto scores are heuristics (1–5 each): completion / instruction_follow / artifact_quality / skill_utility (max 20).",
        "**Human columns are blank — fill before builtin decisions.**",
        "",
        "## Results",
        "",
        "| id | skill | status | files | auto/20 | human_quality | ship_builtin? | notes |",
        "|---|---|---|---:|---:|---|---|---|",
    ]
    for r in results:
        if r.get("error") and not r.get("status"):
            lines.append(
                f"| {r.get('id')} | {r.get('skill','')} | ERROR | 0 |  |  |  | {r.get('error','')[:80]} |"
            )
            continue
        files = len(r.get("files") or [])
        auto = (r.get("scores") or {}).get("auto_total", "")
        lines.append(
            f"| {r['id']} | `{r.get('skill')}` | {r.get('status')} | {files} | {auto} |  |  |  |"
        )
    lines.extend(
        [
            "",
            "## Decision gate",
            "",
            "- [ ] Review artifacts under each `runs/<id>/artifacts/`",
            "- [ ] Fill `human_quality` (1–5) and `ship_builtin?` (yes/no/adapt)",
            "- [ ] Only after approval: copy adapted skills into `skills-starter/`",
            "",
            "## Index",
            "",
        ]
    )
    for r in results:
        rid = r.get("id")
        if rid:
            lines.append(f"- [{rid}](./{rid}/) — skill `{r.get('skill')}`")
    (run_dir / "SCORECARD.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (run_dir / "results.json").write_text(
        json.dumps(results, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=43199)
    ap.add_argument("--only", default="", help="comma-separated case ids")
    ap.add_argument("--skip-baselines", action="store_true")
    ap.add_argument("--reuse-server", action="store_true")
    ap.add_argument("--workspace", type=Path, default=None)
    ap.add_argument(
        "--prompts",
        type=Path,
        default=None,
        help="prompts JSON (default: fixtures/prompts.json)",
    )
    args = ap.parse_args()

    if not CANDIDATES.is_dir():
        print("candidates missing; run stage_skills.py --clean first", file=sys.stderr)
        return 2

    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    run_dir = BAKEOFF / "runs" / stamp
    run_dir.mkdir(parents=True, exist_ok=True)
    workspace = args.workspace or (run_dir / "workspace")
    workspace.mkdir(parents=True, exist_ok=True)
    (workspace / "out").mkdir(exist_ok=True)
    (workspace / "README.md").write_text(
        "Skill bake-off workspace. Artifacts go under out/<case-id>/.\n", encoding="utf-8"
    )
    # Project-local skills root is always scanned by dashboard validation + runtime.
    link_skills_into_workspace(workspace)

    prompts_path = args.prompts or PROMPTS
    prompts = json.loads(prompts_path.read_text(encoding="utf-8"))
    cases = prompts["cases"]
    defaults = prompts.get("defaults") or {}
    for c in cases:
        c.setdefault("timeout_s", defaults.get("timeout_s", 360))
        c.setdefault("agent", defaults.get("agent", "general-purpose"))
    if args.skip_baselines:
        cases = [c for c in cases if not c.get("baseline")]
    if args.only:
        want = {x.strip() for x in args.only.split(",") if x.strip()}
        cases = [c for c in cases if c["id"] in want]

    extra = [str(CANDIDATES), str(BASELINES)]
    # Also include user skills so baselines already installed still resolve if not staged
    user_skills = Path.home() / ".anycode" / "skills"
    if user_skills.is_dir():
        extra.append(str(user_skills))

    prev_extra = None
    patched = False
    proc = None
    base = f"http://127.0.0.1:{args.port}"
    try:
        prev_extra = patch_extra_dirs(extra)
        patched = True
        print(f"patched skills.extra_dirs -> {extra}", flush=True)
        if args.reuse_server:
            code, _ = req("GET", f"{base}/api/health")
            if code != 200:
                print("reuse-server requested but health failed; starting fresh", flush=True)
                proc = start_dashboard(args.port, workspace, run_dir / "dashboard.log")
        else:
            # kill anything on port
            subprocess.run(
                f"lsof -ti :{args.port} | xargs kill -9 2>/dev/null || true",
                shell=True,
            )
            time.sleep(0.5)
            proc = start_dashboard(args.port, workspace, run_dir / "dashboard.log")
        print(f"dashboard {base}", flush=True)

        # create project pointing at workspace
        code, payload = req(
            "POST",
            f"{base}/api/projects",
            {"root_path": str(workspace), "name": f"skill-bakeoff-{stamp}"},
        )
        if code != 200:
            print(f"create project failed: {code} {payload}", file=sys.stderr)
            return 1
        project_id = payload["project"]["id"]

        # sanity: skills catalog if API exists
        code, skills_payload = req("GET", f"{base}/api/skills")
        (run_dir / "skills-catalog.json").write_text(
            json.dumps({"status": code, "payload": skills_payload}, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )

        results: list[dict] = []
        for case in cases:
            try:
                results.append(run_case(base, project_id, workspace, case, run_dir))
            except Exception as exc:  # noqa: BLE001
                print(f"[{case['id']}] EXCEPTION {exc}", flush=True)
                results.append({"id": case["id"], "skill": case.get("skill"), "error": str(exc)})

        write_scorecard(run_dir, results, "deepseek-v4-flash")
        # mirror latest pointer
        latest = BAKEOFF / "runs" / "LATEST"
        if latest.is_symlink() or latest.exists():
            latest.unlink()
        latest.symlink_to(run_dir.name)
        print(f"SCORECARD: {run_dir / 'SCORECARD.md'}", flush=True)
        return 0
    finally:
        if not args.reuse_server:
            stop_proc(proc)
        if patched:
            try:
                restore_extra_dirs(prev_extra)
                print("restored skills.extra_dirs", flush=True)
            except Exception as exc:  # noqa: BLE001
                print(f"WARN restore config failed: {exc}", file=sys.stderr)


if __name__ == "__main__":
    raise SystemExit(main())
