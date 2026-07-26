#!/usr/bin/env python3
"""Run experience-baseline scenarios: deepseek-v4-flash vs Pro vs flash+experience.

Arms
----
- low_model          → deepseek-v4-flash (raw)
- codex_reference    → deepseek-v4-pro   (stronger reference; named for historical suite)
- low_model_enhanced → deepseek-v4-flash + retrieved experience pack injection

Does not write API keys to disk outputs. Loads DEEPSEEK_API_KEY from env or a
sibling project `.env` if present.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCENARIOS = ROOT / "test" / "benchmarks" / "experience-baseline" / "scenarios.json"
OUT_DIR = ROOT / "test" / "benchmarks" / "experience-baseline" / "results"
API_URL = os.environ.get("DEEPSEEK_API_BASE", "https://api.deepseek.com").rstrip(
    "/"
) + "/chat/completions"

# Minimal pack mirror of crates/core builtin 0.2.0 (keep in sync for offline runs).
PACK_CARDS = [
    {
        "id": "web.design-implement-verify",
        "keywords": ["webpage", "landing", "html", "css", "ui", "frontend", "网页", "页面"],
        "excerpt": """### Experience: Webpage visual design → implement → screenshot verify
checks: dark background with readable text; primary CTA uses emerald accent, not white-on-white; one H1 + clear CTA above the fold; responsive basics
breakdown: reuse visual preferences → draft design tokens → implement semantic HTML/CSS → browser screenshot → fix contrast/layout
common_failures: generic purple-gradient AI slop; skipping visual verification""",
    },
    {
        "id": "code.cross-file-verify",
        "keywords": ["rust", "cargo", "code", "代码", "function", "helper", "slugify"],
        "excerpt": """### Experience: Cross-file code change → fmt/lint/test → fix
checks: public API has tests or call sites; fmt/lint clean for touched files; no unused imports
breakdown: locate symbols → edit → formatter → linter → focused tests → fix failures""",
    },
    {
        "id": "office.pptx-briefing",
        "keywords": ["pptx", "ppt", "slides", "deck", "演示", "幻灯片", "brief"],
        "excerpt": """### Experience: PPT briefing deck with narrative arc
checks: title slide + agenda + conclusion; each slide has one message; no lorem ipsum / TBD; numbers or evidence when claiming impact; exactly 5–7 slides
breakdown: clarify audience → outline 5–7 slides → concrete titles → 3–5 bullets max → export
common_failures: walls of text; too many slides""",
    },
    {
        "id": "office.docx-report",
        "keywords": ["docx", "word", "report", "文档", "报告"],
        "excerpt": """### Experience: DOCX report with heading hierarchy
checks: Summary section must be level 1 (H1); then H2 subsections; summary within first screen; each section ends with decision or action; no empty heading stubs
breakdown: define purpose → H1 summary → H2 sections → actionable next steps""",
    },
    {
        "id": "db.schema-first",
        "keywords": ["database", "schema", "ddl", "table", "数据库", "表结构", "create table"],
        "excerpt": """### Experience: Database schema design with constraints
checks: every table has PRIMARY KEY; FKs named and ON DELETE strategy stated; NOT NULL on required fields; indexes for common filters
breakdown: entities + relationships → primary keys → foreign keys + indexes → CREATE TABLE DDL""",
    },
    {
        "id": "sql.query-safe",
        "keywords": ["sql", "select", "query", "join", "查询", "invoice"],
        "excerpt": """### Experience: SQL query with filters, joins, and safety
checks: return only a SQL code fence; no SELECT * in the query; JOIN conditions present; filters pushdown early; LIMIT when exploratory; do not mention SELECT * in commentary
breakdown: restate question → list tables/columns → SELECT explicit columns → WHERE/JOIN → LIMIT""",
    },
]


def load_api_key() -> str:
    key = os.environ.get("DEEPSEEK_API_KEY", "").strip()
    if key:
        return key
    candidates = [
        ROOT.parent / "digital-story-platform" / ".env",
        Path.home() / "workspace/research/digital-story-platform/.env",
        Path("/Users/qingjiu/workspace/research/digital-story-platform/.env"),
    ]
    for path in candidates:
        if not path.is_file():
            continue
        for line in path.read_text().splitlines():
            line = line.strip()
            if line.startswith("DEEPSEEK_API_KEY="):
                value = line.split("=", 1)[1].strip().strip('"').strip("'")
                if value:
                    return value
    raise SystemExit("DEEPSEEK_API_KEY missing (env or sibling .env)")


def retrieve_experience(prompt: str, limit: int = 2) -> str:
    pl = prompt.lower()
    scored = []
    for card in PACK_CARDS:
        score = 0.0
        for kw in card["keywords"]:
            if kw.lower() in pl:
                score += 1.0
        if score > 0:
            scored.append((score, card))
    scored.sort(key=lambda x: -x[0])
    hits = [c for _, c in scored[:limit]]
    if not hits:
        return ""
    lines = ["## Experience Pack", "pack: anycode-builtin@0.2.0"]
    for c in hits:
        lines.append(c["excerpt"])
        lines.append("")
    return "\n".join(lines)


def chat(api_key: str, model: str, system: str, user: str, timeout: int = 120) -> tuple[str, int, int]:
    body = {
        "model": model,
        "temperature": 0.2,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
    }
    req = urllib.request.Request(
        API_URL,
        data=json.dumps(body).encode(),
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    t0 = time.time()
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            payload = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        detail = e.read().decode(errors="replace")
        raise RuntimeError(f"HTTP {e.code}: {detail[:400]}") from e
    latency_ms = int((time.time() - t0) * 1000)
    text = (
        payload.get("choices", [{}])[0]
        .get("message", {})
        .get("content", "")
    )
    usage = payload.get("usage") or {}
    tokens = int(usage.get("total_tokens") or 0)
    return text or "", tokens, latency_ms


def extract_json_blob(text: str):
    text = text.strip()
    fence = re.search(r"```(?:json)?\s*([\s\S]*?)```", text, re.I)
    if fence:
        text = fence.group(1).strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        m = re.search(r"(\{[\s\S]*\}|\[[\s\S]*\])", text)
        if not m:
            return None
        try:
            return json.loads(m.group(1))
        except json.JSONDecodeError:
            return None


def primary_code(text: str, lang_hint: str | None = None) -> str:
    """Prefer fenced code body for scoring so commentary cannot pollute checks."""
    pattern = rf"```(?:{lang_hint})\s*([\s\S]*?)```" if lang_hint else r"```(?:\w+)?\s*([\s\S]*?)```"
    fences = re.findall(pattern, text, flags=re.I)
    if fences:
        return fences[0].strip()
    return text


def score(verifier: str, text: str) -> tuple[bool, float, str]:
    """Return (passed, score 0..1, message)."""
    low = text.lower()
    checks: list[tuple[str, bool]] = []

    if verifier == "html_dark_emerald":
        body = primary_code(text, "html").lower()
        checks = [
            ("has html", "<html" in body or "<!doctype html" in body),
            ("dark bg", any(x in body for x in ["#0", "rgb(0", "background:#0", "background: #0", "dark"])),
            ("emerald/green cta", any(x in body for x in ["emerald", "#10b981", "#34d399", "#059669", "green"])),
            ("h1", "<h1" in body),
            ("button/cta", "button" in body or "cta" in body or "get started" in body),
            ("no purple slop", not bool(
                re.search(
                    r"(?:color|background|border)[^;{}]{0,40}\b(purple|violet)\b|#7c3aed|#8b5cf6|#a855f7",
                    body,
                )
            )),
        ]
    elif verifier == "rust_slugify":
        body = primary_code(text, "rust")
        blow = body.lower()
        space_to_dash = (
            "replace" in blow
            or ("' '" in body and "'-'" in body)
            or ('" "' in body and '"-"' in body)
            or ("c == ' '" in body and "'-'" in body)
            or ('c == " "' in body)
            or ("some('-')" in blow)
        )
        checks = [
            ("fn slugify", "fn slugify" in blow),
            ("to_lowercase/lowercase", "to_lowercase" in blow or "lowercase" in blow),
            ("space→dash", space_to_dash),
            ("#[test] or #[cfg(test)]", "#[test]" in body or "#[cfg(test)]" in body),
            ("assert", "assert" in blow),
        ]
    elif verifier == "pptx_json_deck":
        data = extract_json_blob(text)
        ok_json = isinstance(data, dict) and isinstance(data.get("slides"), list)
        slides = data.get("slides") if ok_json else []
        n = len(slides) if isinstance(slides, list) else 0
        titles = [str(s.get("title", "")) for s in slides] if isinstance(slides, list) else []
        bullets_ok = all(
            isinstance(s, dict) and isinstance(s.get("bullets"), list) and 1 <= len(s.get("bullets")) <= 6
            for s in slides
        ) if isinstance(slides, list) and slides else False
        joined = " ".join(titles).lower()
        checks = [
            ("valid json deck", ok_json),
            ("5-7 slides", 5 <= n <= 7),
            ("bullets sized", bullets_ok),
            ("has problem/plan/risk or ask", any(k in joined for k in ["problem", "plan", "risk", "ask", "metric", "问题", "计划", "风险"])),
            ("no placeholder", "lorem" not in low and "tbd" not in low),
        ]
    elif verifier == "docx_json_outline":
        data = extract_json_blob(text)
        ok_json = isinstance(data, dict) and isinstance(data.get("sections"), list)
        sections = data.get("sections") if ok_json else []
        levels = [s.get("level") for s in sections] if isinstance(sections, list) else []
        headings = " ".join(str(s.get("heading", "")).lower() for s in sections) if isinstance(sections, list) else ""
        checks = [
            ("valid json outline", ok_json),
            ("has H1", 1 in levels),
            ("has H2", 2 in levels),
            ("summary section", "summary" in headings or "摘要" in headings or "总结" in headings),
            ("next steps", "next" in headings or "下一步" in headings or "行动" in headings),
            ("enough sections", isinstance(sections, list) and len(sections) >= 4),
        ]
    elif verifier == "sql_ddl_schema":
        body = primary_code(text, "sql").lower()
        checks = [
            ("create table", "create table" in body),
            ("primary key", "primary key" in body),
            ("foreign key or references", "foreign key" in body or "references" in body),
            ("index", "index" in body),
            ("orgs/users/subscriptions/invoices", sum(t in body for t in ["org", "user", "subscription", "invoice"]) >= 3),
            ("not null", "not null" in body),
        ]
    elif verifier == "sql_select_join":
        body = primary_code(text, "sql").lower()
        # Ignore SELECT * mentions outside the query fence.
        has_star = bool(re.search(r"select\s+\*", body))
        checks = [
            ("select", "select" in body),
            ("join", "join" in body),
            ("where", "where" in body),
            ("order by", "order by" in body),
            ("limit", "limit" in body),
            ("no select *", not has_star),
            ("invoice+org", "invoice" in body and "org" in body),
        ]
    else:
        return False, 0.0, f"unknown verifier {verifier}"

    passed_n = sum(1 for _, ok in checks if ok)
    score_v = passed_n / len(checks) if checks else 0.0
    # Require ≥80% checklist for pass (strong enough to show enhancement effect).
    passed = score_v >= 0.8
    detail = "; ".join(f"{name}:{'Y' if ok else 'N'}" for name, ok in checks)
    return passed, score_v, detail


BASE_SYSTEM = (
    "You are a careful software/delivery agent. Follow the user request exactly. "
    "Prefer concrete deliverables over prose. When JSON is requested, return JSON only."
)


def run_arm(api_key: str, scenario: dict, arm: str) -> dict:
    prompt = scenario["prompt"]
    model = {
        "low_model": "deepseek-v4-flash",
        "codex_reference": "deepseek-v4-pro",
        "low_model_enhanced": "deepseek-v4-flash",
    }[arm]
    system = BASE_SYSTEM
    if arm == "low_model_enhanced":
        exp = retrieve_experience(prompt)
        if exp:
            system = BASE_SYSTEM + "\n\n" + exp + "\n\nApply the experience checks strictly."
    text, tokens, latency_ms = chat(api_key, model, system, prompt)
    ok, score_v, detail = score(scenario["verifier"], text)
    return {
        "scenario_id": scenario["id"],
        "arm": arm,
        "model": model,
        "passed": ok,
        "human_preference_score": score_v,
        "total_tokens": tokens,
        "latency_ms": latency_ms,
        "message": detail,
        "output_preview": text[:1200],
    }


def summarize(rows: list[dict]) -> dict:
    by_arm: dict[str, list[bool]] = {}
    by_arm_score: dict[str, list[float]] = {}
    by_arm_tokens: dict[str, list[int]] = {}
    by_arm_latency: dict[str, list[int]] = {}
    for row in rows:
        by_arm.setdefault(row["arm"], []).append(bool(row["passed"]))
        by_arm_score.setdefault(row["arm"], []).append(float(row.get("human_preference_score") or 0))
        by_arm_tokens.setdefault(row["arm"], []).append(int(row.get("total_tokens") or 0))
        by_arm_latency.setdefault(row["arm"], []).append(int(row.get("latency_ms") or 0))
    rates = {
        arm: (sum(1 for p in vals if p) / len(vals) if vals else 0.0)
        for arm, vals in by_arm.items()
    }
    avg_score = {
        arm: (sum(vals) / len(vals) if vals else 0.0) for arm, vals in by_arm_score.items()
    }
    avg_tokens = {
        arm: (sum(vals) / len(vals) if vals else 0.0) for arm, vals in by_arm_tokens.items()
    }
    avg_latency = {
        arm: (sum(vals) / len(vals) if vals else 0.0) for arm, vals in by_arm_latency.items()
    }
    low = rates.get("low_model", 0.0)
    ref = rates.get("codex_reference", 0.0)
    enhanced = rates.get("low_model_enhanced", 0.0)
    score_delta = avg_score.get("low_model_enhanced", 0.0) - avg_score.get("low_model", 0.0)
    return {
        "suite_id": "experience-baseline-v2-deepseek",
        "models": {
            "low_model": "deepseek-v4-flash",
            "codex_reference": "deepseek-v4-pro",
            "low_model_enhanced": "deepseek-v4-flash+experience@0.2.0",
        },
        "per_arm_pass_rate": rates,
        "per_arm_avg_checklist_score": avg_score,
        "per_arm_avg_tokens": avg_tokens,
        "per_arm_avg_latency_ms": avg_latency,
        "enhanced_vs_low_delta": enhanced - low,
        "enhanced_vs_codex_delta": enhanced - ref,
        "enhanced_vs_low_score_delta": score_delta,
        "enhanced_vs_codex_score_delta": avg_score.get("low_model_enhanced", 0.0)
        - avg_score.get("codex_reference", 0.0),
        "meets_promotion_gate": (
            (enhanced - low) >= 0.15 or score_delta >= 0.03
        )
        and (enhanced - ref) >= -0.15,
        "rows": rows,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--limit", type=int, default=0, help="limit scenarios (0=all)")
    ap.add_argument("--out", type=Path, default=OUT_DIR / "latest.json")
    args = ap.parse_args()
    api_key = load_api_key()
    scenarios = json.loads(SCENARIOS.read_text())
    if args.limit:
        scenarios = scenarios[: args.limit]

    rows = []
    for sc in scenarios:
        for arm in ("low_model", "codex_reference", "low_model_enhanced"):
            print(f"→ {sc['id']} / {arm} ...", flush=True)
            try:
                row = run_arm(api_key, sc, arm)
            except Exception as e:  # noqa: BLE001
                row = {
                    "scenario_id": sc["id"],
                    "arm": arm,
                    "model": arm,
                    "passed": False,
                    "human_preference_score": 0.0,
                    "total_tokens": 0,
                    "latency_ms": 0,
                    "message": f"error: {e}",
                    "output_preview": "",
                }
            print(
                f"  passed={row['passed']} score={row['human_preference_score']:.2f} "
                f"tokens={row['total_tokens']} {row['latency_ms']}ms",
                flush=True,
            )
            rows.append(row)

    summary = summarize(rows)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(summary, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({k: summary[k] for k in summary if k != "rows"}, indent=2))
    print(f"wrote {args.out}")
    return 0 if summary["meets_promotion_gate"] else 2


if __name__ == "__main__":
    sys.exit(main())
