# Glass Skins — 设计原型包

4 套 macOS 毛玻璃皮肤（Mono / Indigo / Coral / Teal），每套支持 light/dark。供 anyCode Desktop 客户端 UI 改版。

## 快速开始

1. **生成静态文件**（若 `shared/`、`prototype/` 尚未存在）：
   ```bash
   cd design/glass-skins
   bash scripts/extract-source-files.sh   # 见 SOURCE_FILES.md 说明
   ```
   或切换到 **Agent 模式**，让 AI 根据 `SOURCE_FILES.md` 写入全部文件。

2. **预览**：
   ```bash
   open design/glass-skins/prototype/index.html
   ```

3. **切换皮肤**：页面顶部 4 色圆点 + Light/Dark 开关；选择写入 `localStorage` 键 `anycode-dashboard-skin` / `anycode-dashboard-theme`。

## 目录（目标结构）

```
design/glass-skins/
├── README.md           ← 本文件
├── TOKENS.md           ← 色值唯一真相源
├── COMPONENTS.md       ← DOM class 约定
├── MOTION.md           ← 动效规范
├── PROMPTS.md          ← 廉价 AI 分阶段提示词
├── SOURCE_FILES.md     ← 全部 CSS/JS/HTML 源码（复制即用）
├── shared/
│   ├── tokens.css
│   ├── glass.css
│   ├── motion.css
│   ├── base.css
│   └── shell.js
└── prototype/
    ├── index.html
    ├── home.html
    ├── conversations.html
    ├── overview.html
    └── settings-skin.html
```

## 皮肤一览

| ID | 名称 | Accent |
|----|------|--------|
| `mono` | 无彩色 | `#ffffff` |
| `indigo` | 电光蓝紫 | `#6e6bff` |
| `coral` | 珊瑚暖橙 | `#e8826b` |
| `teal` | 青绿 | `#2dd4bf` |

## 迁入 React 顺序

见 [PROMPTS.md](./PROMPTS.md) Phase 1–7。**不要跳步**；每 Phase 单独开对话。

## 主攻范围

- App Shell（sidebar + topbar + Tauri drag）
- Home / Conversations / Overview
- Settings 皮肤选择器

不在此包内：后端 API、28 个 Settings 子面板内容重写。
