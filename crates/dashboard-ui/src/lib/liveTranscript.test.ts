import { describe, expect, it } from "vitest";
import { applyChatStreamEvent, hasLiveStreamActivity, mergeTranscriptBlocks } from "@/lib/liveTranscript";
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
        id: "llm-start:1",
        block_type: "system_notice",
        at: "2026-01-01T00:00:00Z",
        title: "Thinking",
        body: "",
        meta: { source: "llm_start", live: true, turn: 1 },
      },
    });
    expect(hasLiveStreamActivity(blocks)).toBe(true);
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

  it("dedupes live tool blocks against snapshot tool_key", () => {
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
});
