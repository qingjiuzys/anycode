# 内置运行时（Python / Node）

anyCode 的许多 skill（office-pptx、spreadsheet-delivery、pdf、md-to-pdf 等）依赖 `python3` 与 `node`。为保证开箱即用，anyCode 采用与主流 AI 工具一致的做法——**自带受管运行时**，不依赖用户环境：

- **Claude Code** 原生安装器自带运行时（无需用户安装 Node.js）；Anthropic 官方 skills 依赖 uv 管理的 Python。
- **云端 Agent**（Devin / E2B 等）在沙箱中预装完整运行时。
- anyCode 作为本地桌面产品，将受管运行时统一放在 `~/.anycode/runtimes/`：

```text
~/.anycode/runtimes/
├── python/bin/python3   # uv 管理的 CPython（python-build-standalone）
├── node/bin/node        # Node.js 官方发行版（校验和验证）
└── bin/uv               # uv（Python 版本与依赖管理）
```

## 工作机制

1. 运行时初始化时（`initialize_runtime`），anyCode 把上述 `bin` 目录**前置注入进程 PATH**——所有工具与 skill 子进程（Bash、`Skill run`、cron 任务）优先使用受管解释器。
2. 若受管运行时缺失且能找到 `scripts/provision-runtimes.sh`，会在**后台自动补给**（不阻塞启动）；失败时静默回退系统解释器。
3. 安装脚本（`scripts/install.sh`）在安装时即执行一次补给。

## 手动补给 / 排障

```bash
# 手动执行补给（幂等；已存在则秒退）
bash scripts/provision-runtimes.sh

# 自定义版本
ANYCODE_PYTHON_VERSION=3.11 ANYCODE_NODE_VERSION=v22.18.0 bash scripts/provision-runtimes.sh

# 自定义安装目录
ANYCODE_RUNTIMES_DIR=/opt/anycode-runtimes bash scripts/provision-runtimes.sh
```

离线环境：脚本会尽量链接系统 `python3` / `node` 作为兜底；skill 依赖包（如 `python-pptx`）仍需在首次使用时联网安装（`pip install` 或 `uv run --with ...`）。
