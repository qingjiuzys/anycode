cli-short = anycode-daemon - headless scheduler service
cli-long = anycode-daemon runs the long-lived `scheduler` service (cron/automations). Configure models via Workbench /setup or ~/.anycode/config.json.
flag-debug = Enable debug logging
flag-config = Path to config file (JSON)
flag-ignore-approval = Skip tool y/n approvals in this process (temporary; deny policies still apply). ANYCODE_IGNORE_APPROVAL=1 is equivalent.
cmd-scheduler-about = Built-in scheduler: run CronCreate jobs from ~/.anycode/tasks/orchestration.json
cmd-scheduler-directory = Working directory for each triggered agent task
cmd-scheduler-reload-secs = Re-read orchestration.json and cap sleep between ticks (seconds)
