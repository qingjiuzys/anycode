import { describe, expect, it } from "vitest";
import { applyChatStreamEvent, hasLiveStreamActivity, hasTurnStreamActivity, mergeTranscriptBlocks, resolveTranscriptBlocks } from "@/lib/liveTranscript";
import type { TranscriptBlock } from "@/api/types";

describe("applyChatStreamEvent", () => {
  it("appends assistant deltas for the same turn", () => {
    let blocks: TranscriptBlock[] = [];
    blocks = applyChatStreamEvent(blocks, {
      session_id: "s1",
      project_id: "p1",
      kind: "assistant_delta",
      turn: 2,
      text: "Hel",
      at: "2026-01-01T00:00:01Z",
    });
    blocks = applyChatStreamEvent(blocks, {
      session_id: "s1",
      project_id: "p1",
      kind: "assistant_delta",
      turn: 2,
      text: "lo",
      at: "2026-01-01T00:00:02Z",
      block: {
        id: "assistant-live:2",
        block_type: "assistant_message",
        at: "2026-01-01T00:00:02Z",
        title: "Assistant (turn 2)",
        body: "Hello",
        meta: { live: true, turn: 2 },
      },
    });
    expect(blocks).toHaveLength(1);
    expect(blocks[0]?.body).toBe("Hello");
  });

  it("upserts tool start blocks by id", () => {
    const evt = {
      session_id: "s1",
      project_id: "p1",
      kind: "tool_start",
      at: "2026-01-01T00:00:00Z",
      block: {
        id: "t1",
        block_type: "tool_call",
        at: "2026-01-01T00:00:00Z",
        title: "Bash",
        body: "ls",
        meta: { tool_key: "1:1", phase: "start" },
        collapsible: true,
        default_collapsed: true,
      },
    };
    const blocks = applyChatStreamEvent([], evt);
    expect(blocks[0]?.block_type).toBe("tool_call");
  });

  it("tracks llm_start as live activity", () => {
    const blocks = applyChatStreamEvent([], {
      session_id: "s1",
      project_id: "p1",
      kind: "llm_start",
      turn: 1,
      at: "2026-01-01T00:00:00Z",
      block: {
        id: "llm-start:u2:1",
        block_type: "system_notice",
        at: "2026-01-01T00:00:00Z",
        title: "Thinking",
        body: "",
        meta: { source: "llm_start", live: true, turn: 1, user_turn_id: "2" },
      },
      payload: { user_turn_id: 2 },
    });
    expect(hasLiveStreamActivity(blocks)).toBe(true);
  });

  it("updates running tool elapsed via tool_progress", () => {
    const start = applyChatStreamEvent([], {
      session_id: "s1",
      project_id: "p1",
      kind: "tool_start",
      at: "2026-01-01T00:00:00Z",
      block: {
        id: "tool-live:u3:2:1:call",
        block_type: "tool_call",
        at: "2026-01-01T00:00:00Z",
        title: "Bash started",
        body: "npm test",
        meta: { tool_key: "u3:2:1", phase: "start" },
      },
    });
    const progressed = applyChatStreamEvent(start, {
      session_id: "s1",
      project_id: "p1",
      kind: "tool_progress",
      at: "2026-01-01T00:00:02Z",
      block: {
        id: "tool-live:u3:2:1:call",
        block_type: "tool_call",
        at: "2026-01-01T00:00:02Z",
        title: "Bash started",
        body: "",
        meta: { tool_key: "u3:2:1", phase: "running", duration_ms: "2000" },
      },
    });
    expect(progressed).toHaveLength(1);
    expect(progressed[0]?.meta?.phase).toBe("running");
    expect(progressed[0]?.meta?.duration_ms).toBe("2000");
  });
});

describe("hasTurnStreamActivity", () => {
  it("treats active execution-log tool as activity", () => {
    expect(hasTurnStreamActivity([], "Bash", [])).toBe(true);
  });

  it("treats running tool_call in replies as activity", () => {
    expect(
      hasTurnStreamActivity(
        [],
        null,
        [
          {
            id: "t1",
            block_type: "tool_call",
            at: "t",
            title: "Bash started",
            body: "ls",
            meta: { tool_key: "2:1" },
          },
        ],
      ),
    ).toBe(true);
  });
});

describe("mergeTranscriptBlocks", () => {
  it("prefers longer live assistant bodies", () => {
    const snapshot = [
      {
        id: "assistant-live:1",
        block_type: "assistant_message",
        at: "t0",
        title: "A",
        body: "Hel",
      },
    ];
    const live = [
      {
        id: "assistant-live:1",
        block_type: "assistant_message",
        at: "t1",
        title: "A",
        body: "Hello world",
        meta: { live: true },
      },
    ];
    const merged = mergeTranscriptBlocks(snapshot, live);
    expect(merged[0]?.body).toBe("Hello world");
  });

  it("appends live tool blocks after snapshot blocks", () => {
    const snapshot = [
      {
        id: "u1",
        block_type: "user_message",
        at: "2026-01-01T00:00:00Z",
        title: "User",
        body: "hi",
      },
    ];
    const live = [
      {
        id: "tool-live:1:1:call",
        block_type: "tool_call",
        at: "2026-01-01T00:00:01Z",
        title: "Bash",
        body: "ls",
        meta: { tool_key: "1:1", phase: "start" },
      },
    ];
    const merged = mergeTranscriptBlocks(snapshot, live);
    expect(merged.map((b) => b.id)).toEqual(["u1", "tool-live:1:1:call"]);
  });

  it("does not prepend live blocks when timestamps are missing", () => {
    const snapshot = [
      {
        id: "u-old",
        block_type: "user_message",
        at: "2026-01-01T00:00:00Z",
        title: "User",
        body: "older",
      },
      {
        id: "a-old",
        block_type: "assistant_message",
        at: "2026-01-01T00:00:01Z",
        title: "Assistant",
        body: "done",
      },
      {
        id: "u-new",
        block_type: "user_message",
        at: "2026-01-01T00:01:00Z",
        title: "User",
        body: "latest",
      },
    ];
    const live = [
      {
        id: "tool-live:2:1:call",
        block_type: "tool_call",
        at: "",
        title: "Bash",
        body: "ls",
        meta: { tool_key: "2:1", phase: "start" },
      },
      {
        id: "assistant-live:2",
        block_type: "assistant_message",
        at: "",
        title: "Assistant (turn 2)",
        body: "streaming",
        meta: { live: true, turn: 2 },
      },
    ];
    const merged = mergeTranscriptBlocks(snapshot, live);
    expect(merged.map((b) => b.id)).toEqual([
      "u-old",
      "a-old",
      "u-new",
      "tool-live:2:1:call",
      "assistant-live:2",
    ]);
  });

  it("dedupes live tool blocks against snapshot tool_key in active tail", () => {
    const snapshot = [
      {
        id: "u1",
        block_type: "user_message",
        at: "2026-01-01T00:00:00Z",
        title: "User",
        body: "hi",
      },
      {
        id: "evt-tool-call",
        block_type: "tool_call",
        at: "2026-01-01T00:00:01Z",
        title: "Bash started",
        body: "ls",
        meta: { tool_key: "2:1", phase: "start" },
      },
    ];
    const live = [
      {
        id: "tool-live:2:1:call",
        block_type: "tool_call",
        at: "2026-01-01T00:00:01Z",
        title: "Bash started",
        body: "ls -la",
        meta: { tool_key: "2:1", phase: "start", live: true },
      },
    ];
    const merged = mergeTranscriptBlocks(snapshot, live);
    expect(merged).toHaveLength(2);
    expect(merged[1]?.id).toBe("evt-tool-call");
    expect(merged[1]?.body).toBe("ls -la");
  });

  it("does not merge live blocks into historical turns with colliding tool_key or turn", () => {
    const snapshot = [
      {
        id: "u-old",
        block_type: "user_message",
        at: "2026-01-01T00:00:00Z",
        title: "User",
        body: "old question",
      },
      {
        id: "tool-old",
        block_type: "tool_call",
        at: "2026-01-01T00:00:01Z",
        title: "Glob started",
        body: "old",
        meta: { tool_key: "1:1", phase: "start" },
      },
      {
        id: "a-old",
        block_type: "assistant_message",
        at: "2026-01-01T00:00:02Z",
        title: "Assistant",
        body: "old answer",
        meta: { turn: 1 },
      },
      {
        id: "u-new",
        block_type: "user_message",
        at: "2026-01-03T00:00:00Z",
        title: "User",
        body: "new question",
      },
    ];
    const live = [
      {
        id: "tool-live:1:1:call",
        block_type: "tool_call",
        at: "2026-01-03T00:00:01Z",
        title: "Glob started",
        body: "new glob",
        meta: { tool_key: "1:1", phase: "start", live: true },
      },
      {
        id: "assistant-live:1",
        block_type: "assistant_message",
        at: "2026-01-03T00:00:02Z",
        title: "Assistant (turn 1)",
        body: "streaming answer",
        meta: { live: true, turn: 1 },
      },
    ];
    const merged = mergeTranscriptBlocks(snapshot, live);
    expect(merged.map((b) => b.id)).toEqual([
      "u-old",
      "tool-old",
      "a-old",
      "u-new",
      "tool-live:1:1:call",
      "assistant-live:1",
    ]);
    expect(merged[4]?.body).toBe("new glob");
    expect(merged[5]?.body).toBe("streaming answer");
  });
});

describe("resolveTranscriptBlocks", () => {
  it("uses snapshot only when not streaming", () => {
    const snapshot = [
      {
        id: "u1",
        block_type: "user_message",
        at: "t0",
        title: "User",
        body: "hi",
      },
      {
        id: "a1",
        block_type: "assistant_message",
        at: "t1",
        title: "Assistant",
        body: "answer",
      },
    ];
    const live = [
      {
        id: "tool-live:u2:1:1:call",
        block_type: "tool_call",
        at: "t2",
        title: "Bash",
        body: "ls",
        meta: { tool_key: "u2:1:1" },
      },
    ];
    expect(resolveTranscriptBlocks(snapshot, live, false)).toEqual(snapshot);
  });

  it("hydrates only through last user message when streaming", () => {
    const snapshot = [
      {
        id: "u-old",
        block_type: "user_message",
        at: "t0",
        title: "User",
        body: "old",
      },
      {
        id: "a-old",
        block_type: "assistant_message",
        at: "t1",
        title: "Assistant",
        body: "old answer",
      },
      {
        id: "u-new",
        block_type: "user_message",
        at: "t2",
        title: "User",
        body: "new",
      },
      {
        id: "stale-tail",
        block_type: "tool_call",
        at: "t3",
        title: "Stale",
        body: "from REST race",
        meta: { tool_key: "u3:1:1" },
      },
    ];
    const live = [
      {
        id: "tool-live:u3:1:1:call",
        block_type: "tool_call",
        at: "t4",
        title: "Bash",
        body: "npm test",
        meta: { tool_key: "u3:1:1", live: true },
      },
    ];
    const resolved = resolveTranscriptBlocks(snapshot, live, true);
    expect(resolved.map((b) => b.id)).toEqual(["u-old", "a-old", "u-new", "tool-live:u3:1:1:call"]);
    expect(resolved[3]?.body).toBe("npm test");
  });
});
