tui-approval-on = Approval: on (↑↓ · Enter for tools)
tui-approval-off = Approval: off
tui-brand-version-suffix =  · v{$version}
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
tui-welcome-dual-back = Welcome back!
tui-welcome-dual-brand-suffix =  · v{$version}
tui-welcome-dual-model-line =   {$provider} · {$model}
tui-welcome-dual-slash-hint =  commands · /help /agents …
tui-welcome-dual-tips-title = Tips for getting started
tui-welcome-dual-agents-hint = Add AGENTS.md in your project root with instructions for the agent.
tui-welcome-dual-shortcuts-hint = Press ? for shortcuts · Shift+Enter for a new line in the prompt.
tui-welcome-dual-recent-title = Recent activity
tui-welcome-dual-recent-empty = No recent activity yet.
tui-welcome-dual-recent-here =  (this folder)
# Compact dual welcome: one line for common actions; short AGENTS hint.
tui-welcome-dual-compact-actions = Enter · ? help · Sh+Enter newline · /commands
tui-welcome-dual-agents-one = Optional: add AGENTS.md in the project root.
tui-welcome-dual-skip-compact = Quick skip: -I  or  ANYCODE_IGNORE_APPROVAL=1
tui-footer-scroll-hint = WS ↑{$n}
tui-footer-ctx-unknown = ctx —
tui-footer-ctx-zero = ctx 0% / {$win} tok
tui-footer-ctx-pct = ctx {$pct}% / {$win} tok
tui-footer-out-tokens =  · out {$k}k
tui-footer-help-hint = ? help
tui-help-title = Shortcuts
tui-help-line1 = ?  help  ·  Esc close panels / exit when prompt empty · Ctrl+U clear prompt
tui-help-line2 = Enter send · Sh+Enter multiline · ↑↓ history
tui-help-line3 = PgUp/PgDn scroll main pane (transcript) · Ctrl+Home/End top/bottom · ctrl+o fold tools
tui-help-line4 = Mouse: in this fullscreen UI, wheel behavior depends on terminal; isolated viewport: ANYCODE_TUI_ALT_SCREEN=1 or config tui alternateScreen true · in-app wheel: ANYCODE_TUI_MOUSE=1 · =0 off
tui-help-line5 = This screen is fullscreen ratatui (`anycode tui`). Default CLI entry is line REPL (`anycode` / `anycode repl`). Alternate-screen canvas: ANYCODE_TUI_ALT_SCREEN=1 or config tui alternateScreen true
tui-help-line6 = Main buffer only: CLEAR_ON_START defaults off (no Clear(All); preserves scroll position) · CLEAR_ON_START=1 clears on first frame to reduce shell overlap · alternate screen skips that path · SYNC_DRAW=0 disables CSI sync · NO_SCROLLBACK_DUMP=1 disables exit echo (alternate screen only)
tui-help-line7 = ^R search history · ^L clear session
tui-help-line8 = /help /agents /tools /clear /compact /exit · /general-purpose /explore /plan · line mode: anycode repl
tui-help-approval-keys = Approval: ↑ / ↓ · Enter · Esc · y / p / n
tui-help-buddy = Buddy: above the prompt (HUD column) when width ≥ 52 cols; falls back beside the prompt if the HUD is too narrow
tui-hud-executing = thinking…
tui-hud-executing-secs = thinking… ({$s}s)
tui-hud-thought-secs = Thought for {$s}s
tui-hud-idle = Ready
tui-hud-await-approval = waiting for approval…
tui-hud-tip-rename = Tip: Name your conversations with /rename to find them easily in /resume later.
tui-hud-tip-resume = Tip: Use /resume to continue a saved session (ids from anycode workspace list or prior exit).
tui-hud-tip-compact = Tip: Run /compact when the context window gets tight.
tui-hud-tip-help = Tip: Press ? for shortcuts; type / to browse slash commands.
tui-hud-tip-clear = Tip: Ctrl+L clears the on-screen transcript for a fresh start in this window.
tui-hud-tip-scroll = Tip: PgUp/PgDn scrolls the main pane; wheel usually scrolls the host—ANYCODE_TUI_ALT_SCREEN=1 for in-app scroll, or ANYCODE_TUI_MOUSE=1 to capture the wheel on the main buffer
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
tui-dock-slash = Slash commands
tui-slash-nav = ↑↓ · Enter submit · Tab pick · Shift+Tab prev
tui-slash-range = {$s}–{$e} / {$n}
tui-approval-question = Do you want to proceed?
tui-approval-pending = (pending approval)
tui-approval-sp-once =  approve (once)   
tui-approval-sp-project =  approve (project)   
tui-approval-sp-mid =  / 
tui-approval-sp-deny =  deny
tui-approval-opt-once = Allow once (this run)
tui-approval-opt-project = Always allow for this project
tui-approval-opt-deny = Deny
tui-approval-hint-arrows = ↑/↓ select · Enter confirm · Esc deny · shortcuts y / p / n
tui-buddy-title = Buddy
tui-read-more-paths =    ⎿  … and {$n} more
tui-germinating = Germinating…
tui-germinating-secs = Germinating… ({$s}s)
# Sub-line while a shell tool has not returned stdout yet
tui-tool-running = Running…
tui-exit-press-again = Press Ctrl-C again to exit
tui-exit-resume-lead = Resume this session with:
tui-exit-resume-print = Resume this session with:
tui-resume-not-found = No saved session for that id (see ~/.anycode/tui-sessions/).
tui-resume-cwd-warn = resume session cwd differs from current directory; messages restored anyway.
tui-err-session-during-task = Cannot load session while a task is running
tui-session-list-title = Saved sessions (newest first):
tui-session-list-empty = (no saved sessions)
tui-session-list-err = Could not list sessions:
tui-session-bad-uuid = Invalid session id (expected UUID).
tui-session-resolve-none = No saved session found for this directory (and no global fallback).
tui-session-resumed = Session loaded · {$id}
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
