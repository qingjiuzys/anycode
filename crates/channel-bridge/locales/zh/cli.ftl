cli-short = anycode-daemon - 无头调度服务
cli-long = anycode-daemon 运行常驻服务：`scheduler`（cron/自动化）。模型配置请用 Workbench /setup 或编辑 ~/.anycode/config.json。
flag-debug = 启用调试日志
flag-config = 配置文件（JSON）路径
flag-ignore-approval = 本进程跳过工具 y/n 审批（临时；deny 等策略仍生效）。环境变量 ANYCODE_IGNORE_APPROVAL=1 等价。
cmd-scheduler-about = 内置调度器：执行 ~/.anycode/tasks/orchestration.json 中 CronCreate 注册的任务
cmd-scheduler-directory = 每次触发 agent 任务时的工作目录
cmd-scheduler-reload-secs = 重新读取 orchestration.json，并限制两次唤醒之间的休眠上限（秒）
