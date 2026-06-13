# anycode 项目级数字工作台产品方案

> **工程状态 (2026-05)：** V1 MVP + V2 切片 A–D **已实现**。规划入口：[`digital-workbench-STATUS.md`](digital-workbench-STATUS.md) · [`digital-workbench-next-steps-zh.md`](digital-workbench-next-steps-zh.md)

## 1. 产品判断

当前目标是对的：anycode 不能只做一个 CLI 输出面板，而应该成为一个企业可依赖的 AI 自动化项目系统。

关键调整：

- **项目优先**：成果、门禁、知识、资产、自动化策略都属于项目；会话只是项目下的一次执行。
- **自动化优先**：企业使用时不应该靠人盯日志，而是靠策略、门禁、重试、告警、报告自动推进。
- **可信交付优先**：AI 声称完成不算完成，必须有测试、审计、资产、运行验证。
- **产品 UI 优先**：不能用“AI dashboard 风格”的渐变大卡片糊弄，必须像成熟 SaaS：稳定、克制、信息密度高、可配置。

## 2. Web 技术选型

### 推荐栈

第一阶段使用 **React + TypeScript + Vite + TanStack Router + TanStack Query + SQLite-backed local API**。

理由：

- `Vite`：构建简单，适合嵌入 CLI 作为静态资源。
- `React`：生态成熟，适合复杂企业后台。
- `TanStack Router`：多页面路由清晰，支持项目、对话、资产、设置等结构。
- `TanStack Query`：天然适配本地 API/SSE 状态刷新。
- `SQLite`：生产级本地事实库，适合项目级长期状态。
- `SSE`：先做单向实时事件推送，简单稳定；后续需要控制任务再上 WebSocket。

### 不建议第一阶段使用

- **Next.js**：对本地 CLI 内嵌来说过重，SSR 不必要。
- **Electron 首发**：打包、签名、更新复杂，会拖慢核心能力。
- **纯静态 HTML 长期维护**：适合原型，不适合正式产品。
- **只用 Tailwind 堆页面**：容易继续变成“AI 味”页面，需要设计系统约束。

### 后续形态

- V1：`anycode dashboard` 启动 `127.0.0.1` WebUI。
- V2：WebUI 稳定后，用 Tauri 封装本地客户端。
- V3：企业部署版，可接外部 Postgres / OIDC / SSO，但本地默认仍用 SQLite。

## 3. UI 设计方向

当前原型的问题：

- 元素太挤在一个页面。
- 卡片和渐变太多，像 AI 生成的大屏。
- 没有成熟产品的信息架构。
- 设置、登录、项目、对话、资产混在一起，导致认知负担大。

正式 UI 应参考这些成熟产品的结构，而不是照抄视觉：

- LangSmith：Projects、Traces、Threads、Dashboards、Datasets、Evaluators。
- Dify：Apps、Dashboard、Logs、Workflow、Settings。
- Linear：克制的导航、项目/任务分层、高信息密度。
- Datadog / Sentry：可观测性、事件、错误、告警、指标。
- Retool / Airplane：企业内部工具、权限、任务执行记录。

### 设计原则

- **少渐变、少发光、少装饰**：更像 Linear / Sentry，而不是 AI 炫酷大屏。
- **信息密度可调**：舒适 / 紧凑 / 审计模式。
- **模块可配置**：首页图表、风险、资产、成本、agent、门禁可开关。
- **项目先于会话**：导航中“项目”是核心，不是“聊天”。
- **设置独立成页**：登录、安全、端口、数据源、模型、技能、资产读取都放设置。
- **资产有可信度**：README、代码、测试、截图、浏览器验证可信度不同。

## 4. 信息架构

### 一级导航

1. **首页**
   - 项目健康度
   - 自动化成功率
   - 成本 / token / 时间
   - 阻塞风险
   - 交付趋势
   - 最近成果

2. **项目**
   - 项目列表
   - 项目目标
   - 项目下会话
   - 项目资产
   - 项目门禁
   - 项目自动化策略

3. **对话**
   - 必须先选择项目
   - 对话沉淀为 project session
   - 支持选择 agent 团队 / 模式
   - 支持继续某个会话或基于项目上下文新开会话

4. **自动化**
   - Goal / workflow / cron
   - 失败重试策略
   - 模型兜底策略
   - 审批策略
   - 交付报告

5. **资产**
   - 文件
   - 报告
   - 测试结果
   - 截图 / 录屏
   - PR / commit
   - 运行日志
   - 外部系统资产

6. **Agent / Skills**
   - Agent 角色
   - Skills 注册表
   - 项目启用的 skills
   - Skill 运行记录
   - Skill 权限范围

7. **设置**
   - 账户与登录
   - 安全与审计
   - 端口与服务
   - 数据源
   - 模型与配额
   - Skills 管理
   - 资产读取权限
   - 通知与集成

## 5. 登录与账户

### 本地版

默认支持：

- 首次初始化管理员账户。
- 本地密码登录。
- 重置密码。
- 登录会话管理。
- API token 管理。
- 可选关闭登录，仅监听 `127.0.0.1` 时允许 “local trusted mode”。

### 企业版

预留：

- OIDC / SSO。
- GitHub OAuth。
- LDAP / SAML（后置）。
- 组织、成员、角色。
- RBAC：Owner / Admin / Maintainer / Viewer / Auditor。

### 重置密码

必须有：

- 本地管理员通过 CLI 重置：
  - `anycode dashboard user reset-password`
- Web 登录页 “忘记密码”
  - 本地模式：生成 CLI 验证 token。
  - 企业模式：邮件或 SSO。
- 审计记录：
  - `auth_events` 表记录重置时间、来源、操作者。

## 6. 端口与服务治理

企业使用时端口必须可管理。

### 端口策略

- 默认监听：`127.0.0.1`
- 默认端口：自动分配，优先配置项，例如 `43180`
- 支持固定端口。
- 支持端口冲突检测。
- 支持打开浏览器。
- 支持只读模式 / 控制模式。

### 端口管理页面

设置页需要有 “服务与端口”：

- Dashboard server：host、port、状态、启动时间。
- SSE endpoint：连接数、最近事件。
- API token：创建 / 删除 / 过期时间。
- 端口冲突提示。
- 一键复制访问地址。

### 数据表

- `local_services`
  - `service_id`
  - `name`
  - `host`
  - `port`
  - `status`
  - `started_at`
  - `pid`
  - `auth_mode`

## 7. Skills 体系

Skills 不能只是文件夹能力，应该进入项目级工作台。

### Skill Registry

展示：

- skill 名称
- 来源
- 版本
- 描述
- 适用项目类型
- 需要的工具权限
- 最近运行
- 成功率
- 成本

### Project Skill Packs

项目可以启用 skill 包：

- Flutter App Pack
  - 创建项目
  - 修复 analyze/test
  - widget/browser 验收
  - 打包发布

- Rust CLI Pack
  - fmt / clippy / test / release build
  - CI 对齐
  - changelog / release note

- Docs Pack
  - docs-site build
  - 双语文档
  - API 参考生成

### Skill 权限

每个 skill 需要声明：

- 可读目录
- 可写目录
- 可用工具
- 是否允许网络
- 是否允许调用外部 API
- 是否需要审批

### 数据表

- `skills`
- `project_skills`
- `skill_runs`
- `skill_permissions`

## 8. 资产读取

企业场景下，资产不仅是代码文件。

### 资产类型

- 本地文件：代码、配置、文档。
- 测试结果：JUnit、Flutter test、cargo test。
- 构建产物：release binary、apk、web build。
- 截图 / 录屏。
- Git：commit、branch、PR、diff。
- 任务日志：output.log、events.jsonl。
- 外部系统：GitHub、Linear、Slack、Sentry、Datadog。
- 知识库：Markdown、PDF、网页、内部文档。

### 读取策略

- 项目内文件默认可读。
- 项目外文件需要显式授权。
- 外部系统通过 connector 授权。
- 资产读取要有审计。
- 大文件要索引摘要，不直接塞进上下文。

### 资产索引

需要后台索引器：

- 文件元数据。
- 内容摘要。
- hash。
- 最近修改。
- 关联会话 / agent / gate。
- 可信度。

### 数据表

- `artifacts`
- `artifact_versions`
- `artifact_links`
- `asset_sources`
- `asset_permissions`
- `asset_index_jobs`

## 9. 数据库方案

推荐 SQLite 文件：

```text
~/.anycode/projects.db
```

核心表：

- `users`
- `organizations`
- `projects`
- `sessions`
- `agents`
- `skills`
- `project_skills`
- `project_events`
- `artifacts`
- `artifact_versions`
- `gates`
- `automation_policies`
- `local_services`
- `auth_events`
- `asset_sources`
- `asset_permissions`
- `metrics_daily`

### 为什么不是只用 JSONL

JSONL 适合审计事件流，不适合作为企业工作台查询库。

SQLite 的好处：

- 标准生产数据库。
- 支持索引和查询。
- 适合项目级长期状态。
- 易备份、易迁移。
- 本地优先，不需要部署服务。

建议：

- `events.jsonl` 保留为 append-only 审计流。
- `projects.db` 作为查询与 UI 状态库。
- 后续企业服务器版可迁移到 Postgres。

## 10. 自动化能力

企业依赖系统的关键不是“能看”，而是“能自动推进”。

### 自动化策略

- 失败自动重试。
- 重复失败自动总结根因。
- 配额耗尽自动换模型。
- 测试失败自动注入真实错误。
- 高风险操作自动请求审批。
- 任务完成自动生成报告。
- 项目资产自动归档。
- 项目指标自动更新。

### 自动化等级

- Level 0：只读观察。
- Level 1：自动验收和报告。
- Level 2：自动修复和重试。
- Level 3：跨 agent 并行协作。
- Level 4：跨系统自动交付。

## 11. 需要重做的原型方向

当前两个 HTML 原型只能作为探索稿，正式原型应重做为多页面产品：

- 登录页。
- 首页图表。
- 项目列表。
- 项目详情。
- 项目对话。
- 自动化策略。
- Skills 管理。
- 资产库。
- 设置。

UI 风格：

- 不再使用大面积炫光和 AI 感渐变。
- 更像 Linear + Sentry + LangSmith。
- 高信息密度、克制、清晰、企业可用。

## 12. 下一步建议

先不要急着写后端。

建议下一步只做三个东西：

1. **重新设计 WebUI 信息架构和视觉稿**
   - 多页面。
   - 登录。
   - 设置独立。
   - 项目为中心。

2. **定义 SQLite schema**
   - 至少覆盖项目、会话、事件、资产、门禁、skills、端口、登录。

3. **定义 dashboard API**
   - `/api/projects`
   - `/api/projects/:id/sessions`
   - `/api/projects/:id/artifacts`
   - `/api/projects/:id/gates`
   - `/api/projects/:id/events/stream`
   - `/api/settings/services`
   - `/api/skills`

确认后再进入实现。
