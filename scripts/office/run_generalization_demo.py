#!/usr/bin/env python3
"""Batch demo: multi-brand, scenario, charts — writes report + artifacts for visual review."""
from __future__ import annotations

import json
import shutil
import subprocess
import sys
import zipfile
from datetime import datetime
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "brand-kits" / "lib"))
sys.path.insert(0, str(REPO / "scripts" / "office"))

from brand_kit import infer_brand_kit, infer_scenario, list_brand_kits, load_scenario  # noqa: E402

OUT = REPO / "test" / "benchmarks" / "agent-quality" / "results" / f"office-generalization-{datetime.now().strftime('%Y%m%d-%H%M%S')}"
# Source slides for the brand-fill demo: the starter FDE editorial templates
# (stable, in-repo) instead of a pinned historical results directory.
SLIDES_SRC = REPO / "scripts" / "office" / "slide-templates"
PY = sys.executable


def pptx_stats(path: Path) -> dict:
    slides = sp = pics = text = raster = 0
    with zipfile.ZipFile(path) as zf:
        for name in zf.namelist():
            if name.startswith("ppt/slides/slide") and name.endswith(".xml"):
                slides += 1
                xml = zf.read(name).decode("utf-8", errors="replace")
                sp += xml.count("<p:sp")
                pics += xml.count("<p:pic")
                st = sum(
                    len(p[: p.find("</a:t>")].lstrip(">").strip())
                    for p in xml.split("<a:t")[1:]
                    if "</a:t>" in p
                )
                text += st
                if xml.count("<p:pic") >= 1 and xml.count("<p:sp") <= 2 and st < 20:
                    raster += 1
    has_chart = False
    with zipfile.ZipFile(path) as zf:
        for name in zf.namelist():
            if "chart" in name.lower():
                has_chart = True
                break
            if name.endswith(".xml"):
                if "c:chart" in zf.read(name).decode("utf-8", errors="replace"):
                    has_chart = True
                    break
    editable = text >= 120 and not (
        pics >= slides and slides >= 2 and raster >= max(slides - 1, 1)
    )
    return {
        "slides": slides,
        "native_shapes": sp,
        "pics": pics,
        "text_chars": text,
        "editable": editable,
        "has_chart": has_chart,
        "bytes": path.stat().st_size,
    }


def docx_stats(path: Path) -> dict:
    with zipfile.ZipFile(path) as zf:
        names = set(zf.namelist())
        blob = ""
        for n in ("word/document.xml", "word/header1.xml"):
            if n in names:
                blob += zf.read(n).decode("utf-8", errors="replace")
    lower = blob.lower()
    return {
        "has_header": any(n.startswith("word/header") for n in names),
        "has_footer": any(n.startswith("word/footer") for n in names),
        "has_classification": any(k in lower for k in ("密级", "内部", "classification", "confidential")),
        "bytes": path.stat().st_size,
    }


def xlsx_stats(path: Path) -> dict:
    sheets = has_chart = 0
    with zipfile.ZipFile(path) as zf:
        for n in zf.namelist():
            if n.startswith("xl/worksheets/sheet") and n.endswith(".xml"):
                sheets += 1
            if "charts/chart" in n:
                has_chart = True
    return {"sheets": sheets, "has_chart": has_chart, "bytes": path.stat().st_size}


def run(cmd: list[str], cwd: Path | None = None) -> tuple[int, str]:
    p = subprocess.run(cmd, cwd=cwd or REPO, capture_output=True, text=True)
    out = (p.stdout or "") + (p.stderr or "")
    return p.returncode, out.strip()


def make_chart_slide_html(dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    tpl = REPO / "scripts/office/slide-templates"
    for f in sorted(tpl.glob("*.html")):
        shutil.copy(f, dest / f.name)
    metrics = dest / "metrics.html"
    html = metrics.read_text(encoding="utf-8")
    chart_block = """
<div class="chart-host" style="grid-column:1/-1;height:420px;margin-top:20px"
  data-chart='{"type":"bar","title":"Q2 Revenue by Region","categories":["APAC","EMEA","AMER"],"series":[{"name":"Revenue","data":[120,95,140]}]}'>
</div>
"""
    if "data-chart" not in html:
        html = html.replace("</body>", chart_block + "\n</body>")
        metrics.write_text(html, encoding="utf-8")


def make_gov_doc_md(dest: Path) -> None:
    dest.write_text(
        """# 政务汇报材料

## Summary
本周平台运维总体平稳，关键指标均在阈值内。

## Background
用户规模较上周增长 4.2%，核心接口 P99 延迟 182ms。

## Analysis
- 支付链路成功率 99.94%（来源：监控面板 2026-07-22）
- 发布窗口内零 P0 事故

## Recommendations
Decision: 维持现网容量规划，暂不扩容。
Action: 王明 于 2026-07-28 前完成支付链路压测报告。

密级：内部
""",
        encoding="utf-8",
    )


def make_finance_workbook_json(dest: Path) -> None:
    spec = {
        "sheets": [
            {
                "name": "Summary",
                "rows": [["Metric", "Value"], ["Revenue", "354000"], ["COGS", "121000"]],
            },
            {
                "name": "Detail",
                "rows": [
                    ["Region", "Product", "Units", "Revenue"],
                    ["APAC", "Pro", "120", "48000"],
                    ["EMEA", "Pro", "95", "38000"],
                    ["AMER", "Ent", "80", "64000"],
                ],
            },
            {
                "name": "Pricing",
                "rows": [
                    ["Tier", "Unit Price", "Min Qty", "Status"],
                    ["Standard", "49", "1", "Active"],
                    ["Professional", "99", "10", "Active"],
                ],
            },
        ],
        "charts": [
            {
                "title": "Revenue by Region",
                "categories": ["APAC", "EMEA", "AMER"],
                "series": [{"name": "Revenue", "data": [48000, 38000, 64000]}],
            }
        ],
    }
    dest.write_text(json.dumps(spec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    rows: list[dict] = []

    # --- inference matrix ---
    prompts = [
        ("政府公文汇报 docx", "gov-formal", "gov-briefing"),
        ("高中物理课纲教案 pptx", "edu-clean", "education-lesson-plan"),
        ("年度述职 OKR review docx", "fde-editorial", "performance-review"),
        ("医美诊所方案 pitch", "fde-editorial", "med-aesthetic-proposal"),
        ("enterprise product launch deck", "fde-editorial", "product-launch"),
        ("Q2 finance quarterly review xlsx", "fde-editorial", "finance-quarterly-review"),
    ]
    infer_path = OUT / "inference.json"
    inferred = []
    for prompt, exp_brand, exp_scenario in prompts:
        b = infer_brand_kit(prompt)
        s = infer_scenario(prompt) or ""
        ok = b == exp_brand and s == exp_scenario
        inferred.append({"prompt": prompt, "brand": b, "scenario": s, "ok": ok})
        rows.append({"case": f"infer:{prompt[:20]}", "ok": ok, "detail": f"brand={b} scenario={s}"})
    infer_path.write_text(json.dumps(inferred, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    # --- PPTX x3 brands ---
    if SLIDES_SRC.is_dir():
        for brand in list_brand_kits():
            out_pptx = OUT / "pptx" / brand / "deck.pptx"
            out_pptx.parent.mkdir(parents=True, exist_ok=True)
            ws = OUT / "workspaces" / brand / "slides"
            shutil.copytree(SLIDES_SRC, ws, dirs_exist_ok=True)
            rc, log = run([PY, str(REPO / "scripts/office/fill_potx.py"), str(ws), str(out_pptx), brand])
            stats = pptx_stats(out_pptx) if out_pptx.is_file() else {}
            ok = rc == 0 and stats.get("editable") and stats.get("slides", 0) >= 2
            rows.append({"case": f"pptx:{brand}", "ok": ok, "rc": rc, "stats": stats, "log": log[-200:]})

    # --- chart slide demo ---
    chart_ws = OUT / "workspaces" / "chart-demo" / "slides"
    make_chart_slide_html(chart_ws)
    chart_pptx = OUT / "pptx" / "lingqi-chart" / "deck-with-chart.pptx"
    chart_pptx.parent.mkdir(parents=True, exist_ok=True)
    rc, log = run([PY, str(REPO / "scripts/office/fill_potx.py"), str(chart_ws), str(chart_pptx), "lingqi"])
    stats = pptx_stats(chart_pptx) if chart_pptx.is_file() else {}
    ok = rc == 0 and stats.get("has_chart")
    rows.append({"case": "pptx:lingqi-native-chart", "ok": ok, "rc": rc, "stats": stats})

    # --- DOCX gov + lingqi ---
    for brand, name, maker in [
        ("gov-formal", "gov-report", make_gov_doc_md),
        ("lingqi", "ops-report", lambda p: p.write_text("# Summary\n\n## Metrics\n\nAction: Lin Wei by 2026-07-25.\n", encoding="utf-8")),
    ]:
        md = OUT / "workspaces" / name / "report.md"
        md.parent.mkdir(parents=True, exist_ok=True)
        maker(md)
        docx = OUT / "docx" / brand / f"{name}.docx"
        docx.parent.mkdir(parents=True, exist_ok=True)
        rc, log = run(
            [PY, str(REPO / "scripts/office/build_docx_from_md.py"), str(md), str(docx), brand]
        )
        stats = docx_stats(docx) if docx.is_file() else {}
        ok = rc == 0 and (stats.get("has_header") or stats.get("has_footer"))
        if brand == "gov-formal":
            ok = ok and stats.get("has_classification")
        rows.append({"case": f"docx:{brand}:{name}", "ok": ok, "rc": rc, "stats": stats})

    # --- XLSX finance ---
    wb_json = OUT / "workspaces" / "finance" / "workbook.json"
    wb_json.parent.mkdir(parents=True, exist_ok=True)
    make_finance_workbook_json(wb_json)
    xlsx = OUT / "xlsx" / "lingqi" / "finance-q2.xlsx"
    xlsx.parent.mkdir(parents=True, exist_ok=True)
    rc, log = run(
        [PY, str(REPO / "scripts/office/build_xlsx_from_source.py"), str(wb_json), str(xlsx), "lingqi"]
    )
    stats = xlsx_stats(xlsx) if xlsx.is_file() else {}
    ok = rc == 0 and stats.get("sheets", 0) >= 3
    rows.append({"case": "xlsx:finance+chart", "ok": ok, "rc": rc, "stats": stats})

    # --- scenarios load ---
    for sid in [
        "performance-review",
        "education-lesson-plan",
        "gov-briefing",
        "finance-quarterly-review",
        "med-aesthetic-proposal",
        "product-launch",
        "work-report",
    ]:
        try:
            m = load_scenario(sid)
            rows.append({"case": f"scenario:{sid}", "ok": bool(m.get("id")), "title": m.get("title")})
        except FileNotFoundError as e:
            rows.append({"case": f"scenario:{sid}", "ok": False, "error": str(e)})

    # --- Rust spot checks ---
    rc, log = run(["cargo", "test", "-p", "anycode-agent", "task_compiler", "--", "--quiet"])
    rows.append({"case": "rust:task_compiler", "ok": rc == 0, "rc": rc})
    rc, log = run(["cargo", "test", "-p", "anycode-core", "experience_pack", "--", "--quiet"])
    rows.append({"case": "rust:experience_pack", "ok": rc == 0, "rc": rc})

    passed = sum(1 for r in rows if r.get("ok"))
    summary = {"total": len(rows), "passed": passed, "failed": len(rows) - passed, "out_dir": str(OUT)}
    (OUT / "summary.json").write_text(json.dumps({"summary": summary, "rows": rows}, indent=2) + "\n", encoding="utf-8")

    html_rows = ""
    for r in rows:
        cls = "pass" if r.get("ok") else "fail"
        html_rows += f"<tr class='{cls}'><td>{r.get('case')}</td><td>{'PASS' if r.get('ok') else 'FAIL'}</td><td><pre>{json.dumps({k:v for k,v in r.items() if k not in ('case','ok')}, ensure_ascii=False, indent=2)}</pre></td></tr>"

    artifacts = []
    for p in sorted(OUT.rglob("*")):
        if p.suffix.lower() in (".pptx", ".docx", ".xlsx", ".png") and p.is_file():
            artifacts.append(f"<li><a href='file://{p}'>{p.relative_to(OUT)}</a> ({p.stat().st_size//1024}KB)</li>")

    index = f"""<!DOCTYPE html><html><head><meta charset="utf-8"/><title>Office Generalization Demo</title>
<style>body{{font-family:system-ui;max-width:1100px;margin:2rem auto;padding:0 1rem}}
.pass{{background:#e8f5e9}} .fail{{background:#ffebee}} table{{border-collapse:collapse;width:100%}}
td,th{{border:1px solid #ddd;padding:8px;vertical-align:top}} pre{{margin:0;font-size:12px;white-space:pre-wrap}}</style></head>
<body><h1>Office 通用化 Demo</h1><p>{summary['passed']}/{summary['total']} passed · {OUT}</p>
<h2>Artifacts</h2><ul>{''.join(artifacts)}</ul>
<h2>Cases</h2><table><tr><th>Case</th><th>Result</th><th>Detail</th></tr>{html_rows}</table></body></html>"""
    (OUT / "index.html").write_text(index, encoding="utf-8")

    print(json.dumps(summary, indent=2))
    print(f"index: {OUT / 'index.html'}")
    return 0 if summary["failed"] == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
