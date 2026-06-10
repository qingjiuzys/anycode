# 工作运行 · 三 Flutter App 交付报告

日期：2026-06-04

## 执行摘要

基于互联网需求调研，在固定目录交付 3 个 iOS 优先的 Flutter MVP。

### 门禁验收（2026-06-04 复测）

| 门禁项 | 来源 | 01 | 02 | 03 |
|--------|------|----|----|-----|
| `flutter analyze` | `gate_runner` preset | 通过 | 通过 | 通过 |
| `flutter test` | `gate_runner` preset | 通过 | 通过 | 通过 |
| `GOAL_ACCEPTANCE_OK` | `goal_engine` README | 有 | 有 | 有 |
| `widget_test` tap+pumpAndSettle | `goal_engine.rs` | 有 | 有 | 有 |
| `flutter build ios --simulator` | iOS 冒烟 | **失败** | **失败** | **失败** |

**iOS 构建失败原因（非应用代码）**：Xcode 26.5 需要安装 **iOS 26.5 Platform**（Xcode → Settings → Components），当前仅装有 Simulator Runtime **iOS 26.3**，报错 `iOS 26.5 is not installed`。此前仅用 Chrome 预览，**未在模拟器上真实跑通**，此处更正。

**环境修复（需本机一次）**：

```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
# Xcode → Settings → Components → 下载 iOS 26.5
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
./scripts/work-run-verify-flutter-apps.sh
./scripts/work-run-ios-simulator.sh   # 三端依次 flutter run
```

| 目录 | 产品 | 流量机制 | 验收 |
|------|------|----------|------|
| `test-flutter-01` | 心迹 · 心情日记 | 连续打卡、周回顾分享 | analyze ✓ test ✓ |
| `test-flutter-02` | 灵感日签 | 分享、收藏、连续打开、onboarding | analyze ✓ test ✓ |
| `test-flutter-03` | 习惯跃迁 · 21 天挑战 | 进度环、徽章、成就分享 | analyze ✓ test ✓ |

## 需求依据

详见 [MARKET_RESEARCH.md](./MARKET_RESEARCH.md)。三类均落在 wellness + 轻工具 + 社交分享 赛道，MVP 可在单目录内自动化验收。

## UI 打磨要点

- Material 3 + 种子色区分品牌（紫 / 橙 / 绿）
- 渐变欢迎页、卡片阴影、NavigationBar 三栏结构
- 全站简体中文文案
- 浅色 / 深色 / 跟随系统主题

## 与 anycode 的关系

- 配置：`anycode status` 显示 `deepseek-v4-pro` 已就绪
- Goal 引擎要求：`README.md` 含 `GOAL_ACCEPTANCE_OK`，`widget_test.dart` 含 `tester.tap` + `pumpAndSettle`
- 可选用 Goal 模式再生/扩展：

```bash
anycode run -C /path/to/anycode \
  --agent goal-runner \
  --goal "Build MVP in test-flutter-01 only" \
  --done-when "GOAL_ACCEPTANCE_OK" \
  "按 PRODUCT_BRIEF 完善心迹 App"
```

## 批量验证

```bash
./scripts/work-run-verify-flutter-apps.sh
```

## 下一轮增长实验

1. **心迹**：周情绪折线图 + 推送提醒 A/B
2. **灵感日签**：分享卡片海报图 + 邀请码
3. **习惯跃迁**：好友榜 / 双人挑战 + 订阅去广告

## iOS 真机

各目录 `RUNBOOK.md` 含 `flutter run -d ios`。若签名失败，使用模拟器或 `flutter build ios --no-codesign` 做构建冒烟。
