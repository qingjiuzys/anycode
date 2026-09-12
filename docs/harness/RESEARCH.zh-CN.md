# AnyCode × Pi Harness × 818cloud：源码调研与重构设计

## 0. 交付定位与核验边界

这是一份基于真实源码读取的**第一阶段重构基础补丁**，不是“整个 AnyCode 已完成替换并通过生产验收”的声明。提供独立 Rust 内核、扩展、真实 AnyCode 适配接缝、818cloud 身份适配、图 UI、测试、技能示例和后续迁移清单。旧桌面聊天／调度入口默认不切换。没有向仓库推送、创建线上资源或修改账户数据。

本次调研针对关键执行链路，不是逐文件审计全部仓库。已经读取工作区、架构约定、AgentRuntime、共享工具内核、图引擎、子代理、共享服务、工具注册与调用、消息／模型类型、bootstrap、Dashboard 依赖及 818cloud SSO 服务端实现。部分大文件按行段读取；技能目录的治理／安装子模块仅做结构识别，不能把这些算作完整安全审计。[S01–S14]

源码基线：AnyCode `0411ea3a94fe6aa2d37326f9342cfc5d6a13f4aa`；818cloud `e8c9b49eafd12a69cf2007e59cd25b3d9b52683e`。Pi 上游当前地址跳转到 `earendil-works/pi`，另外锁定了 `7b4cfd6eb0fd490e3b54370ec9e9227b29717cb4` 的 agent-loop.ts 做源码核对。网页文档读取自当前 main，不宣称所有网页都严格对应同一提交。[S15–S17]

用户提到的 `grapl` 存在歧义：同名 Grapl 是检测响应图平台，不等于 LangGraph。[S19] 本补丁暂按“Graph／LangGraph 式工作流编排及图形界面”落实。**不宣称兼容 Grapl API、LangGraph Python 程序、任意循环状态图或 checkpoint 格式**。确认确切目标库后，应增加独立协议适配器，而不是污染运行内核。

## 1. 结论：方向对，但不是复制一个 Pi CLI

建议保留 Rust、Tauri Desktop、内嵌 Dashboard、模型供应商、已有工具与审批系统，按 Pi 的职责边界重新组织运行时。Pi 适合借鉴的，是把消息上下文、工具循环、宿主扩展和事件生命周期分开，不是它的终端 UI 外壳。[S02,S15,S16]

为什么不整体替换成 TypeScript Pi：现有 AnyCode 的原生能力、工具审批、技能治理、持久化、模型兼容、记忆层和平台联动已经形成代码资产。换语言不是解决边界问题的必要条件；反而会增加 Rust↔Node IPC、双依赖树、安装分发和权限一致性的工作。这是基于当前仓库结构做出的工程判断，而非对两种语言性能的比较。[S01,S02,S06,S07,S13]

也不应该机械复刻一个历史版本的 Pi。当前 agent-core 文档已经允许并行工具执行，并描述了按源顺序持久化结果及执行前后的 hook；它与早期简化的“永远串行工具”印象不同。当前扩展文档也有项目信任与资源生命周期说明，不能一概说 Pi 完全没有权限控制。不过，这不等于已经提供 818cloud 的企业租户、设备登记和计费授权模型。[S15,S16]

本补丁的原子工具调用仍采用保守串行策略，DAG／子代理并行放在外层；这是本项目明确的首阶段策略，不是对当前 Pi 默认行为的照抄。新 Rust 实现独立编写，未打包或复制 Pi 源码，不存在一个隐藏的 Pi subprocess。

## 2. 现有资产应保留什么

`initialize_runtime` 已经装配模型栈、安全配置、工具、记忆、Agent profiles、技能 allowlist 和媒体 registry。它应继续作为 composition root，不再新增一个暗中绕过它的启动器。文件 Agent 定义、project_enabled、global_allowlist 与 agent_allowlists 的合并逻辑应完整保留。[S13]

`AgentRuntime` 是既有多轮执行权威。`execute_task` 与 `execute_turn_from_messages` 已经共用工具分发核，但仍各自承担任务/会话的循环生命周期。重构目标是两者最终退化为薄的入口适配，转换输入、上下文和输出，再进入同一个 Kernel；不是让新旧循环在同一任务里嵌套调用。[S02,S03]

`Tool`、`LLMClient`、`Message`、`ToolCall`、`StreamEvent` 这些接口允许渐进迁移。工具调用链里已有 security → gating → approval → execute → audit，因此新 Host 不能直接持有 `Tool` 后调用 execute。补丁里的 bridge 调用既有 `execute_tool_call`，并在前面增加必须实现的 `HarnessBoundary`。[S07,S08]

消息不是只有字符串。既有消息 metadata 包含 tool calls、reasoning、视觉输入及响应链信息。基础适配优先使用原生 `Message` 序列化，不把它们压成统一 text 再丢掉协议细节。流式传输遇到 Failed 或没有 Done，必须丢弃不完整 Hop，不能执行半截工具调用。[S08,S09]

Dashboard 已依赖 React 19 和 reactflow 11，故新增图组件复用 reactflow，不引入第二套前端框架或庞大的画布引擎。[S14]

## 3. 源码中的具体问题与风险分级

### 3.1 已确认：Partial 被当作图步骤成功

GraphEngine 的成功判断同时接受 `TaskResult::Success` 与 `TaskResult::Partial`，之后将步骤 mark_passed。Partial 明确表达尚有剩余工作，后续依赖被放行会让多阶段任务产生假完成。这是直接读取代码可确认的行为，不是推断。[S04]

处理：新图有独立 Partial 状态并阻止后继执行。可选 legacy hardening 将旧图的成功判断收紧为仅 Success。不能通过修改展示文案掩盖后端仍然通过的问题。

### 3.2 已确认：required_gates 被自动写成 true

旧图在得到成功／部分成功结果后，对声明的 required_gates 循环插入 true。没有在这一代码路径调用独立 verifier。因此“步骤说完成”与“门禁实际通过”被混为一谈。[S04]

处理：新图独立 Gate 节点，只有受信任 NodeExecutor.verify 返回 passed 和有效 artifact digest 才能通过。模型文本不是验证证据。旧图的可选加固采取 fail-closed：带 required_gates 的旧流程在执行副作用前报错并要求迁移，而不是伪造一个通用验证器。注意：这会改变这些旧流程的行为，必须显式选择。

### 3.3 已确认：图恢复按名称寻找 checkpoint，身份与定义未绑定

旧图默认 checkpoint 文件名来自 workflow.name；文件存在就读取，解析失败又退回 fresh。读取路径没有验证当前图定义哈希、租户、项目与输入是否仍然对应旧执行。写文件的错误在 helper 中也被忽略。这会造成新任务错误复用旧状态或无法证明持久化成功。[S04]

处理：新图将 Start 与 Resume 分为两个 API，绑定 run_id、definition_digest 和 scope_digest；损坏文件是硬错误。持久化使用私有目录、临时文件、flush、rename 和 writer lease；恢复到 Running 的节点转为 Uncertain，禁止无证据自动重放。旧图可选加固只关闭不安全自动恢复并给新执行使用唯一文件名，不声称顺手完成旧格式迁移。旧图的吞写错误问题仍应随新图切换清除。

### 3.4 已确认：有拓扑层，但 inspected GraphEngine 仍逐节点 await

代码是 layer 外循环、step 内循环，并直接等待每个 execute_task。拓扑分层不等于实际并发。[S04]

处理：新图使用 FuturesUnordered，同时保留宿主 concurrency_key。默认所有节点共享 exclusive 键，所以默认仍串行；只有宿主确实为只读任务或独立 worktree 分配无冲突资源，才返回独立键并并行。这比让用户在 JSON 写 max_parallel=8 就获得并发写权更安全。

### 3.5 已确认结构、推断风险：全局父工具限制和深度计数

ToolServices 含单个 parent_task_tool_deny 和 sub_agent_depth。恢复 previous 值能帮助单线嵌套，但它不天然区分并发兄弟。是否在真实执行中触发跨任务串扰，需要并发回归证明；本报告把它标为高优先级结构性风险，而非已复现线上泄漏。[S06]

处理：RunContext 含 root、parent、depth、scope、capabilities、budget、cancel、deadline；子代理继承并收窄，而不是写全局槽位。旧共享状态没有在本补丁中被彻底删除；在完成迁移前，不得对共享旧 ToolServices 的写任务开放真正并行。

### 3.6 已确认：子代理预算与身份没有形成云端继承链

nested_task 创建新的 session_id，user_id 为 None，预算由环境变量函数读取。不能把这个模型直接当作多租户云执行的身份／预算隔离。父链总预算也不能用“每个子代理重新拿同样上限”表达。[S05]

处理：共享 BudgetPool 先 reserve 再 settle；子代理无法增加根上限。请求已发但用量未知时，保守计入预留最大值。精确货币费用、cache token 价格和账单结算仍由云平台后续的用量与钱包服务实现，补丁里的 token budget 不是人民币钱包。

### 3.7 已确认：后台任务状态文件不等于 durable execution

BackgroundAgentJob 是进程内状态；持久化的 state.json 标记 diagnostic_only。它对可观测性有价值，但不能据此声称进程重启后任务能安全续跑。[S06]

新实现不启动无归属的 detached 子代理任务。Supervisor 使用结构化 await、有限槽位和独立取消链；它本身也不宣称是分布式任务队列。需要跨进程执行时，应由宿主管理持久化 job、worker lease、恢复与对账。

### 3.8 已确认：安全层不能被新架构绕过去

execute_tool_call 当前持有工具表 read lock 并进入调用管线；其中含内容策略、审批、automem 特殊门控与 audit。[S07] 长 await 持有 registry read lock 可能影响热加载，但不能为释放锁而删除安全检查。后续可改为 Arc 工具句柄快照，并用单独 RunPolicy 做实时检查；这一优化不在本次已经完成的变更中。

## 4. 目标分层

```text
Tauri Desktop / Dashboard / Scheduler / future 818cloud worker
                     │ authenticated, scoped request
                     ▼
              composition root / host factory
                     │
        ┌────────────┴─────────────┐
        │ Graph + Supervisor       │    Skills / Computer / Verifier
        │ orchestration extensions│              │
        └────────────┬─────────────┘              │
                     ▼                            │
         one Harness Kernel ◄──── trusted Host adapters
         context → infer → checked tool → events
                     │
           existing LLM + security pipeline
                     │
             OS / browser / workspace
```

四个新增 crate 的职责：

| crate | 已编写的职责 | 刻意不承担 |
|---|---|---|
| harness-core | 单循环、消息投影接口、事件、journal、scope、预算、审批票据、session tree | UI、账户服务、OS 驱动 |
| harness-extensions | DAG、checkpoint、子代理、skills、computer broker、X11 后端、host subprocess、worktree | 第二套 LLM loop、用户账户库 |
| harness-host | 原生消息／provider 适配、Graph→Kernel、受检查执行接口 | 直接调用裸 Tool.execute、默许企业权限 |
| harness-cloud818 | 真实 SSO v2 introspection 客户端、身份与本地 ACL 接缝、用量事实 DTO | 第二套钱包、伪造 OIDC、未存在的生产计费 API |

原有 agent crate 增加 `harness-v1` feature 及 bridge。主入口没有自动切换，这让 Cursor 可以先完成 read-only pilot，再迁移完整生命周期。并存是迁移阶段，不是最终允许多个权威循环。

## 5. 图语义：边界必须可解释

当前提供 Work、Gate、Branch、Human 四类节点；支持依赖、all/any 合流、条件边、有限尝试、节点输出命名空间、显式恢复和资源冲突序列化。Any join 是“所有前驱达到终态后，至少一个匹配”，不是“最快一个先到就立即抢跑”。并行输出不写同一个全局 mutable map，因此不需要暗含 last-write-wins。

当前拒绝任意有环图。将来需要 LangGraph 式循环时，应引入显式迭代上限、superstep、reducer 冲突规则、checkpoint 版本和等待点语义，而不是去掉 cycle validation。用显式展开的版本化节点表达有限循环，当前可行但不等于原生循环支持。

任务重试只有宿主 safe_to_retry 才能允许；Uncertain 不自动重试。分布式副作用不存在“写了 checkpoint 就恰好执行一次”的保证：外部接口需幂等键，外部非幂等动作需要结果对账。LangGraph 的持久化文档也区分状态快照、恢复与 task 写入；这里借鉴问题分解，不宣称实现其全部执行语义。[S18]

FileCheckpoint 对一个操作使用进程内 lease，对文件使用进程间锁。它不是多机数据库。部署到 818cloud 集群前需要 PostgreSQL 等持久化适配、版本 CAS、worker fencing 和事务 outbox；不能将本机文件锁描述为集群锁。

## 6. 电脑操作不是“安装一个 skill”

ComputerBroker 约束设备 UUID、run、scope、有效期及独占控制。Observe 产出 frame_id、尺寸、PNG 与目标窗口；Act 使用具体动作参数生成审批 binding，一次消费。改变参数、换设备／run、过期画面、超界点击及未支持的按键会被拒绝。

目前有具体的 Linux X11 实现：调用宿主批准的 xdotool 与 ImageMagick import，可观察活动窗口、截图、点击、输入文字、有限按键和滚动。命令参数采用 argv，不拼接 shell。**没有实际启动本次环境的桌面控制、没有验证真实 X11 行为，也未完成 macOS Accessibility/ScreenCapture、Windows UIAutomation 或 Wayland 后端。**

X11 本身不是安全隔离。焦点校验与输入之间仍有竞态，应用级路径 canonicalize 也不能代替 OS sandbox。强安全场景应使用专用虚拟桌面／受控容器，并对原生设备单独登记和授权。host subprocess 的 kill_on_drop 只保证直接子进程，不是跨平台进程树回收；通用不可信 shell 必须等待 cgroup/job object 等 worker containment 后再开放。

已有 Browser/CDP 工具应优先用于浏览器任务；真实桌面辅助功能应处理浏览器以外的窗口操作，不应把一切降级为像素点击。[S07] 截图可能包含凭证或个人信息，默认不上传；模型视觉输入与云端留存必须分别经过策略。

## 7. Skills：扩展数量与权限分开

现有 bootstrap 已把全局、Agent、项目技能治理信息装配进 ToolServices。不能新增一个 Catalog 就覆盖安装、审核、项目启用和 Agent allowlist 规则。[S13]

新增 Catalog 是 metadata-first 的受限参考实现：扫描显式根目录、限制文件数与大小、校验名称和目录一致性、记录 SHA-256，激活时校验内容未变；读取正文需 skill.read 与宿主 allowlist。它不会执行 SKILL.md、运行安装 hook、自动 npm/pip install 或提升 capabilities。

附带 12 个技能说明，覆盖代码调研、Rust 重构、图编排、子代理、测试、浏览器、电脑、818cloud、FDE、文档分析、产物和发布复核。这些是可加载的工作指导，不是假装新实现了 PDF/Office/视频等底层工具。后续应通过资源适配把现有成熟技能库接到新 Harness。

Pi 扩展运行的是代码，并有系统权限风险；本补丁没有把任意 TypeScript 插件放进 Rust 进程执行。[S16] 要兼容其生态，建议另做受控 plugin worker/protocol，明确模块信任、版本 pin、进程生命周期和资源访问。插件代码与技能说明不能混称为“都安全的 skill”。

## 8. 818cloud 原生集成：复用真实契约

现有 Accounts 的 SSO v2 已有 authorize、token、introspect、revoke；交换采用客户端认证、精确 redirect URI、S256、一次性 code 和实时权限有效性校验。源码明确没有把它宣称为 OIDC provider。[S10,S11]

因此接入分为两个控制面：云身份由 Accounts UUID、组织、产品租户、外部租户映射及产品项目 ACL 决定；本机执行权由设备登记、工作区边界、OS 权限、用户审批和运行策略决定。组织不是租户，企业成员不是默认对所有产品有权限，SSO role 不是直接对所有 AnyCode project 放行。

新增服务端 AccountsClient 调真实 introspect，验证 active、iss、aud、sub、personal/enterprise 及 tenant 结构；随后必须经过 ProductAcl。它没有内置“允许所有项目”的默认实现。已有 user_id/local UUID 和外部租户映射必须由数据库层实际解析，不能通过邮箱相同自动合并旧用户。

桌面不能保存产品 client secret。本补丁中的 HTTP 身份 client 仅供可信服务端使用。浏览器侧可复用现有 product_sso 网关；桌面与 BFF 的配对、短期设备凭证、OS keychain、撤销和令牌更新仍是待实现的协议，已在 `desktop-pairing.PROPOSED.json` 中明确标记，不能在 UI 上假装已经接通。[S10–S12]

计费仅定义 UsageReceipt 事实：root/child run、turn、模型、token 和幂等键。没有扣费方法、没有新钱包表、没有把未知用量当零费用。平台结算需要 transactional outbox、幂等入账、provider usage 校准和已有钱包契约，当前未实装。818cloud 仓库、生产部署与存量数据未被修改。

## 9. 事件与恢复

Journal 保存原生消息与执行事实，具有递增 cursor 和哈希链，写失败会阻止继续；哈希链用于发现意外损坏，不是抵抗拥有目录管理员权限者的数字签名。必须使用宿主私有目录与账户授权读取。

UI 默认 preview 只发布运行元信息；原生 provider reasoning 不应直接发给前端。文本流明确标记 provisional。遇到 provider stream 错误，持久化对话不能包含半截工具指令。

SessionTree 提供 parent/leaf 的分支历史结构，但它没有在本补丁中替换既有 Dashboard session DB。完整对话树持久化、会话列表、断线重连与 renderer 仍需要迁移。不要把“有一个 tree 类型”当作“UI 的所有历史会话已迁移”。

## 10. 为什么用 feature-gated 渐进迁移

全量切换前，原系统的 compaction、memory hooks、failover、delivery acceptance、artifact renderer、session notifications 与 dashboard session scoping 必须保留行为。新 Host 的 transform/completion 是这些逻辑的接缝，不是已经全部迁移完成的证据。[S03,S13]

当前只读 Pilot 仅允许宿主指定目录的 FileRead，要求 sandbox_mode，拒绝企业 scope。桥接还拒绝旧 Agent/Task 等编排／控制工具，防止通过旧工具掉回另一个循环。这些限制是迁移保护，不应为了展示功能而删除。

下一阶段应先为每个旧入口建立 golden transcripts 和失败／取消用例，再把重复生命周期移动到 Kernel。完成 M1–M8 见 `CURSOR_START.md`。两条循环只允许在不同测试／配置的运行中对照，不允许同一个任务既旧循环又新循环。

## 11. 验证与非声明

已实际执行：Node 图模型测试、Python 安装／补丁应用与回滚夹具测试、JSON 示例与 Schema 校验、Cargo TOML 解析和文件清单检查。具体次数与日志见 `VALIDATION.md`。

未执行：Rust cargo check/test/clippy/fmt、完整 upstream 仓库应用、现有 workspace 全量回归、React TSX 编译、Tauri 编译／打包、实际 LLM、真实桌面输入和 818cloud PostgreSQL/Redis/SSO 联调。当前容器无 Rust 工具链，直接网络安装尝试无法解析下载域名。真实源码由 GitHub connector 读取，不等于整个仓库已经挂载到容器。

因此发布门禁保持关闭。Cursor 必须先编译修正潜在类型／借用问题、审阅 Cargo.lock、补足产品接线、完成集成和端到端测试，再考虑启用任何写操作或企业部署。

## 12. 来源索引

S01 AnyCode `Cargo.toml`，blob b58a41a6a1c0b9a04624055d57d25ab7e732ddc4。
S02 AnyCode `AGENTS.md`，blob 5cf7588ced081697ca59a0585e7489206bd07e9b。
S03 AnyCode `crates/agent/src/runtime/{mod.rs,agentic_loop.rs,agentic_turn.rs}` 与 `crates/agent/src/lib.rs`。
S04 AnyCode `crates/agent/src/graph_engine.rs`，blob b0267172732127fbe9416e586ee1b1d423874a8e；读取到核心 run、完成判定和 persist helper。
S05 AnyCode `crates/agent/src/runtime/nested_task.rs`，blob 6811a328f63f1071a9c4ca9629bffae112218ecf。
S06 AnyCode `crates/tools/src/services.rs`，blob db70c9dc68cd6be519a302b80330e5572073b541；读取 1–240、310–490 行。
S07 AnyCode `crates/tools/src/registry.rs`、`runtime/execute_tool.rs` 与 `runtime/tool_invocation.rs`；调用管线 blob 4f6a43b8c71cda4a209e1c7c803e2f86c2bb6b6e。
S08 AnyCode `crates/core/src/{traits.rs,message.rs,lib.rs}`。
S09 AnyCode `crates/core/src/llm_types.rs`，blob d710cb1012096fc8f8a932845a593c40204994a1。
S10 818cloud `lingxi-accounts/src/product_sso.rs`，blob 595cad646edf01159a9e7d298e115794968c8c50。
S11 818cloud `lingxi-accounts/src/platform/sso.rs`，blob 624da9b40b8d2fbd780ff2e15ada00c391cc0a39。
S12 818cloud `README.md` 及 Accounts/platform 目录结构。没有读取真实生产数据。
S13 AnyCode `crates/bootstrap/src/runtime.rs` 1–190 行，blob 3325d3754fa7822e4f87e715b0d65c151ea413b4。
S14 AnyCode `crates/dashboard-ui/package.json`，blob 6f12f14e14b6a34193c9bf576acb8ab8c70e1b38。
S15 Pi agent-core README：https://github.com/earendil-works/pi/tree/main/packages/agent
S16 Pi extensions 文档：https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md
S17 Pi agent-loop.ts 固定提交：https://github.com/earendil-works/pi/blob/7b4cfd6eb0fd490e3b54370ec9e9227b29717cb4/packages/agent/src/agent-loop.ts
S18 LangGraph persistence：https://docs.langchain.com/oss/python/langgraph/persistence
S19 Grapl 官方仓库：https://github.com/grapl-security/grapl

S01–S09、S13–S14 均基于上述 AnyCode commit；S10–S12 基于上述 818cloud commit。以上判断区分直接代码事实、工程建议和未复现风险。
