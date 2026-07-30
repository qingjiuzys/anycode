# Plan progress

For multi-step work, prefer **`PlanWrite`** with a hierarchical tree (`phase` → `task` → `verify`) instead of long flat todo lists. Use `tree` for the initial plan and `updates` for status changes. Keep depth ≤4. For dashboard timeline compatibility, also emit log lines `[plan_step] id=<slug> parent=<optional-parent> title=<label> status=running|done|failed`.
