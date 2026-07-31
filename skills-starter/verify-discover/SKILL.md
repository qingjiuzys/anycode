---
name: verify-discover
description: Discover and run official verification for any stack — repo clues, web research, execute, evidence. Not language-specific.
description_zh: 开放验证发现：识别栈 → 查官方验法 → 实际执行 → 留证据。不绑定特定语言。
name_zh: 验证发现
category: engineering
version: 1.0.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [files, shell, web]
provides_capabilities: [verify.discover]
priority: 95
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: true
---

# verify-discover

> **中文**：改完可运行产物后，自己找出「怎么验」并跑通，不要把用户当编译器。
> **English**: After changing runnable artifacts, discover how to verify and run checks yourself — never offload compilation to the user.

## When to use

- 新建或大改可运行工程（后端、小程序、Docker、脚本、前端构建）
- 用户贴了编译/运行错误，你修完后
- 准备说「完成 / 已修好 / 重新编译即可」之前
- **不适用**：纯文档/纯问答；已有独立 Gate 的 HTML/Office 交付（等 Gate，不抢完成权）

## Workflow

1. **Identify stack** — 从路径与配置文件判断：README、`docker-compose.yml`、`miniprogram/`、`Cargo.toml`、`package.json` 等。
2. **Check memory** — 项目记忆里若有 `verify_recipe:` 行，先尝试；失败再重新发现。
3. **Discover** — 仓库内找不到时，`WebSearch` + `WebFetch` 查 **官方** 文档（关键词示例：`official build verify`, `cli preview`, `miniprogram-ci`, `docker compose config`）。
4. **Pick smallest proof** — lint < build/compile < smoke test < e2e；能证明修复即可，不必跑全量。
5. **Execute** — `Bash` 跑命令；UI 必须肉眼确认时用 Browser。记录 exit code / 编译器输出。
6. **Fix loop** — 失败则按真实报错修，再执行步骤 5，直到通过或环境阻断。
7. **Done criteria** — 仅在有工具输出证据后声明完成；阻断时写清已尝试命令与缺失依赖。

## Examples (patterns, not hardcoded branches)

| Stack hint | Search terms | Typical verify (discover, don't assume) |
|------------|--------------|----------------------------------------|
| 微信小程序 | `微信开发者工具 cli preview`, `miniprogram-ci` | DevTools CLI / CI 包编译小程序目录 |
| Docker | `docker compose validate` | `docker compose config` / `up -d` + health |
| Node | `npm test`, `package.json scripts` | `npm run build` / `npm test` |
| Rust | `cargo test`, `cargo check` | `cargo test -p crate` / `cargo check` |
| Python | `pytest`, `uvicorn` | `pytest` / import smoke / `docker compose up` |

## Quality contract

- 禁止空口「重新编译即可」作为唯一交付。
- 证据必须来自工具输出，不可编造 exit code。
- 环境缺失（未装 IDE、无 GPU）→ 报告 **blocker**，不假装 PASS。
- Skill 只教方法；Office/HTML 等 Gate 家族仍由独立 validator 决定完成。

## Failure recovery

- 搜不到官方 CLI → 静态检查（grep 禁语法、配置文件 schema）+ 标注「未机械验证」。
- 命令需交互/ GUI -only → 列出最短官方 headless 替代路径；若无则 blocker。
- 验证通过 → 建议写入记忆：`verify_recipe: <stack> → <command> @ <cwd>`。
