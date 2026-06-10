# 对话生成流程（flutter-app 模板）

0. **环境（Agent 自主）**：读 `.anycode/flutter-project.json`，自行安装/配置 Flutter 与平台目录（见 skill `flutter-bootstrap`），**不要**默认执行某个固定 shell 安装脚本。
1. **PRD**：完善 `PRODUCT_BRIEF.md`（`flutter-prd`）
2. **结构**：页面与状态（`flutter-screen-plan`）
3. **实现**：`lib/` MVP
4. **门禁**：`flutter analyze` / `flutter test`（`flutter-gate-fix`）
5. **打磨**：`flutter-ui-polish`
6. **验收**：`README.md` 含 `GOAL_ACCEPTANCE_OK`，测试通过
