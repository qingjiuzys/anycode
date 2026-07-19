import { describe, expect, it } from "vitest";
import type { TranscriptBlock } from "@/api/types";
import type { ToolStep, TurnReplyItem } from "@/lib/transcriptGrouping";
import {
  resolveActiveReplySegment,
  resolveFinalAssistantIndex,
  toolClusterSegmentActive,
  toolClusterSegmentSettled,
  toolTraceShowThinkingHeader,
  toolTraceStreaming,
} from "@/lib/toolTraceState";

function step(key: string, phase: "start" | "end" | "fail"): ToolStep {
  const call: TranscriptBlock = {
    id: `${key}:call`,
    block_type: "tool_call",
    at: "2026-01-01T00:00:00Z",
    title: "Skill started",
    body: "",
    meta: { tool_key: key },
  };
  if (phase === "start") {
    return { key, call };
  }
  const result: TranscriptBlock = {
    id: `${key}:result`,
    block_type: "tool_result",
    at: "2026-01-01T00:00:01Z",
    title: phase === "fail" ? "Skill failed" : "Skill finished",
    body: phase === "fail" ? "error" : "ok",
    meta: { tool_key: key },
  };
  return { key, call, result };
}

describe("toolTraceStreaming", () => {
  it("stays live while a tool is running", () => {
    expect(toolTraceStreaming([step("1", "start")], 1, true)).toBe(true);
  });

  it("stops streaming after all tools finish even if session is running", () => {
    expect(toolTraceStreaming([step("1", "fail")], 2, true)).toBe(false);
  });

  it("streams while waiting for the first tool", () => {
    expect(toolTraceStreaming([], 1, true)).toBe(true);
  });

  it("does not stream when the segment is inactive", () => {
    expect(toolTraceStreaming([], 1, false)).toBe(false);
  });

  it("stops spinning for unpaired tools when the segment is inactive", () => {
    expect(toolTraceStreaming([step("1", "start")], 0, false)).toBe(false);
  });
});

describe("toolTraceShowThinkingHeader", () => {
  it("shows thinking only before tool steps exist", () => {
    expect(toolTraceShowThinkingHeader([], 2, true)).toBe(true);
    expect(toolTraceShowThinkingHeader([step("1", "start")], 2, true)).toBe(false);
    expect(toolTraceShowThinkingHeader([step("1", "fail")], 2, true)).toBe(false);
  });
});

describe("toolClusterSegmentActive", () => {
  it("deactivates completed clusters while the session keeps running", () => {
    expect(toolClusterSegmentActive([step("1", "fail")], true, true, false)).toBe(
      false,
    );
  });

  it("stays active for running tools or pre-tool waiting", () => {
    expect(toolClusterSegmentActive([step("1", "start")], true, true, false)).toBe(
      true,
    );
    expect(toolClusterSegmentActive([], true, true, false)).toBe(true);
  });

  it("stops when a later assistant reply has landed", () => {
    expect(toolClusterSegmentActive([step("1", "fail")], true, true, true)).toBe(
      false,
    );
  });
});

describe("toolClusterSegmentSettled", () => {
  it("detects a substantive assistant reply after the cluster", () => {
    expect(
      toolClusterSegmentSettled(
        [
          {
            kind: "tool_cluster",
            id: "c1",
            steps: [],
            processMessageCount: 0,
            processSnippets: [],
          },
          {
            kind: "block",
            block: {
              id: "a1",
              block_type: "assistant_message",
              at: "2026-01-01T00:00:00Z",
              title: "Assistant",
              body: "done",
            },
          },
        ],
        0,
      ),
    ).toBe(true);
  });

  it("settles when a later tool round starts", () => {
    expect(
      toolClusterSegmentSettled(
        [
          {
            kind: "tool_cluster",
            id: "c1",
            steps: [step("1", "end")],
            processMessageCount: 0,
            processSnippets: [],
          },
          {
            kind: "tool_cluster",
            id: "c2",
            steps: [step("2", "start")],
            processMessageCount: 0,
            processSnippets: [],
          },
        ],
        0,
      ),
    ).toBe(true);
  });
});

describe("resolveActiveReplySegment", () => {
  const narration = (id: string, body: string, live = false): TurnReplyItem => ({
    kind: "block",
    block: {
      id,
      block_type: "system_notice",
      at: "2026-01-01T00:00:00Z",
      title: "Status",
      body,
      meta: { source: "intermediate_assistant", ...(live ? { live: true } : {}) },
    },
  });

  it("returns -1 when the turn is not running", () => {
    expect(
      resolveActiveReplySegment([narration("n1", "hi")], {
        isLast: true,
        isRunning: false,
      }),
    ).toBe(-1);
  });

  it("always expands the tip segment while the turn is running", () => {
    const items: TurnReplyItem[] = [
      narration("n1", "planning"),
      {
        kind: "tool_cluster",
        id: "c1",
        steps: [step("1", "start")],
        processMessageCount: 0,
        processSnippets: [],
      },
    ];
    expect(resolveActiveReplySegment(items, { isLast: true, isRunning: true })).toBe(1);
  });

  it("moves the open tip to the newest narration", () => {
    const items: TurnReplyItem[] = [
      {
        kind: "tool_cluster",
        id: "c1",
        steps: [step("1", "end")],
        processMessageCount: 0,
        processSnippets: [],
      },
      narration("n2", "next step"),
    ];
    expect(resolveActiveReplySegment(items, { isLast: true, isRunning: true })).toBe(1);
  });
});

describe("resolveFinalAssistantIndex", () => {
  it("finds the last non-narration assistant", () => {
    const items: TurnReplyItem[] = [
      {
        kind: "block",
        block: {
          id: "a0",
          block_type: "assistant_message",
          at: "t",
          title: "A",
          body: "mid",
          meta: { narration: true },
        },
      },
      {
        kind: "block",
        block: {
          id: "a1",
          block_type: "assistant_message",
          at: "t",
          title: "A",
          body: "final",
        },
      },
    ];
    expect(resolveFinalAssistantIndex(items)).toBe(1);
  });
});
