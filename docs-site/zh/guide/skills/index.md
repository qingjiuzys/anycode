# 官方 Skills 目录

可在工作台 **Agents → Official catalog** 中安装的技能精选列表。

| ID | 分类 | 来源 |
|----|------|------|
| web-research | data | anthropics/skills:skills/web-research |
| code-review | quality | anthropics/skills:skills/code-review |
| git-workflow | quality | anthropics/skills:skills/git-workflow |
| office-docx | business | anthropics/skills:skills/docx |
| readonly-db | data | anthropics/skills:skills/readonly-db |

## 安装

1. 打开仪表盘 `/agents`。
2. 切换到 **Official catalog** 标签。
3. 在技能卡片上点击 **Install**。

技能会复制到本地 skills 目录，并出现在 **Installed** 列表。

## 同步脚本

从 `skill_market.rs` 重新生成本页：

```bash
node scripts/sync-skill-catalog-docs.mjs
```
