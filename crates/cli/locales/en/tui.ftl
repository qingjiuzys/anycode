tui-approval-on = Approval: on (y/n for tools)
tui-approval-off = Approval: off
tui-welcome-tagline =  — Chat with your repo; tools run locally.
tui-welcome-enter-send =  send one line
tui-welcome-history =  history  
tui-welcome-alt-arrows =  line cursor  
tui-welcome-clear =  clear session
tui-welcome-scroll-ws =  scroll Workspace  
tui-welcome-multiline =  multiline
tui-welcome-shortcuts =  shortcuts  
tui-welcome-arrows-move =  move cursor
tui-welcome-slash-rest = help  agents  tools  clear  compact  exit  ·  /general-purpose  /explore  /plan
tui-welcome-repl =  — line mode (native scroll/copy)
tui-welcome-perm-label =   permission: 
tui-welcome-perm-sep =   ·  
tui-welcome-skip-label =   skip y/n: 
tui-welcome-skip-or =  or 
tui-welcome-skip-restart =  then restart
tui-help-title = Shortcuts
tui-help-line1 = ?  help  ·  Esc clear/exit
tui-help-line2 = Enter send · Sh+Enter multiline · ↑↓ history
tui-help-line3 = PgUp/PgDn scroll Workspace · ctrl+o fold/unfold tool blocks
tui-help-line4 = Mouse reporting on by default (wheel scrolls Workspace); ANYCODE_TUI_MOUSE=0 if drag-select breaks
tui-help-line5 = Alt screen echoes session on exit; main buffer: ANYCODE_TUI_ALT_SCREEN=0 or CLAUDE_CODE_NO_FLICKER=0
tui-help-line6 = Disable echo: ANYCODE_TUI_NO_SCROLLBACK_DUMP=1 (alt screen only)
tui-help-line7 = ^R search history · ^L clear session
tui-help-line8 = /help /agents /tools /clear /compact /exit · /general-purpose /explore /plan · line mode: anycode repl
tui-help-approval-keys = Approval: y / n / Esc
tui-help-buddy = Buddy: shown in Dock when width ≥ 52 cols
tui-agent-gp = general-purpose — general tasks
tui-agent-explore = explore         — quick codebase browse
tui-agent-plan = plan            — architecture / planning
tui-agent-switch = Switch: /general-purpose  ·  /explore  ·  /plan
tui-revsearch-nav = Tab / ^R next  ·  Sh+Tab prev  ·  Enter pick
tui-err-paste-truncated = Paste truncated to first {$n} characters
tui-err-clear-during-task = Cannot clear session while a task is running
tui-err-switch-agent-during-task = Cannot switch agent while a task is running
tui-err-compact-during-task = Cannot compact context while a task is running
tui-err-compact-empty = Nothing to compact (need at least one user message)
tui-compact-done = (session context compacted manually)
tui-auto-compact-done = (session context auto-compacted)
tui-err-compact-failed = Compact failed: {$err}
tui-err-autocompact-failed = Auto-compact failed: {$err}
tui-agent-switched = Switched agent to `{$id}` (system prompt updated)
tui-approval-tui-exited = TUI exited; cannot approve
tui-approval-cancelled = Approval cancelled
tui-expand-hint = (ctrl+o to expand)
tui-status-await-approval = awaiting approval
tui-status-working = working
tui-status-working-secs = working ({$s}s)
tui-status-idle = idle
tui-hdr-status = status 
tui-hdr-model = model 
tui-hdr-agent = agent 
tui-hdr-provider = provider 
tui-hdr-plan = plan 
tui-hdr-permission = permission 
tui-hdr-approval = approval 
tui-approval-on-short = on
tui-approval-off-short = off
tui-hdr-key-prefix = key 
tui-help-panel-title = Help
tui-workspace-title = Workspace
tui-workspace-scrolled = Workspace  ↑{$n}
tui-dock-approve = Approve tool
tui-dock-search = Search
tui-dock-prompt = Prompt
tui-approval-question = Do you want to proceed?
tui-approval-pending = (pending approval)
tui-approval-sp-approve =  approve   
tui-approval-sp-mid =  / 
tui-approval-sp-deny =  deny
tui-buddy-title = Buddy
tui-read-more-paths =    ⎿  … and {$n} more
tui-germinating = Germinating…
tui-germinating-secs = Germinating… ({$s}s)
tui-read-ex-a = {$n ->
    [one] Reading {$n} file…
   *[other] Reading {$n} files…
}
tui-read-ex-i = {$n ->
    [one] Read {$n} file
   *[other] Read {$n} files
}
tui-read-col-a = {$n ->
    [one] Reading {$n} file… {$hint}
   *[other] Reading {$n} files… {$hint}
}
tui-read-col-i = {$n ->
    [one] Read {$n} file {$hint}
   *[other] Read {$n} files {$hint}
}
tui-block-truncated =    … (truncated)
tui-revsearch-no-match = (no match)
tui-revsearch-prefix = (reverse-i-search) 
tui-col-tools = Tools
tui-col-sep =  · 
tui-col-search-a = searching for {$n} {$unit}
tui-col-search-i = searched for {$n} {$unit}
tui-col-read-a = reading {$n} {$unit}
tui-col-read-i = read {$n} {$unit}
tui-col-list-a = listing {$n} {$unit}
tui-col-list-i = listed {$n} {$unit}
tui-col-bash-a = running {$n} shell {$unit}
tui-col-bash-i = ran {$n} shell {$unit}
tui-col-bash-active-secs =  ({$s}s)
tui-col-unit-pattern = pattern
tui-col-unit-patterns = patterns
tui-col-unit-file = file
tui-col-unit-files = files
tui-col-unit-dir = directory
tui-col-unit-dirs = directories
tui-col-unit-cmd = command
tui-col-unit-cmds = commands
tui-md-truncated = … (Markdown output limit reached, truncated)
