term-approval-on = Approval: on (↑↓ · Enter for tools)
term-approval-off = Approval: off
term-brand-version-suffix =  · v{$version}
term-welcome-tagline =  — Chat with your repo; tools run locally.
term-welcome-enter-send =  send one line
term-welcome-history =  history  
term-welcome-alt-arrows =  line cursor  
term-welcome-clear =  clear session
term-welcome-scroll-ws =  scroll Workspace  
term-welcome-multiline =  multiline
term-welcome-shortcuts =  shortcuts  
term-welcome-arrows-move =  move cursor
term-welcome-slash-rest = help  agents  tools  clear  compact  exit  ·  /general-purpose  /explore  /plan
term-welcome-repl =  — line mode (native scroll/copy)
term-welcome-perm-label =   permission: 
term-welcome-perm-sep =   ·  
term-welcome-skip-label =   skip y/n: 
term-welcome-skip-or =  or 
term-welcome-skip-restart =  then restart
term-welcome-dual-back = Welcome back!
term-welcome-dual-brand-suffix =  · v{$version}
term-welcome-dual-model-line =   {$provider} · {$model}
term-welcome-dual-slash-hint =  commands · /help /agents …
term-welcome-dual-tips-title = Tips for getting started
term-welcome-dual-agents-hint = Add AGENTS.md in your project root with instructions for the agent.
term-welcome-dual-shortcuts-hint = Press ? for shortcuts · Shift+Enter for a new line in the prompt.
term-welcome-dual-recent-title = Recent activity
term-welcome-dual-recent-empty = No recent activity yet.
term-welcome-dual-recent-here =  (this folder)
# Compact dual welcome: one line for common actions; short AGENTS hint.
term-welcome-dual-compact-actions = Enter · ? help · Sh+Enter newline · /commands
term-welcome-dual-agents-one = Optional: add AGENTS.md in the project root.
term-welcome-dual-skip-compact = Quick skip: -I  or  ANYCODE_IGNORE_APPROVAL=1
term-footer-scroll-hint = WS ↑{$n}
term-footer-ctx-unknown = ctx —
term-footer-ctx-zero = ctx 0% / {$win} tok
term-footer-ctx-pct = ctx {$pct}% / {$win} tok
term-footer-out-tokens =  · out {$k}k
term-footer-help-hint = ? help
term-help-title = Shortcuts
term-help-line1 = ?  help  ·  Esc close panels / exit when prompt empty · Ctrl+U clear prompt
term-help-line2 = Enter send · Sh+Enter multiline · ↑↓ history
term-help-line3 = PgUp/PgDn scroll main pane (transcript) · Ctrl+Home/End top/bottom · ctrl+o fold tools
term-help-line4 = Mouse: in this fullscreen UI, wheel behavior depends on terminal; isolated viewport: ANYCODE_TERM_ALT_SCREEN=1 or config terminal alternateScreen true · in-app wheel: ANYCODE_TERM_MOUSE=1 · =0 off
term-help-line5 = This screen is fullscreen ratatui (`anycode tui`). Default bare `anycode` on a TTY is the same; use `anycode repl` for the Inline dock layout. Non-TTY pipes fall back to stdio line REPL. Alternate-screen canvas: ANYCODE_TERM_ALT_SCREEN=1 or config terminal alternateScreen true
term-help-line6 = Main buffer only: CLEAR_ON_START defaults off (no Clear(All); preserves scroll position) · CLEAR_ON_START=1 clears on first frame to reduce shell overlap · alternate screen skips that path · SYNC_DRAW=0 disables CSI sync · NO_SCROLLBACK_DUMP=1 disables exit echo (alternate screen only)
term-help-line7 = ^R search history · ^L clear session
term-help-line7b = Ctrl+C while a turn is running: cooperative stop · Ctrl+C when idle: press twice to quit
term-help-line8 = /help /agents /tools /clear /compact /exit · /general-purpose /explore /plan · line mode: anycode repl
term-turn-cooperative-cancelled = Turn stopped (cooperative cancel).
term-help-approval-keys = Approval: ↑ / ↓ · Enter · Esc · y / p / n
term-help-buddy = Buddy: above the prompt (HUD column) when width ≥ 52 cols; falls back beside the prompt if the HUD is too narrow
term-hud-executing = thinking…
term-hud-executing-secs = thinking… ({$s}s)
term-hud-thought-secs = Thought for {$s}s
term-hud-idle = Ready
term-hud-await-approval = waiting for approval…
term-hud-tip-rename = Tip: Name your conversations with /rename to find them easily in /resume later.
term-hud-tip-resume = Tip: Use /resume to continue a saved session (ids from anycode workspace list or prior exit).
term-hud-tip-compact = Tip: Run /compact when the context window gets tight.
term-hud-tip-help = Tip: Press ? for shortcuts; type / to browse slash commands.
term-hud-tip-clear = Tip: Ctrl+L clears the on-screen transcript for a fresh start in this window.
term-hud-tip-scroll = Tip: PgUp/PgDn scrolls the main pane; wheel usually scrolls the host—ANYCODE_TERM_ALT_SCREEN=1 for in-app scroll, or ANYCODE_TERM_MOUSE=1 to capture the wheel on the main buffer
term-agent-gp = general-purpose — general tasks
term-agent-explore = explore         — quick codebase browse
term-agent-plan = plan            — architecture / planning
term-agent-switch = Switch: /general-purpose  ·  /explore  ·  /plan
term-revsearch-nav = Tab / ^R next  ·  Sh+Tab prev  ·  Enter pick
term-err-paste-truncated = Paste truncated to first {$n} characters
term-err-clear-during-task = Cannot clear session while a task is running
term-err-switch-agent-during-task = Cannot switch agent while a task is running
term-err-compact-during-task = Cannot compact context while a task is running
term-err-compact-empty = Nothing to compact (need at least one user message)
term-compact-done = (session context compacted manually)
term-auto-compact-done = (session context auto-compacted)
term-err-compact-failed = Compact failed: {$err}
term-err-autocompact-failed = Auto-compact failed: {$err}
term-agent-switched = Switched agent to `{$id}` (system prompt updated)
term-approval-ui-exited = UI exited; cannot approve
term-approval-cancelled = Approval cancelled
term-expand-hint = (ctrl+o to expand)
term-status-await-approval = awaiting approval
term-status-working = Working…
term-status-working-secs = Working… ({$s}s)
term-status-idle = idle
term-hdr-status = status 
term-hdr-model = model 
term-hdr-agent = agent 
term-hdr-provider = provider 
term-hdr-plan = plan 
term-hdr-permission = permission 
term-hdr-approval = approval 
term-approval-on-short = on
term-approval-off-short = off
term-hdr-key-prefix = key 
term-help-panel-title = Help
term-workspace-title = Workspace
term-workspace-scrolled = Workspace  ↑{$n}
term-dock-approve = Approve tool
term-dock-search = Search
term-dock-prompt = Prompt
term-dock-slash = Slash commands
term-slash-nav = ↑↓ · Enter submit · Tab pick · Shift+Tab prev
term-slash-range = {$s}–{$e} / {$n}
term-approval-question = Do you want to proceed?
term-approval-pending = (pending approval)
term-approval-sp-once =  approve (once)   
term-approval-sp-project =  approve (project)   
term-approval-sp-mid =  / 
term-approval-sp-deny =  deny
term-approval-opt-once = Allow once (this run)
term-approval-opt-project = Always allow for this project
term-approval-opt-deny = Deny
term-approval-hint-arrows = ↑/↓ select · Enter confirm · Esc deny · shortcuts y / p / n
ask-user-ui-exited = UI exited before selection completed
ask-user-empty-selection = Empty selection
ask-user-cancelled = Cancelled
ask-user-title = Choose an answer
ask-user-hint-arrows = ↑/↓ select · Enter confirm · Esc cancel
term-buddy-title = Buddy
term-read-more-paths =    ⎿  … and {$n} more
term-germinating = Thinking…
term-germinating-secs = Thinking… ({$s}s)
# Sub-line while a shell tool has not returned stdout yet
term-tool-running = Running…
term-exit-press-again = Press Ctrl-C again to exit
term-exit-resume-lead = Resume this session with:
term-exit-resume-print = Resume this session with:
term-resume-not-found = No saved session for that id (see ~/.anycode/sessions/).
term-resume-cwd-warn = resume session cwd differs from current directory; messages restored anyway.
term-err-session-during-task = Cannot load session while a task is running
term-session-list-title = Saved sessions (newest first):
term-session-list-empty = (no saved sessions)
term-session-list-err = Could not list sessions:
term-session-bad-uuid = Invalid session id (expected UUID).
term-session-resolve-none = No saved session found for this directory (and no global fallback).
term-session-resumed = Session loaded · {$id}
term-read-ex-a = {$n ->
    [one] Reading {$n} file…
   *[other] Reading {$n} files…
}
term-read-ex-i = {$n ->
    [one] Read {$n} file
   *[other] Read {$n} files
}
term-read-col-a = {$n ->
    [one] Reading {$n} file… {$hint}
   *[other] Reading {$n} files… {$hint}
}
term-read-col-i = {$n ->
    [one] Read {$n} file {$hint}
   *[other] Read {$n} files {$hint}
}
term-block-truncated =    … (truncated)
term-revsearch-no-match = (no match)
term-revsearch-prefix = (reverse-i-search) 
term-col-tools = Tools
term-col-sep =  · 
term-col-search-a = searching for {$n} {$unit}
term-col-search-i = searched for {$n} {$unit}
term-col-read-a = reading {$n} {$unit}
term-col-read-i = read {$n} {$unit}
term-col-list-a = listing {$n} {$unit}
term-col-list-i = listed {$n} {$unit}
term-col-bash-a = running {$n} shell {$unit}
term-col-bash-i = ran {$n} shell {$unit}
term-col-bash-active-secs =  ({$s}s)
term-col-unit-pattern = pattern
term-col-unit-patterns = patterns
term-col-unit-file = file
term-col-unit-files = files
term-col-unit-dir = directory
term-col-unit-dirs = directories
term-col-unit-cmd = command
term-col-unit-cmds = commands
term-md-truncated = … (Markdown output limit reached, truncated)
