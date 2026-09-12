# 验证报告 · 2026-09-11

## 结论

这是经过源码审阅、离线安装测试及部分单元测试的 **Phase 1 重构基础补丁**。不是完整产品重写，不是 Rust 编译通过证明，也不是可直接上线的构建产物。

| 检查 | 实际结果 | 证明范围 |
|---|---|---|
| Node.js 图模型单元测试 | **18 / 18 通过** | 图校验、布局输入、状态与人工确认版本等纯模型逻辑；不含 React 组件渲染 |
| Python 补丁工具测试 | **14 / 14 通过** | 路径边界、源片段替换、Git blob 计算、清单结构及通用应用/回滚 |
| JSON Schema | **Schema 自检和 2 份示例通过** | JSON 结构符合 v1 schema，不代表图已在真实宿主执行 |
| 交付的整份 foundation.patch | **离线夹具中 check、apply、reverse-check、reverse 全部通过** | 新增文件内容逐字节一致；不等同完整上游仓库测试 |
| 源文件基线 | **3 份完整文件 Git blob 一致；1 份只核验前 8 行** | Cargo.toml、agent/Cargo.toml、agent/lib.rs 为完整匹配；runtime/mod.rs 仅已读取前缀 |
| Rust 用例 | **42 个测试定义已提供，未执行** | 运行验证入口返回 exit code 2，明确报告 Cargo 缺失 |
| Cargo.lock / cargo fmt / clippy / workspace build | **未执行，未生成新锁文件** | 交接后必须完成并审阅 |
| React TSX 类型检查 / 浏览器 E2E | **未执行** | 只完成 .mjs 模型测试，不能声称图工作台编译或渲染通过 |
| 真实模型、电脑、818cloud | **未执行端到端联调** | 未调用真实 LLM、未操控实机、未访问生产 Accounts/数据库/钱包 |

## 测试命令与原始输出

在交付包内执行：

```sh
node --test overlay/crates/dashboard-ui/src/features/harness/graph-model.test.mjs
python3 -m unittest discover -s tests -v
python3 overlay/tools/harness/verify.py --mode standalone-rust --repo overlay
```

前两条退出码为 0。第三条实际结果是：

```text
BLOCKED/FAILED: Rust cargo is not installed; Rust tests NOT executed
EXIT_CODE=2
```

本环境已确认没有 cargo、rustc、rustfmt。Node 为 v22.16.0。所有原始输出保存在 reports/：node-tests.txt、python-tests.txt、rust-not-executed.txt、schema-tests.json、patch-fixture.json 和 counts.json。

## 完整 patch 验证与完整上游验证的区别

整份 patch 的每个新增文件均在临时 Git 仓库中应用并与 overlay 字节比较，再反向移除。四处修改中的三处使用完整、blob 已匹配的基线文件；runtime/mod.rs 使用已核实的前 8 行，并添加明确标记的合成尾部检查 diff 不会截断后续内容。这证明交付 diff 的离线应用/回滚有效，**不证明完整 AnyCode workspace 可以编译，也不证明安装器在完整上游源树上执行过**。

随包 apply.py 在真实仓库中仍要求全部四个被修改文件的完整 blob hash 匹配；runtime/mod.rs 的完整预期 hash 来自 GitHub，不会降级到只比较前缀。源码不匹配时拒绝自动覆盖，不使用 --reject、强制 reset 或自动合并。

## 尚未关闭的发布阻断项

1. 编译全部新增 crate 和 harness-v1 bridge，补齐 Cargo.lock、格式化、Clippy 与现有全量测试；本报告不能排除 Rust 类型、借用或 API 兼容问题。
2. 将两个旧入口、图和子代理真正接到新内核，迁移原有 compaction、failover、memory、approval、验收和 session 生命周期。默认补丁没有切换旧聊天/调度入口，也没有默认修改旧 GraphEngine 的行为。
3. 连接可信 Graph verifier、企业 ProductAcl 数据映射、设备授权及宿主动作审批，完成跨租户隔离、撤销、恢复与取消的真实测试。
4. X11 电脑后端需专用桌面实测；macOS、Windows、Wayland 后端尚未实现。子进程 helper 不是 OS 沙箱，也不能保证整个子进程树被终止。
5. 818cloud 服务端 SSO v2 适配已写，但产品登录、桌面 pairing、分布式运行存储、用量 outbox 与钱包结算没有完成。没有部署或迁移存量账号/钱包/订单。

## 完整性说明

manifest.json 中的 SHA-256 清单用于发现传输/编辑变化，不是数字签名。ZIP 和 patch 的发布校验记录在外部 .sha256 文件。该报告不把 source presence、fixture pass、build pass、integration pass 和 production readiness 混为一谈。
