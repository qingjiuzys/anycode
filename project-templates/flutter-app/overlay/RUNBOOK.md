# {{app_title}} · RUNBOOK

## 原则

- **创建项目**：只落 Dart/anycode 骨架，**不**要求本机已有 Flutter。
- **环境与 SDK**：由 **Agent 自主** 安装/配置（读 `.anycode/flutter-project.json`，自行选择 brew、官方 SDK、路径等）。`scripts/` 下的文件仅供人工或 Dashboard 一键门禁，**不是** Agent 的执行契约。

## Agent 目标（非脚本步骤）

1. `flutter` 可用  
2. 按 `platforms` 补齐 `ios/` / `android/` / `web/`（通常 `flutter create . --project-name … --org … --platforms=…`）  
3. `flutter pub get` → `flutter analyze` → `flutter test`  

Skill：`flutter-bootstrap`、`flutter-gate-fix`。

## 人工 / Dashboard 门禁（可选）

若本机已装好 Flutter，可手动：

```bash
flutter pub get && flutter analyze && flutter test
```

或 Dashboard 预设 **project verify**（调用 `scripts/verify.sh`，仅跑 analyze/test，**不会**代替 Agent 安装 SDK）。

iOS smoke（可选）：`REQUIRE_IOS_BUILD=1 bash scripts/verify.sh`

## iOS / Xcode

`export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`；Platform 版本需与模拟器一致。
