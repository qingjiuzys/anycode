# anycode-ppt 组件索引

**用法**：从 `templates/` 复制最接近的 HTML，只改文案/数据，保留 `:root` 令牌与主视觉 class 名。

**交付**：分页 HTML + `run` 生成 `index.html` 浏览器预览。**不导出 pptx。**

## 页型模板

| 文件 | 用途 | data-type |
|------|------|-----------|
| `cover.html` | 封面 + 迷你 ladder | cover |
| `section.html` | 章节分隔 + 议程 | section |
| `closing.html` | 收尾 / Next Steps | closing |
| `diagram-image.html` | 插图 / 截图主视觉 | content |

## 讲解图组件（content 页必选其一）

| 类名 | 模板 | 适用场景 |
|------|------|----------|
| `.ladder` | `ladder-flow.html` | 线性流程 / 阶段梯子 |
| `.layer-stack` | `layer-stack.html` | 四层技术栈（Agent 分层） |
| `.layer-stack-4` | `layer-stack-4.html` | 四层架构 + 右侧上下箭头侧栏 |
| `.agent-cycle` | `agent-cycle.html` | 五步环状 Agent 循环 + 中心 Harness |
| `.duo` | `duo-compare.html` | 双卡对比 |
| `.trio` | `trio-cards.html` | 三列要点卡片 |
| `.metrics` | `metrics-kpi.html` | KPI 数字 + 解读 |
| `.quote` | `quote-insight.html` | 金句 / 核心结论 |
| `.timeline` | `timeline.html` | 横向路线图 / 里程碑 |
| `.checklist` | `checklist.html` | 行动清单 + 侧栏预告 |
| `.diagram-box` / `<img>` | `diagram-image.html` | 外链图示 |

## 选用原则

| 内容结构 | 用哪个 |
|----------|--------|
| 分层 / L1–L4 / 底座→应用 | `layer-stack-4.html` |
| 多轮循环 / Tool loop | `agent-cycle.html` 或 `ladder-flow.html` |
| A vs B 对比 | `duo-compare.html` |
| 三个并列要点 | `trio-cards.html` |
| 数字 KPI | `metrics-kpi.html` |
| 一句定调金句 | `quote-insight.html` |
| 按周/按阶段推进 | `timeline.html` |
| 课后作业 / 验收清单 | `checklist.html` |
| 已有 PNG/SVG | `diagram-image.html` |

## 预览

```bash
~/.anycode/skills/anycode-ppt/run slides/
open slides/index.html   # macOS
```

## 禁止

- 自造 lingqi 企业蓝 / 渐变 / 阴影 / 大圆角
- 删掉主视觉 class 名（validate 会 FAIL）
- 只有标题没有主视觉的空 content 页
- 导出 pptx（本 skill 只交付 HTML）
