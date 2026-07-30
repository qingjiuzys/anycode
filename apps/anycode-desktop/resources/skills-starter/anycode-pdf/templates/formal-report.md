# 项目方案报告 · anyCode 办公技能升级

## Summary

本报告说明 anyCode 如何将 Kimi 办公技能蒸馏为可验证的 anycode-* 技能包，并在 DeepSeek 模型下稳定交付。

## Background

- 用户需要 ppt / docx / xlsx / pdf 四类办公产物
- DeepSeek 对**编号步骤 + 模板复制**遵循度高
- 终稿必须经 `run` 脚本 validate，避免模型手改二进制

## Approach

| 层级 | 做法 |
|------|------|
| 写作 | 复制 templates，只改文案 |
| 预览 | FDE HTML preview |
| 终稿 | docx / xlsx / pdf / HTML slides |
| 验证 | validate + xlsx recheck |

## Metrics

| 维度 | 目标 | 当前 |
|------|------|------|
| 模板命中率 | ≥95% | 93% |
| 占位符拦截 | 100% | 100% |
| 默认安装覆盖 | 4 skills | 4 |

## References

[1] GB/T 7714—2015 信息与文献 参考文献著录规则 [S].  
[2] anyCode FDE Editorial Contract, docs/design/fde-editorial-contract.md [EB/OL].

Decision: 默认安装 anycode-ppt、anycode-docx、anycode-xlsx、anycode-pdf 四技能。

Action: 平台组 2026-08-01 前完成 bootstrap 自动安装回归。
