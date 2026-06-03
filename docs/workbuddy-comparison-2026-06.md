# WorkBuddy 对标简报（2026-06）

维护者用：记录 anyCode 相对腾讯 WorkBuddy 的差距与取舍。**IM 通道策略：仅个人微信**（不扩展飞书/钉钉/企微/QQ）。

产品 MVP 边界仍以 [docs-site/guide/roadmap.md](../docs-site/guide/roadmap.md) 为准；可执行 backlog 见 [roadmap.md](roadmap.md)。

## 同步基线

| 项 | 值 |
|----|-----|
| WorkBuddy 公开上线 | 2026-03-09 |
| anyCode 对照 | WorkBuddy 对标 Phase 1–3（Skills / 专家 / 资料库 / 自动化 / 微信 UX / Tauri） |
| OpenClaw 关系 | 两者均对齐 OpenClaw **Skills 形态**；anyCode 仍以 [openclaw-sync-brief](openclaw-sync-brief-2026-05.md) 跟踪上游 |

---

## 七领域差距矩阵（通道 = 个人微信 only）

图例：**Port** = 借鉴实现；**Partial** = 子集；**Skip** = 不做；**Done** = 已有等价。

### 1. 产品形态 / Onboarding

| WorkBuddy | anyCode | 决策 |
|-----------|---------|------|
| 桌面 GUI 下载即用 | Terminal + `anycode dashboard` | **Port** — Tauri sidecar + tag CI（2026-06） |
| 内置模型 + Credits | BYOK | **Skip** 计费绑定；保留 setup 向导 |
| 小程序云 relay | 无 | **Skip**（ADR 003） |

### 2. Skills 生态

| WorkBuddy | anyCode | 决策 |
|-----------|---------|------|
| 20+ 内置 Skills | `skills-starter/`（7 个）+ install 脚本 | **Done**（2026-06） |
| SkillHub / Git / ZIP 导入 | `anycode skills install` + Dashboard import | **Done**（2026-06） |
| skill-vetter | `anycode skills vet` | **Done**（2026-06） |
| 对话内选用 Skills | Dashboard 多选 + `[Use skills: …]` prompt 前缀 | **Done**（2026-06） |
| 导出类 Skills | `report-to-csv` / `md-to-pdf` + run 脚本 | **Done**（2026-06） |

### 3. 专家模式

| WorkBuddy | anyCode | 决策 |
|-----------|---------|------|
| 140+ 虚拟专家 | 8–12 办公 preset（declarative profiles） | **Done**（2026-06） |
| 角色 + 技能 + 资料库一体 | preset + `knowledge.paths` + KnowledgeSearch overlay | **Done** v1（2026-06） |

### 4. 自动化中心

| WorkBuddy | anyCode | 决策 |
|-----------|---------|------|
| GUI 创建定时任务 | Dashboard POST `/api/cron/jobs` + 模板 + NL 解析 | **Done**（2026-06） |
| 执行监控 / 失败原因 | `AutomationsPage` + cron 重试 + `cron-runs.jsonl` | **Done**（2026-06） |
| 自然语言建 cron | Dashboard NL 解析 + `POST /api/cron/parse-schedule` | **Done**（2026-06） |
| Stable cron session | `CronJob.session_id` | **Done**（orchestration 字段已有） |
| 微信结果推送 | `cron_notify` | **Done** |

### 5. 资料库 / RAG

| WorkBuddy | anyCode | 决策 |
|-----------|---------|------|
| 文件夹 + 腾讯文档 | 项目 `knowledge.paths` + SQLite 索引 + Dashboard 检索预览 | **Done** v1（2026-06） |
| 语义向量检索 | 可选 `knowledge-embeddings`（FastEmbed + Sled 混合 BM25） | **Done** v2（2026-06，`--features knowledge-embeddings`） |
| 腾讯文档 / ima OAuth | 无 | **Skip**；可选 MCP Later |

### 6. 多 Agent

| WorkBuddy | anyCode | 决策 |
|-----------|---------|------|
| 并行看板 | `Task*` / `run_in_background` + Dashboard 任务表 | **Done**（2026-06） |
| 多机集群 | `RemoteTrigger` | **Later** |

### 7. 微信遥控 UX（个人微信 only）

| WorkBuddy | anyCode | 决策 |
|-----------|---------|------|
| 手机下发 + 状态追踪 | iLink 桥 | **Done**（2026-06） |
| 任务生命周期推送 | received / running / done | **Done**（2026-06） |
| 交付物回传 | 扩展 `cron_notify` / bridge + CDN `file_item` | **Done** v2（2026-06，需 live iLink） |
| `channel status` | `anycode channel status` | **Done** — doctor 行 + 微信详情 |

### 8. 安全

| WorkBuddy | anyCode | 决策 |
|-----------|---------|------|
| skill-vetter | `skills vet` | **Done**（2026-06） |
| 沙盒叙事 | `SecurityLayer` + audit | **Done** |

---

## anyCode 差异化（不必削弱）

- 开发者 Harness：LSP/MCP、worktree、YAML workflow、Digital Workbench 门禁/eval、会话 replay/budget、Conversations 超预算筛选、eval trajectory CI + guard 负向自检
- 单一 `AgentRuntime` 编排权威（ADR 000）
- 自托管 BYOK
- 原生 Rust 微信桥（无 Node Gateway）

---

## 明确不做

- 飞书 / 钉钉 / 企微 / QQ 通道
- 腾讯混元 Credits / 文档 OAuth 硬绑定
- Gateway / 小程序云 relay v1
- 复制 WorkBuddy 内置 Skills 原文

---

## 相关文档

- [closure-plan-2026-06.md](closure-plan-2026-06.md) — **套件收口规划（Wave 0–4 + Exit Criteria）**
- [openclaw-sync-brief-2026-05.md](openclaw-sync-brief-2026-05.md)
- [weixin-plugin-parity.md](weixin-plugin-parity.md)
- [digital-workbench-next-steps-zh.md](digital-workbench-next-steps-zh.md)
- [docs-site/guide/skills.md](../docs-site/guide/skills.md)
