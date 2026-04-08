---
title: 版本与特性开关
description: 版本号、GitHub Releases、以及 anycode enable/disable 实验能力。
summary: 更新发布渠道；用统一 CLI 入口切换运行时 feature。
read_when:
  - 发布或安装 anyCode 构建。
  - 需要 enable/disable 管理实验功能。
---

# 版本与特性开关

## 版本与发布

- **版本号**：工作区根目录 `Cargo.toml` 的 `version`。
- **GitHub Releases**：对常用平台打 tag 并附带 `anycode` 二进制（非 `cargo install` 场景）。
- **文档站**（`docs-site/` VitePress）：GitHub Pages 部署时设置 `VITEPRESS_BASE=/仓库名/`。

## 运行时特性（enable / disable）{#runtime-feature-flags}

```bash
anycode enable skills
anycode disable workflows
anycode status
```

名称与 `anycode_core::FeatureFlag` 一致：

| 能力 | enable / disable 参数 |
|------|------------------------|
| CLI skills 扫描 | `skills` |
| 工作流相关 | `workflows` 或 `workflow` |
| 目标模式配套 | `goal-mode` 或 `goal` |
| 通道模式配套 | `channel-mode` 或 `channel` |
| 实验审批 | `approval-v2` 或 `approval` |
| 上下文压缩配套 | `context-compression` 或 `compact` |
| 工作区 profile | `workspace-profiles` 或 `workspace` |

## 相关

- [总览](./cli)  
- [路由](./routing)  
