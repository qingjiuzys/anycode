# 廉价 AI 分阶段提示词

每 Phase **单独开新对话**，整段复制对应 Prompt。不要一次做多个 Phase。

---

## Phase 0 — 生成静态原型（强模型 / 人工，一次性）

**Role:** 你是前端工程师，根据 `design/glass-skins/SOURCE_FILES.md` 创建全部文件。

**Steps:**
1. 读取 `SOURCE_FILES.md`，按「文件路径」小节逐个创建文件。
2. 确保 `prototype/index.html` 在浏览器可离线打开。
3. 验证 4 皮肤 × 2 明暗模式切换正常。
4. 不要修改 `crates/` 下任何文件。

**Acceptance:**
- [ ] `shared/` 下 5 个文件存在
- [ ] `prototype/` 下 5 个 HTML 存在
- [ ] localStorage 键为 `anycode-dashboard-skin` 和 `anycode-dashboard-theme`

---

## Phase 1 — Token 层（仅 CSS）

```
你是前端工程师，只做 CSS token 迁移。

必读：
- design/glass-skins/TOKENS.md
- design/glass-skins/shared/tokens.css
- design/glass-skins/shared/glass.css
- crates/dashboard-ui/src/index.css

任务：
1. 将 tokens.css + glass.css 的核心变量合并进 index.css（@theme 或 :root 区块）。
2. 添加 html[data-skin="mono|indigo|coral|teal"] 四套 accent 定义。
3. 映射：--color-primary → var(--accent)，--color-background → var(--bg)。
4. 不要修改任何 .tsx 文件。

验收：
- [ ] document.documentElement.dataset.skin = 'coral' 时 primary 为 #e8826b
- [ ] cargo fmt / 无 TS 改动
- [ ] 现有页面不崩溃（允许颜色变化）

禁止：hardcode 新 hex 在 TSX；删 html.dark 规则；引入新 npm 包。
```

---

## Phase 2 — Skin Hook

```
你是 React 工程师，添加皮肤选择状态。

必读：
- crates/dashboard-ui/src/hooks/useTheme.ts（照抄模式）
- design/glass-skins/TOKENS.md（storage 键名）

任务：
1. 新建 hooks/useSkin.ts：Skin = 'mono'|'indigo'|'coral'|'teal'，localStorage anycode-dashboard-skin，默认 indigo。
2. main.tsx 首屏前 applySkin(getSkin())，设置 document.documentElement.dataset.skin。
3. 新建 components/SkinPicker.tsx：4 色 swatch + 选中 ring。
4. 挂到 pages/settings/SettingsPreferencesSection.tsx。
5. i18n：zh.ts / en.ts 增加 settings.skin* 文案。

验收：
- [ ] Settings 选皮肤后刷新仍保留
- [ ] 与 light/dark 独立组合
- [ ] npm test 通过

禁止：改 API；删现有 theme toggle。
```

---

## Phase 3 — Shell 玻璃化

```
你是前端工程师，只改 App Shell 视觉。

必读：
- design/glass-skins/prototype/home.html（sidebar/topbar 结构）
- design/glass-skins/shared/base.css
- crates/dashboard-ui/src/components/Layout.tsx
- crates/dashboard-ui/src/index.css（.dw-sidebar .dw-topbar）

任务：
1. .dw-sidebar 加 glass-panel 效果（backdrop-filter + var(--glass-bg)）。
2. .dw-topbar 同样玻璃化；保留 html.dw-tauri drag region 规则。
3. .dw-nav-link--active 使用 var(--accent-muted) + 左边 2px accent 条。
4. 不改 NAV 数组、不改路由、不改 API 调用。

验收：
- [ ] Tauri 下 topbar 可拖拽，按钮 no-drag 可点
- [ ] 4 皮肤下 nav active 态 accent 正确

禁止：改 Outlet 逻辑；删 SseStatusBadge。
```

---

## Phase 4 — Home 页

```
你是前端工程师，对齐 Home 视觉到 HTML 原型。

必读：
- design/glass-skins/prototype/home.html（全文）
- crates/dashboard-ui/src/pages/HomePage.tsx
- crates/dashboard-ui/src/components/HomeHeroComposer.tsx

任务：
1. 结构对齐：.dw-home-hero + .hero-glow + .dw-composer + .dw-suggestion-grid。
2. 发送按钮改用 accent 圆形 .btn-accent 风格。
3. 只改 className 和 CSS，不改 submit / API 逻辑。
4. 删除或替换 hardcode 珊瑚色 #e8a090，改用 var(--accent)。

验收：
- [ ] Home 在 4 皮肤下 accent 一致
- [ ] composer focus ring 可见

禁止：改 useMutation；删 i18n key。
```

---

## Phase 5 — Conversations 页

```
你是前端工程师，对齐三栏会话 UI。

必读：
- design/glass-skins/prototype/conversations.html
- crates/dashboard-ui/src/pages/ConversationsPage.tsx
- crates/dashboard-ui/src/components/ConversationTranscript.tsx

任务：
1. 会话列表栏 glass 化；active session pill 用 accent-muted。
2. User bubble → var(--solid-bubble)；Assistant → glass-panel。
3. .code-block / .tool-strip 样式迁入 index.css 或组件 class。
4. 不改 blocksToTurns / API / SSE 逻辑。

验收：
- [ ] 三栏布局在 1280px 正常
- [ ] 代码块可读

禁止：改 sanitizeAssistantDisplay 逻辑。
```

---

## Phase 6 — Overview 页

```
你是前端工程师，对齐 Overview KPI + chart 容器。

必读：
- design/glass-skins/prototype/overview.html
- crates/dashboard-ui/src/pages/OverviewPage.tsx
- crates/dashboard-ui/src/components/MetricsChart.tsx

任务：
1. KPI 卡片 glass-card 化。
2. ECharts  legend/textColor 改读 CSS 变量（getComputedStyle），不要 hardcode #505f76。
3. 不改 overview API 数据结构。

验收：
- [ ] 图表在 dark/light 下轴标签可读
- [ ] 4 皮肤下 chart 线条可用 accent 或 neutral gray

禁止：换 chart 库。
```

---

## Phase 7 — 动效 + 收尾

```
你是前端工程师，添加 motion 并跑测试。

必读：
- design/glass-skins/MOTION.md
- design/glass-skins/shared/motion.css

任务：
1. 复制 motion.css 规则到 index.css。
2. 添加 prefers-reduced-motion 媒体查询。
3. LoginPage.tsx 登录卡片简单 glass-panel（可选）。
4. 跑：cd crates/dashboard-ui && npm test && npm run build

验收：
- [ ] reduced-motion 下无 hover translate
- [ ] build 成功

禁止：引入 framer-motion。
```

---

## 常见错误（所有 Phase）

- 在 TSX 里写 `#6e6bff` 而不是 `var(--accent)`
- 删除 `html.dark` 或现有 MD3 变量
- 创建 tailwind.config.js（项目用 Tailwind v4 CSS-first）
- 一次 PR 做多个 Phase
- 改 crates/dashboard 后端
