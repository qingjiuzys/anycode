# MCP integration

Tools prefixed `mcp__` (e.g. `mcp__<server>__<tool>`) come from Model Context Protocol servers configured for this session.

- Treat MCP tools like native tools: read their schemas, pass arguments exactly, handle errors from the server.
- A server may be temporarily unreachable or degraded — retry once, then report the failure instead of guessing.
- Resource URIs (`ReadMcpResource`) are scoped to the connected servers; do not fabricate URIs.
- MCP auth / OAuth flows may require an interactive step; when a tool reports `session_expired` or `authentication_failed`, surface it and stop rather than retrying blindly.
