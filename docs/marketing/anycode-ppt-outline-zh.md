# anyCode 功能与优势清单（PPT 素材 · 通用受众 · 中文）

> 本文档可直接整段复制给 AI 生成 PPT。受众：非技术潜在用户。语言：中文。

---

## 产品一句话

anyCode 是跑在你自己电脑上的 AI 助手——终端里对话、改代码、跑任务；也可从微信发消息驱动，本地网页看板管理一切。**自带 API Key（BYOK）**，数据与执行留在本机，不是云端托管的 Agent 网关。

---

## 逐页文案（14 页，可直接作 slide 正文）

### 第 1 页 · 封面

**标题**：anyCode  
**副标题**：编码万物，构建一切  
**补充**：本机 AI 助手 · BYOK · 微信桥 · 本地工作台

---

### 第 2 页 · 它是什么

- 安装在你电脑上的 AI 助手，不是云端托管服务
- 用**自己的 API Key** 连接 GLM、DeepSeek 等模型，费用透明
- 能读改本地文件、跑命令、查网页——真正**动手做事**
- 配置与任务记录留在本机，你掌控数据边界

---

### 第 3 页 · 三大使用方式

| 方式 | 一句话 |
|------|--------|
| **终端** | 在项目目录打开 `anycode`，像聊天一样协作 |
| **微信** | 手机发消息，驱动家里/公司电脑上的 Agent |
| **本地工作台** | 浏览器看板管理项目、会话、定时任务与报告 |

三种入口，**同一套 AI 引擎**，行为一致。

---

### 第 4 页 · 日常能帮你做什么

- **问答与写作**：改 README、写摘要、解释代码
- **项目协作**：读文件、搜索代码、批量修改（敏感操作会问你）
- **多角色**：通用 / 探索 / 规划助手，先摸清再动手
- **技能包**：安装 Skills 扩展场景（如每日简报、微信历史）
- **记忆**：可选记住项目上下文，长对话自动压缩
- **图片**：粘贴或发送图片（需 Vision 模型）

---

### 第 5 页 · 为什么选 anyCode

**定位**
- BYOK 多厂商，不绑单一 AI 产品
- 本机执行，代码不出你的机器（除 LLM API 调用）
- CLI、微信、工作台共用同一引擎

**体验**
- 个人**微信桥**：国内少见，手机下任务、微信内审批、回传文件
- **本地工作台**：不用记命令，网页一目了然
- **自然语言定时**：「每天 8 点」即可，不必学 cron
- **macOS 原生**：语音/OCR 走 Apple 框架，免下载大模型

---

### 第 6 页 · 微信里也能用

**场景**：通勤路上，用手机微信给电脑下任务。

- 扫码一次绑定（iLink），长期可用，无需 Node 网关
- 支持：文字、图片、引用回复、语音（自动转文字）
- 敏感操作可在**微信内审批**
- 任务完成后自动回传 pdf、图片、视频等文件
- 定时提醒可**推送到微信**
- 斜杠命令：如 `/cwd` 切换工作目录

---

### 第 7 页 · 本地工作台一览

打开：`anycode dashboard --open` → `http://127.0.0.1:43180`

| 页面 | 你能做什么 |
|------|------------|
| 总览 | 今天有没有异常，一眼判断 |
| 项目 / 会话 | 浏览对话与任务历史 |
| 自动化 | 自然语言建定时任务、看重试记录 |
| 资产 | 查看助手改过的文件 |
| 报告 | 一键导出 Markdown / HTML |
| 设置 | 模型、安全、中/英、浅/深色主题 |

macOS 桌面 App 打开即带工作台。

---

### 第 8 页 · 定时与自动化

- 用自然语言写 schedule：「每周五 18:30 提醒我整理周报」
- 自动解析为 cron，支持 Asia/Shanghai 等时区
- 运行记录、失败重试、可选失败通知（微信 / webhook）
- 项目**护栏**：如门禁失败时阻断、完成后自动生成报告
- 微信/Telegram/Discord 桥接运行时**内置调度**，不必单独开服务

---

### 第 9 页 · 安全与可控

- **默认会问你再改文件或跑命令**——像手机 App 权限弹窗
- 可配置「始终允许 / 始终询问」规则
- 可选沙箱：限制工作目录
- 网页抓取会拦截内网地址，降低误访问风险
- 工作台默认仅本机访问（127.0.0.1）
- Ctrl+C 可中断正在跑的任务

---

### 第 10 页 · 模型自由选择（BYOK）

- 自带 Key，按量付费给厂商，**不经过 anyCode 中转**
- 30+ 厂商目录：GLM、DeepSeek、Anthropic、Bedrock、Copilot、OpenRouter、Ollama…
- `anycode setup` / `anycode model` 向导配置
- 工作台「探测」按钮验证连通性
- 不同 Agent 可用不同模型（自动路由）
- 国内友好：默认智谱 GLM，DeepSeek 有专门适配

---

### 第 11 页 · macOS 更好用

- 从 Releases 下载 **anyCode.app**（`.dmg`），一键安装
- 内置 CLI + 工作台，无需单独配 PATH
- **Apple Speech**：原生语音识别，无需下载 Whisper
- **Apple Vision OCR**：设备端文字识别
- 原生 TTS、系统通知、钥匙串集成
- *注：仅浏览器访问 localhost 时无法使用这些原生能力*

---

### 第 12 页 · 3 步上手

1. **安装** — macOS 推荐 `.dmg`；Linux/Windows 用 install 脚本
2. **配置** — 运行 `anycode setup`（选模型 + 可选微信）
3. **验证** — `anycode run "请只回复：OK"` 或 `anycode dashboard --open`

**文档**：https://qingjiuzys.github.io/anycode/  
**开源**：https://github.com/qingjiuzys/anycode （MIT）

---

### 第 13 页 · 适合谁

- **开发者**：终端 AI pair programming、改 repo、跑脚本
- **独立创作者**：微信远程触发本机任务、定时简报
- **小团队**：本地工作台看项目状态、报告、资产
- **注重隐私/合规**：BYOK + 本机执行 + 可审计开源
- **国内用户**：GLM/DeepSeek 友好 + **个人微信桥**

---

### 第 14 页 · 收尾

**anyCode** — 编码万物，构建一切

- GitHub：github.com/qingjiuzys/anycode
- 文档：qingjiuzys.github.io/anycode
- 许可：MIT 开源

**Q&A**

---

## 可选附加页 · 与常见方案对比

> 仅用下表，勿写 anyCode 未实现的能力（如企业 SSO 仍为路线图项）。

| 维度 | anyCode | 常见云端 Agent / 纯聊天 |
|------|---------|-------------------------|
| 执行位置 | 本机 | 多为云端 |
| 改本地代码 | 原生能力 | 通常不支持或受限 |
| 微信驱动 | 个人 iLink 桥 | 一般无 |
| 模型 | BYOK 多厂商 | 常绑单一产品 |
| 定时 / 看板 | 内置 | 需另配 |
| 数据 | 配置与任务在本机 | 依赖服务商 |

**与 Claude Code 等终端 Agent 的差异（简述）**

- anyCode 额外提供：**个人微信桥**、**本地 Digital Workbench**、**自然语言 cron**、**多渠道**（Telegram/Discord）
- 同样强调本机执行与 BYOK，但 anyCode 是**开源 MIT**，可内网部署与二次集成（REST API）

---

## 附录 A · 完整功能清单

### 1. 终端 AI 协作
- 全屏对话（默认）或 `anycode repl` 一行一行输入
- `anycode run "任务"` 一条命令跑完
- 读/写/编辑文件、Glob/Grep、Bash/PowerShell
- 图片输入（Vision 模型）

### 2. 多角色助手
- general-purpose / explore / plan / workspace-assistant
- 可自定义 profile（builder、reviewer 等）

### 3. 技能包（Skills）
- `~/.anycode/skills/<id>/SKILL.md`
- `anycode skills install-starter` 安装 starter 包
- 工作台查看与管理

### 4. 记忆系统
- noop / Markdown / hybrid / pipeline + 向量
- setup 向导一步配置；长对话 auto-compact

### 5. 个人微信桥
- iLink 扫码；入站文字/图片/语音；出站文件/图片/视频
- 微信内审批；SendWeChatMessage；QueryWeChatHistory（需配置）

### 6. 其他渠道
- Telegram、Discord；AskUserQuestion 内联选题

### 7. Digital Workbench
- 总览、项目、会话、自动化、资产、报告、审计、Agent/Skills、设置
- REST API + API Token；MCP 图形化配置

### 8. 定时与自动化
- CronCreate 自然语言；cron-runs.jsonl 日志；项目 guardrails

### 9. 工具能力（简化表述）
- 代码、终端、网络（WebFetch 私网防护）、MCP、LSP、嵌套 Agent、计划模式

### 10. macOS 桌面 App
- anyCode.app：STT、OCR、TTS、通知、Keychain、剪贴板

### 11. 配置与 onboarding
- `anycode setup` / `model` / `config`；ANYCODE_LANG=zh/en

---

## 附录 B · 完整优势清单（20 条）

**定位**：BYOK · 本机执行 · 单一 Rust 运行时  
**体验**：终端优先 · 微信桥 · 本地工作台 · 自然语言定时 · macOS 原生媒体  
**模型**：30+ 厂商 · 国内 GLM/DeepSeek · OpenClaw 对标同步  
**安全**：审批 · 沙箱 · MCP 过滤 · 循环上限 · 协作式取消  
**企业**：REST API · 审计 · Eval 门禁 · MIT 开源

---

## 附录 C · 平台支持

| 平台 | 支持程度 |
|------|----------|
| macOS | 最完整：`.dmg`、原生 STT/OCR/TTS、内置 CLI |
| Linux | CLI + 浏览器工作台；install.sh |
| Windows | CLI + 工作台；PowerShell |

微信扫码需有图形界面/浏览器的机器。

---

## 给 AI 做 PPT 的附加指令（复制此段 + 上文逐页文案）

```
请根据附件《anyCode 功能与优势清单》制作 14 页中文 PPT（可选加 1 页对比表），受众为非技术潜在用户。

要求：
- 风格简洁、少术语；每页 3–5 个 bullet
- 多用场景句（如「通勤时微信下任务」「不用学 cron」）
- 重点：本机 BYOK、微信桥、本地工作台、定时自动化、macOS 原生体验
- 避免：Rust/Tokio/ratatui 等实现细节
- 勿夸大：企业 SSO/RBAC 仍为路线图，不要写「已就绪」
- 配色：品牌紫 + accent 橙（与 anyCode 终端 UI 一致）
- 最后一页：GitHub qingjiuzys/anycode、文档站、MIT 开源
- 可选对比页：仅用文档中「与常见方案对比」表格，勿扩展未实现功能
```

---

## 文档来源

- [README.zh.md](../../README.zh.md)
- [docs-site/zh/index.md](../../docs-site/zh/index.md)
- [docs-site/zh/guide/workbench.md](../../docs-site/zh/guide/workbench.md)
- [docs-site/zh/guide/wechat.md](../../docs-site/zh/guide/wechat.md)
- [docs-site/zh/guide/architecture.md](../../docs-site/zh/guide/architecture.md)
