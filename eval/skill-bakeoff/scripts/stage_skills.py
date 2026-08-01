#!/usr/bin/env python3
"""Stage bake-off skills with prefixed ids and thin anyCode frontmatter."""
from __future__ import annotations

import argparse
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parents[1] / "skills-candidates"
STARTER = ROOT / "skills-starter"

ANTHROPICS = Path("/tmp/skill-bakeoff-src/anthropics-skills/skills")
VERCEL = Path("/tmp/skill-bakeoff-src/vercel-agent-skills/skills")
AGENTS = Path.home() / ".agents" / "skills"

# (bakeoff_id, source_dir, category, name_zh, description_zh_suffix)
CANDIDATES: list[tuple[str, Path, str, str, str]] = [
    (
        "bakeoff-frontend-design",
        ANTHROPICS / "frontend-design",
        "design",
        "前端设计（Anthropic）",
        "Anthropic 官方前端视觉设计 skill（评测候选）。",
    ),
    (
        "bakeoff-webapp-testing",
        ANTHROPICS / "webapp-testing",
        "qa",
        "Web 应用测试（Anthropic）",
        "Playwright 本地 Web 测试（评测候选）。",
    ),
    (
        "bakeoff-doc-coauthoring",
        ANTHROPICS / "doc-coauthoring",
        "docs",
        "文档共创（Anthropic）",
        "结构化文档共创工作流（评测候选）。",
    ),
    (
        "bakeoff-internal-comms",
        ANTHROPICS / "internal-comms",
        "writing",
        "内部沟通写作（Anthropic）",
        "状态汇报/简报等内部沟通（评测候选）。",
    ),
    (
        "bakeoff-mcp-builder",
        ANTHROPICS / "mcp-builder",
        "platform",
        "MCP 构建（Anthropic）",
        "高质量 MCP server 设计指南（评测候选）。",
    ),
    (
        "bakeoff-canvas-design",
        ANTHROPICS / "canvas-design",
        "visual",
        "画布视觉设计（Anthropic）",
        "海报/静态视觉设计哲学与落盘（评测候选）。",
    ),
    (
        "bakeoff-algorithmic-art",
        ANTHROPICS / "algorithmic-art",
        "creative",
        "算法艺术（Anthropic）",
        "p5.js 生成艺术（评测候选）。",
    ),
    (
        "bakeoff-theme-factory",
        ANTHROPICS / "theme-factory",
        "design",
        "主题工厂（Anthropic）",
        "预设主题应用到制品（评测候选）。",
    ),
    (
        "bakeoff-web-artifacts-builder",
        ANTHROPICS / "web-artifacts-builder",
        "frontend",
        "Web 制品构建（Anthropic）",
        "React+Tailwind 复杂 HTML artifact（评测候选）。",
    ),
    (
        "bakeoff-slack-gif-creator",
        ANTHROPICS / "slack-gif-creator",
        "media",
        "Slack GIF（Anthropic）",
        "Slack 优化动画 GIF（评测候选）。",
    ),
    (
        "bakeoff-skill-creator",
        ANTHROPICS / "skill-creator",
        "meta",
        "Skill 创作（Anthropic）",
        "创建/迭代/评测 Agent Skill（评测候选）。",
    ),
    (
        "bakeoff-claude-api",
        ANTHROPICS / "claude-api",
        "api",
        "Claude API（Anthropic）",
        "Claude API 多语言用法（评测候选）。",
    ),
    (
        "bakeoff-vercel-react-best-practices",
        VERCEL / "react-best-practices",
        "react",
        "React 最佳实践（Vercel）",
        "Vercel Engineering React/Next 性能规则（评测候选）。",
    ),
    (
        "bakeoff-vercel-web-design-guidelines",
        VERCEL / "web-design-guidelines",
        "design",
        "Web 设计指南（Vercel）",
        "Vercel Web Interface Guidelines（评测候选）。",
    ),
    (
        "bakeoff-vercel-composition-patterns",
        VERCEL / "composition-patterns",
        "react",
        "组合模式（Vercel）",
        "React composition patterns（评测候选）。",
    ),
    (
        "bakeoff-vercel-writing-guidelines",
        VERCEL / "writing-guidelines",
        "writing",
        "写作指南（Vercel）",
        "Vercel writing guidelines（评测候选）。",
    ),
    (
        "bakeoff-design-taste-frontend",
        AGENTS / "design-taste-frontend",
        "design",
        "设计品味前端",
        "Anti-slop 落地页/作品集设计（评测候选，本地 agents）。",
    ),
    (
        "bakeoff-find-skills",
        AGENTS / "find-skills",
        "discovery",
        "查找 Skills",
        "发现/安装 agent skills（评测候选，本地 agents）。",
    ),
]

BASELINES = [
    "anycode-xlsx",
    "deep-research",
    "verify-discover",
    "mindmap",
]

FRONTMATTER_RE = re.compile(r"^---\n(.*?)\n---\n", re.DOTALL)


def rewrite_skill_md(text: str, skill_id: str, category: str, name_zh: str, desc_zh: str) -> str:
    m = FRONTMATTER_RE.match(text)
    body = text[m.end() :] if m else text
    raw_fm = m.group(1) if m else f"name: {skill_id}\ndescription: staged bakeoff skill"

    # Parse simple key: value lines; keep description if present.
    desc = ""
    for line in raw_fm.splitlines():
        if line.startswith("description:"):
            desc = line.split(":", 1)[1].strip().strip("\"'")
            break
    if not desc:
        desc = f"Bake-off candidate skill {skill_id}"

    fm = "\n".join(
        [
            "---",
            f"name: {skill_id}",
            f"description: {desc}",
            f"description_zh: {desc_zh}",
            f"name_zh: {name_zh}",
            "category: " + category,
            "version: 0.0.0-bakeoff",
            "mode: instructions",
            "priority: 50",
            "permissions:",
            "  read_dirs: [workspace]",
            "  write_dirs: [workspace]",
            "  network: true",
            "---",
            "",
            f"> **Bake-off staging** — id `{skill_id}`. Not an anyCode built-in.",
            "",
        ]
    )
    return fm + body.lstrip("\n")


def copy_tree(src: Path, dst: Path) -> None:
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(
        src,
        dst,
        ignore=shutil.ignore_patterns(
            ".git",
            "node_modules",
            "__pycache__",
            "*.pyc",
            ".DS_Store",
        ),
    )


def stage_one(skill_id: str, src: Path, category: str, name_zh: str, desc_zh: str) -> None:
    if not src.is_dir() or not (src / "SKILL.md").is_file():
        raise FileNotFoundError(f"missing skill source: {src}")
    dst = OUT / skill_id
    copy_tree(src, dst)
    md_path = dst / "SKILL.md"
    md_path.write_text(
        rewrite_skill_md(md_path.read_text(encoding="utf-8"), skill_id, category, name_zh, desc_zh),
        encoding="utf-8",
    )
    # SOURCE provenance
    (dst / "BAKEOFF_SOURCE.txt").write_text(f"source={src}\nid={skill_id}\n", encoding="utf-8")
    print(f"staged {skill_id} <- {src}")


def stage_baselines() -> None:
    base_out = OUT / "_baselines"
    base_out.mkdir(parents=True, exist_ok=True)
    for sid in BASELINES:
        src = STARTER / sid
        if not src.is_dir():
            print(f"WARN baseline missing in starter: {sid}")
            continue
        dst = base_out / sid
        copy_tree(src, dst)
        print(f"staged baseline {sid}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--clean", action="store_true")
    args = ap.parse_args()
    if args.clean and OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True, exist_ok=True)
    for row in CANDIDATES:
        stage_one(*row)
    stage_baselines()
    print(f"done -> {OUT}")


if __name__ == "__main__":
    main()
