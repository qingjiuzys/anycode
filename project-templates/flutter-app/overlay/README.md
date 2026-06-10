# {{app_title}}

anycode **flutter-app** 项目模板（Agent 自主环境 + MVP 工作流）。

## 门禁

- Dashboard：`flutter analyze`、`flutter test`（需 Agent 已准备好 Flutter 环境）
- 可选本地：`scripts/verify.sh` 仅串联 analyze/test，**不**替代 Agent 安装 SDK

## 对话生成

```bash
anycode run -C . --agent goal-runner \
  --goal "Build Flutter MVP in this project" \
  --done-when "GOAL_ACCEPTANCE_OK" \
  "按 PRODUCT_BRIEF 与 PROMPTS.md 实现 iOS 优先 Flutter App"
```

## Skills

`.anycode/skills/`：`flutter-bootstrap`（自主装 SDK/平台）、`flutter-prd`、`flutter-screen-plan`、`flutter-ui-polish`、`flutter-gate-fix`。

元数据：`.anycode/flutter-project.json`。

GOAL_ACCEPTANCE_OK
