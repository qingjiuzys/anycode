# Cursor 接续开发入口：先编译、再迁移，不能伪完成

## 你拿到的是什么

这是 AnyCode 的 Pi-inspired Rust Harness 第一阶段基础补丁，不是已经替换全产品的成品。不要从头重写已有工具，不要将所有项目糅合进 818cloud，不要为了跑 demo 放开权限。先读 `RESEARCH.zh-CN.md`、`VALIDATION.md`、`docs/harness/ARCHITECTURE.md` 和原仓库 AGENTS.md。

基线 AnyCode：`0411ea3a94fe6aa2d37326f9342cfc5d6a13f4aa`。基线 818cloud：`e8c9b49eafd12a69cf2007e59cd25b3d9b52683e`。当前代码若更新，按具体源文件重放补丁，不要 reset 用户分支。

## M0：接收补丁并验证编译（必须先做）

将解压目录放在仓库外，在干净新分支执行：

```sh
python3 /path/to/anycode-harness-refactor/apply.py --repo /path/to/anycode --check
python3 /path/to/anycode-harness-refactor/apply.py --repo /path/to/anycode --apply
cd /path/to/anycode
python3 tools/harness/verify.py --mode node
python3 tools/harness/verify.py --mode standalone-rust
python3 tools/harness/verify.py --mode integrated-rust
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
```

没有替你生成 Cargo.lock：让 targeted cargo check/test 正常补齐本地 path dependencies，审阅锁文件 diff，禁止不必要的全量 cargo update。修复可能的 Rust 类型、Send、borrow 和 API 兼容问题；不能把尚未执行的 Rust 用例写成通过。对照默认 feature 关闭与开启后的编译，不要把 compile failure 归咎于“以后再接”。

旧图的可选加固必须在**首次应用时**增加 `--legacy-hardening`；它关闭旧图自动复用 checkpoint、自动副作用重试和无证据 required_gates，Partial 不再通过。已应用默认补丁后，不要重复跑安装器覆盖；直接依据 `legacy-hardening.json` 审阅后重放变更。

## M1：只读真实 Host Pilot

在 `initialize_runtime` 的 composition root 或一个真实测试 host 中使用 `AgentRuntime::harness_host`。提供合法 RunContext、同一项目根路径、只含 FileRead 的 bindings、ReadOnlyPilotBoundary 和原生模型配置，然后调用 Kernel.run。不能直接 Tool.execute。

现有 `execute_task` 和 `execute_turn_from_messages` 默认仍旧路径。新增显式实验开关，经回归批准后仅切一个只读入口。图组件仅在 host 回调真正接通时启用执行按钮；禁止 mock API 返回固定成功。

验收：真实一次读文件任务、Done 前无工具执行、流中断失败、父取消有效、实际 token usage 入账、原生 reasoning/tool IDs 不被破坏。不要把 provider-private reasoning 展示给用户。

## M2：把两个旧入口收敛为同一内核

重点文件：`runtime/execute_task.rs`、`execute_turn.rs`、`agentic_turn.rs`、`tool_dispatch.rs`、`execute_turn_finalize.rs`。先建立 golden transcripts 和预算／错误／取消基准。

把 compaction、memory recall/save、profile prompt、failover、language、artifact、delivery acceptance、session notify 接到 Host 的 transform／inference／completion 生命周期。修改的是调用层，不删除原有功能。最终 execute_task、execute_turn、graph node、subagent 都必须落到同一循环。

验收：原聊天和 scheduler 入口测试不退化，新增 Kernel 不在旧循环里再次启动循环；最终更新 ADR，明确唯一权威。

## M3：完整 subagent 接线

复用 `Supervisor` 与 `KernelNodeExecutor`。把 Agent profiles 解析为可信能力子集；新 child RunContext 继承 scope／root／budget／deadline，而非读环境变量重新分配。将 `sub_agent_depth` 和 `parent_task_tool_deny` 从全局槽位迁走。

写任务必须使用独立 worktree 或宿主执行容器。默认 concurrency_key=exclusive；只有真实隔离后才允许并发。添加父子取消、两兄弟权限不串扰、预算合计、深度与容量拒绝、异常释放槽位用例。再提供 AgentSpawn/Join/Cancel 的 Tool 包装与 UI 事件。

## M4：Graph API 与 UI 接到真实执行

图扩展已有执行器和 ReactFlow 组件；尚未挂载产品路由。新增产品路由时，认证、项目 ACL、CSRF、运行 lease、输入大小、revision 和事件 scope 都必须检查。Start 和 Resume 必须不同语义。

将 `AgentHostFactory::build` 接到现有 profile/模型/安全服务；将 verify 接到实际测试命令、产物 hash 和报告存储。样例 verifier ID 不是已实现验证器。Human 决策要验证审批人授权，并记录事实；不能让 LLM 直接调用宿主 resolve_human 来自批。

验收：条件分支、all/any、人工暂停、跨租户 resume 拒绝、定义变化拒绝、坏 checkpoint 拒绝、进程崩溃 Running→Uncertain、Partial 不放行、真实 gate 失败。需要 LangGraph 循环时新增有界迭代语义，不删除 cycle 检查。

## M5：电脑与技能成为可发现工具

为 ComputerBroker 提供宿主授权的 device/lease，给模型暴露 Observe/Act 的受限 schema；票据只由可信 UI 发出。先在专用 X11 虚拟桌面做真实测试，再接 macOS Accessibility/ScreenCapture 和 Windows UIAutomation。当前 X11 后端并未实机验证，不能宣传为已支持全部桌面。

复用原 SkillCatalog 和 SkillsGovernance，把 metadata/search/activate 接到现有 skills install/review/project allowlist；示例 skill pack 不自动安装。不要让 Markdown 配置授予系统权限。提供 prompt injection、symlink、内容变化和越权激活用例。

## M6：818cloud 产品原生集成

服务端使用 `harness-cloud818` 调真实 `/api/v2/sso/introspect`，紧接 ProductAcl。Desktop 禁止携带 PRODUCT_SSO_CLIENT_SECRET；浏览器使用既有 product_sso gateway，桌面 BFF pairing 参照 PROPOSED 契约另做。

复用 Accounts UUID 与产品租户 external_tenant_id，不建新账户真源；不合并钱包。对企业 revoke、member/grant version 变化、个人/企业切换、其他产品 aud 令牌、错误项目、设备撤销做真实 PostgreSQL/Redis 集成测试。

## M7：恢复、账单和部署强化

文件 checkpoint 不是集群数据库。实现持久化 RunStore、CAS/fencing、节点 lease、budget ledger 和 transactional outbox。用量事实与钱包结算分开，补全 cache token/cost；重放幂等，未知结果对账。设备 token 进 OS keychain，服务密钥进入服务器 secret store。

为主进程/子进程树提供 cgroups、job objects 或等效隔离；当前 subprocess primitive 不能当作通用不可信 shell sandbox。没有这些条件不开放云多租户任意命令执行。

## M8：最后切换与验收

完成真实 LLM、现有 Workbench、scheduler、graph、subagent、desktop 与 818cloud 端到端联调后，逐租户／项目灰度启用。保留回滚开关及存量数据兼容。模型说完成、构建成功、镜像推送、线上切流是不同事实，报告应分别描述。

## 不得绕过的约束

不删除测试来消除失败；不伪造 endpoint、token、gate evidence、provider usage 或生产状态；不把未授权用户设为 admin；不自动部署；不从用户机器复制凭证；不让远程页面指令越过宿主授权。保留旧账户、钱包、订单与本地 session，不做破坏性清库或自动合并。
