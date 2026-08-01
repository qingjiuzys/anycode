#!/usr/bin/env python3
"""Promote bake-off candidates into skills-starter (excludes claude-api)."""
from __future__ import annotations

import argparse
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CANDIDATES = Path(__file__).resolve().parents[1] / "skills-candidates"
STARTER = ROOT / "skills-starter"
DESKTOP = ROOT / "apps" / "anycode-desktop" / "resources" / "skills-starter"
USER_SKILLS = Path.home() / ".anycode" / "skills"

# bakeoff_id -> final starter id, category, name_zh, description_zh
PROMOTE: list[tuple[str, str, str, str, str]] = [
    (
        "bakeoff-frontend-design",
        "frontend-design",
        "design",
        "前端设计",
        "有辨识度的前端视觉设计：排版、配色与布局，避免模板化 AI 审美。",
    ),
    (
        "bakeoff-webapp-testing",
        "webapp-testing",
        "qa",
        "Web 应用测试",
        "用 Playwright 测试本地 Web 应用：截图、选择器与可复现检查。",
    ),
    (
        "bakeoff-doc-coauthoring",
        "doc-coauthoring",
        "docs",
        "文档共创",
        "结构化文档共创：收集上下文、迭代润色、读者验收。",
    ),
    (
        "bakeoff-internal-comms",
        "internal-comms",
        "writing",
        "内部沟通",
        "内部沟通写作：3P 更新、状态汇报、简报与事故通报等。",
    ),
    (
        "bakeoff-mcp-builder",
        "mcp-builder",
        "platform",
        "MCP 构建",
        "设计高质量 MCP server：工具 schema、错误模型与安全边界。",
    ),
    (
        "bakeoff-canvas-design",
        "canvas-design",
        "visual",
        "画布视觉设计",
        "视觉哲学驱动的海报/静态设计，输出 PNG/PDF/Markdown。",
    ),
    (
        "bakeoff-algorithmic-art",
        "algorithmic-art",
        "creative",
        "算法艺术",
        "用 p5.js 做可复现的生成艺术（种子随机 + 参数探索）。",
    ),
    (
        "bakeoff-theme-factory",
        "theme-factory",
        "design",
        "主题工厂",
        "把预设或自定义主题应用到幻灯片、文档与 HTML 制品。",
    ),
    (
        "bakeoff-web-artifacts-builder",
        "web-artifacts-builder",
        "frontend",
        "Web 制品构建",
        "构建多组件前端制品（React/Tailwind），可打包为单文件 HTML。",
    ),
    (
        "bakeoff-slack-gif-creator",
        "slack-gif-creator",
        "media",
        "Slack GIF",
        "制作符合 Slack 限制的动画 GIF（尺寸/帧率/时长）。",
    ),
    (
        "bakeoff-skill-creator",
        "skill-creator",
        "meta",
        "Skill 创作",
        "创建、评测并迭代改进 Agent Skill。",
    ),
    (
        "bakeoff-vercel-react-best-practices",
        "vercel-react-best-practices",
        "react",
        "React 最佳实践",
        "Vercel Engineering 的 React/Next.js 性能规则与重构指引。",
    ),
    (
        "bakeoff-vercel-web-design-guidelines",
        "web-design-guidelines",
        "design",
        "Web 设计指南",
        "按 Vercel Web Interface Guidelines 审计无障碍与交互质量。",
    ),
    (
        "bakeoff-vercel-composition-patterns",
        "vercel-composition-patterns",
        "react",
        "React 组合模式",
        "用组合模式替代布尔 prop 膨胀，构建可扩展组件 API。",
    ),
    (
        "bakeoff-vercel-writing-guidelines",
        "vercel-writing-guidelines",
        "writing",
        "写作指南",
        "按 Vercel Writing Guidelines 审校产品文案与文档语气。",
    ),
    (
        "bakeoff-design-taste-frontend",
        "design-taste-frontend",
        "design",
        "设计品味前端",
        "Anti-slop 落地页/作品集设计：先读 brief，再做有辨识度的视觉决策。",
    ),
    (
        "bakeoff-find-skills",
        "find-skills",
        "discovery",
        "查找 Skills",
        "从 skills 生态发现并安装合适的 Agent Skill。",
    ),
]

FRONTMATTER_RE = re.compile(r"^---\n(.*?)\n---\n", re.DOTALL)
SKIP_NAMES = {".git", "node_modules", "__pycache__", ".DS_Store", "BAKEOFF_SOURCE.txt"}


def rewrite_frontmatter(
    text: str, skill_id: str, category: str, name_zh: str, desc_zh: str
) -> str:
    m = FRONTMATTER_RE.match(text)
    body = text[m.end() :] if m else text
    raw_fm = m.group(1) if m else ""
    desc = ""
    for line in raw_fm.splitlines():
        if line.startswith("description:"):
            desc = line.split(":", 1)[1].strip().strip("\"'")
            break
    if not desc:
        desc = desc_zh
    # Drop bakeoff banner if present
    body = re.sub(
        r"(?m)^> \*\*Bake-off staging\*\*.*\n(?:\n)?",
        "",
        body,
        count=1,
    )
    fm = "\n".join(
        [
            "---",
            f"name: {skill_id}",
            f"description: {desc}",
            f"description_zh: {desc_zh}",
            f"name_zh: {name_zh}",
            f"category: {category}",
            "version: 1.0.0",
            "mode: instructions",
            "priority: 80",
            "permissions:",
            "  read_dirs: [workspace]",
            "  write_dirs: [workspace]",
            "  network: true",
            "---",
            "",
        ]
    )
    return fm + body.lstrip("\n")


def copy_skill(src: Path, dst: Path) -> None:
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(
        src,
        dst,
        ignore=shutil.ignore_patterns(*SKIP_NAMES, "*.pyc"),
    )


def promote_one(
    bakeoff_id: str,
    final_id: str,
    category: str,
    name_zh: str,
    desc_zh: str,
    *,
    install_user: bool,
) -> None:
    src = CANDIDATES / bakeoff_id
    if not (src / "SKILL.md").is_file():
        raise FileNotFoundError(f"missing staged skill: {src}")
    dst = STARTER / final_id
    copy_skill(src, dst)
    md = dst / "SKILL.md"
    md.write_text(
        rewrite_frontmatter(md.read_text(encoding="utf-8"), final_id, category, name_zh, desc_zh),
        encoding="utf-8",
    )
    (dst / "THIRD_PARTY_SOURCE.txt").write_text(
        f"promoted_from_bakeoff={bakeoff_id}\n"
        f"final_id={final_id}\n"
        "See upstream LICENSE.txt in this skill directory when present.\n",
        encoding="utf-8",
    )
    print(f"starter: {final_id}")

    if DESKTOP.parent.is_dir():
        desk = DESKTOP / final_id
        copy_skill(dst, desk)
        print(f"desktop: {final_id}")

    if install_user:
        USER_SKILLS.mkdir(parents=True, exist_ok=True)
        user_dst = USER_SKILLS / final_id
        copy_skill(dst, user_dst)
        print(f"user: {final_id}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--no-user-install", action="store_true")
    args = ap.parse_args()
    if not CANDIDATES.is_dir():
        raise SystemExit("run stage_skills.py first")
    for row in PROMOTE:
        promote_one(*row, install_user=not args.no_user_install)
    print(f"promoted {len(PROMOTE)} skills (excluded bakeoff-claude-api)")


if __name__ == "__main__":
    main()
