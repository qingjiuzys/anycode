import { describe, expect, it } from "vitest";
import type { TranscriptBlock } from "@/api/types";
import { groupTurnReplies } from "@/lib/transcriptGrouping";
import {
  dedupeNarrationWithProgress,
  groupTurnRepliesByPhase,
  lastToolClusterRef,
  phaseVisibilityPlan,
  type TranscriptTurnLike,
} from "@/lib/phaseGrouping";

function block(id: string, blockType: string, extra?: Partial<TranscriptBlock>): TranscriptBlock {
  return {
    id,
    block_type: blockType,
    at: "2026-01-01T00:00:00Z",
    title: "",
    body: "",
    ...extra,
  };
}

function toolPair(id: string, key: string): TranscriptBlock[] {
  return [
    block(`${id}:call`, "tool_call", { meta: { tool_key: key } }),
    block(`${id}:result`, "tool_result", { meta: { tool_key: key } }),
  ];
}

describe("phaseGrouping", () => {
  it("attaches tool cluster to preceding progress segment", () => {
    const items = groupTurnReplies([
      block("p1", "progress_update", {
        body: "checking tests",
        meta: { phase: "execute", summary: "checking tests" },
      }),
      block("t1", "tool_call", { meta: { tool_key: "1:1" } }),
      block("t2", "tool_result", { meta: { tool_key: "1:1" } }),
    ]);
    const segments = groupTurnRepliesByPhase(items);
    expect(segments).toHaveLength(1);
    expect(segments[0]?.toolCluster?.steps).toHaveLength(1);
  });

  it("dedupes narration when progress_update exists for same turn", () => {
    const items = groupTurnReplies([
      block("p1", "progress_update", { meta: { turn: 2, phase: "execute" }, body: "plan" }),
      block("n1", "assistant_message", {
        body: "plan",
        meta: { turn: 2, narration: true, message_role: "status" },
      }),
    ]);
    const deduped = dedupeNarrationWithProgress(items);
    expect(deduped.filter((i) => i.kind === "block")).toHaveLength(1);
  });

  it("archives older phases on live turns", () => {
    const segments = [
      { id: "1", phase: "intent" as const, extras: [] },
      { id: "2", phase: "execute" as const, extras: [] },
      { id: "3", phase: "execute" as const, extras: [] },
      { id: "4", phase: "discovery" as const, extras: [] },
      { id: "5", phase: "deliver" as const, extras: [] },
    ];
    const plan = phaseVisibilityPlan(segments, true);
    expect(plan.archived).toHaveLength(1);
    expect(plan.visible).toHaveLength(4);
  });
});

describe("lastToolClusterRef", () => {
  function turn(id: string, replies: TranscriptBlock[]): TranscriptTurnLike {
    return { id, replies };
  }

  it("returns the last tool cluster inside the last turn", () => {
    const turns = [
      turn("t1", [
        ...toolPair("a", "1:1"),
        block("done1", "assistant_message", { body: "first done" }),
      ]),
      turn("t2", [
        block("plan", "assistant_message", { body: "plan" }),
        ...toolPair("b", "1:1"),
        block("mid", "assistant_message", { body: "继续" }),
        ...toolPair("c", "1:2"),
      ]),
    ];
    expect(lastToolClusterRef(turns)).toEqual({ turnId: "t2", itemIndex: 3 });
  });

  it("falls back to an earlier turn when the last turn has no tool cluster", () => {
    const turns = [
      turn("t1", [
        ...toolPair("a", "1:1"),
        block("done1", "assistant_message", { body: "done" }),
      ]),
      turn("t2", [block("plain", "assistant_message", { body: "ok" })]),
    ];
    expect(lastToolClusterRef(turns)).toEqual({ turnId: "t1", itemIndex: 0 });
  });

  it("returns null when no turn has a tool cluster", () => {
    const turns = [
      turn("t1", [block("plain", "assistant_message", { body: "hi" })]),
      turn("t2", [block("plain2", "assistant_message", { body: "bye" })]),
    ];
    expect(lastToolClusterRef(turns)).toBeNull();
  });

  it("returns null for empty transcripts", () => {
    expect(lastToolClusterRef([])).toBeNull();
  });
});
