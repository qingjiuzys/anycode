---
title: 模型子命令
description: anycode model list、status、set 与交互式编辑。
summary: z.ai 静态目录与其它提供商在 config 中直配的区别。
read_when:
  - 要在终端里改默认模型或路由相关模型。
---

# 模型子命令

```bash
./target/release/anycode model list --plain
./target/release/anycode model status
./target/release/anycode model set glm-5
```

**`model list`** 当前主要为 **z.ai** 静态目录；使用 **Anthropic** 时在 **`config.json`** 中直接设置 **`provider`** 与 **`model`**。

无子命令的 **`anycode model`** 可交互编辑全局默认值与 **`routing.agents`**（以当前版本行为为准）。

均遵守 **`-c/--config`**。

## 相关

- [模型与端点](./models)  
- [路由](./routing)  

English: [Model commands](/guide/cli-model).
