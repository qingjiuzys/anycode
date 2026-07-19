import { describe, expect, it } from "vitest";
import type { TranscriptBlock } from "@/api/types";
import { groupTurnReplies } from "@/lib/transcriptGrouping";
import {
  dedupeNarrationWithProgress,
  groupTurnRepliesByPhase,
  phaseVisibilityPlan,
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
