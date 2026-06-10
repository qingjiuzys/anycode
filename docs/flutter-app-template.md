# Flutter App 项目模板（`flutter-app`）

内置模板：创建时只写入 **骨架 + skills + 工作流**；Flutter SDK、平台目录、门禁由 **Agent 自主决定如何完成**（读 `.anycode/flutter-project.json`，用 Bash/工具按环境安装，**不**绑定某个安装脚本）。

## 设计原则

| 阶段 | 行为 |
|------|------|
| **创建项目** | `skeleton/` + `overlay/` + `.anycode/`（含 `flutter-project.json`） |
| **Agent** | Skill `flutter-bootstrap`：自行装 SDK、`flutter create`、`pub get` |
| **门禁** | `flutter analyze` / `flutter test`；`scripts/verify.sh` 仅给人/Dashboard 一键跑 analyze+test（**不**装 SDK） |

`scripts/` 不是 Agent 的执行契约，只是可选快捷方式。

## CLI

```bash
anycode project init --template flutter-app --path ./my_app --name my_app --title "我的 App"
```

## Dashboard

新建项目 → **Flutter 应用** → 无需预装 Flutter。

## Skills

- `flutter-bootstrap` — 自主准备环境与平台目录  
- `flutter-prd` / `flutter-screen-plan` / `flutter-ui-polish` / `flutter-gate-fix`

## 可选

本机已有 Flutter 且创建时就要平台目录：`anycode project init ... --flutter-create`
