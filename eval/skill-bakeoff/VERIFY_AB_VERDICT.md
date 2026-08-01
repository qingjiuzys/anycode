# verify-discover vs verification-before-completion

Run: `runs/20260801-091251` · model: deepseek-v4-flash

| skill | role | result |
|---|---|---|
| verify-discover | 识别栈 → 发现验法 → 跑通最小证明 | 写出完整 pipeline 发现 + e2e docx 证据 |
| verification-before-completion | 宣称完成前的 Iron Law 门禁 | 对「可执行+语法正确」做新鲜命令取证后才下结论 |

**Decision: 并存 (coexist)** — 不互相替换。
- `verify-discover`：不知道怎么验时用（开放发现）
- `verification-before-completion`：准备说「好了/通过了」时用（证据门禁）

`verification-before-completion` 已 promote 进 `skills-starter/`（来源 obra/superpowers，MIT）。
