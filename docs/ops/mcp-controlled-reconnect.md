# MCP Controlled Reconnect

ADR 007 keeps silent stdio reconnect disabled. The production path is:

1. Diagnose current state with `anycode doctor mcp`.
2. Fail fast when stdio child is dead (`mcp_stdio_dead`).
3. Introduce a future explicit `mcp.auto_restart` flag before reconnecting.
4. On reconnect, re-run `initialize` and `tools/list`.
5. Replace the registry atomically; keep the dead marker if reconnect fails.

This document is the implementation checklist for the future reconnect command.
The current release implements the diagnostic surface and keeps reconnect manual.

