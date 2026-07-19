import { describe, expect, it } from "vitest";
import type { TranscriptBlock } from "@/api/types";
import { groupTurnReplies } from "@/lib/transcriptGrouping";
import { groupTurnForWorkLog } from "@/lib/workLogGrouping";

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

describe("workLogGrouping", () => {
  it("merges tool clusters and keeps only terminal assistant as finalReply", () => {
    const items = groupTurnReplies([
      block("p1", "progress_update", {
        body: "checking repo",
        meta: { phase: "execute", summary: "checking repo" },
      }),
      block("a1", "assistant_message", {
        body: "Let me inspect files.",
        meta: { narration: true },
      }),
      block("t1", "tool_call", { meta: { tool_key: "1:1", name: "Glob" } }),
      block("t2", "tool_result", { meta: { tool_key: "1:1", name: "Glob" } }),
      block("a2", "assistant_message", {
        body: "Found 3 files.",
        meta: { narration: true },
      }),
      block("t3", "tool_call", { meta: { tool_key: "2:1", name: "Read" } }),
      block("t4", "tool_result", { meta: { tool_key: "2:1", name: "Read" } }),
      block("a3", "assistant_message", { body: "Here is the final report." }),
    ]);
    const render = groupTurnForWorkLog(items);
    expect(render.finalReply?.id).toBe("a3");
    expect(render.work.toolSteps).toHaveLength(2);
    expect(render.work.progressLines.some((b) => b.id === "a1")).toBe(true);
    expect(render.work.progressLines.some((b) => b.id === "a2")).toBe(true);
    expect(render.work.progressLines.some((b) => b.id === "a3")).toBe(false);
  });

  it("does not treat trailing tool cluster as final deliver", () => {
    const items = groupTurnReplies([
      block("a1", "assistant_message", {
        body: "Running grep.",
        meta: { narration: true },
      }),
      block("t1", "tool_call", { meta: { tool_key: "1:1", name: "Grep" } }),
    ]);
    const render = groupTurnForWorkLog(items);
    expect(render.finalReply).toBeUndefined();
    expect(render.work.progressLines.some((b) => b.id === "a1")).toBe(true);
    expect(render.work.toolSteps).toHaveLength(1);
  });

  it("includes replay intermediate system notices in work lines", () => {
    const items = groupTurnReplies([
      block("n1", "system_notice", {
        body: "Let me check context.",
        meta: { source: "intermediate_assistant" },
      }),
      block("a1", "assistant_message", { body: "Done." }),
    ]);
    const render = groupTurnForWorkLog(items);
    expect(render.finalReply?.id).toBe("a1");
    expect(render.work.progressLines.some((b) => b.id === "n1")).toBe(true);
  });
});
