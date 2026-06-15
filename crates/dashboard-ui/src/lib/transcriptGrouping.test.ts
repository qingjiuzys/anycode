import { describe, expect, it } from "vitest";
import type { TranscriptBlock } from "@/api/types";
import { countLogicalToolSteps, groupTurnReplies } from "@/lib/transcriptGrouping";

function block(
  id: string,
  blockType: string,
  extra?: Partial<TranscriptBlock>,
): TranscriptBlock {
  return {
    id,
    block_type: blockType,
    at: "2026-01-01T00:00:00Z",
    title: "Bash",
    body: "",
    ...extra,
  };
}

describe("groupTurnReplies", () => {
  it("merges non-consecutive tool blocks into one group", () => {
    const replies = [
      block("t1", "tool_call", { meta: { tool_key: "1:1", phase: "start" } }),
      block("a1", "assistant_message", { body: "planning" }),
      block("t2", "tool_result", { meta: { tool_key: "1:1", phase: "end" } }),
      block("t3", "tool_call", { meta: { tool_key: "1:2", phase: "start" } }),
      block("f1", "assistant_message", { body: "done" }),
    ];
    const grouped = groupTurnReplies(replies);
    expect(grouped).toHaveLength(3);
    expect(grouped[0]?.kind).toBe("tool_group");
    if (grouped[0]?.kind === "tool_group") {
      expect(grouped[0].tools).toHaveLength(3);
    }
    expect(grouped[1]?.kind).toBe("block");
    expect(grouped[2]?.kind).toBe("block");
  });

  it("folds intermediate assistant notices into process message count", () => {
    const replies = [
      block("n1", "system_notice", {
        meta: { source: "intermediate_assistant" },
        body: "checking env",
      }),
      block("t1", "tool_call"),
      block("t2", "tool_result"),
    ];
    const grouped = groupTurnReplies(replies);
    expect(grouped).toHaveLength(1);
    if (grouped[0]?.kind === "tool_group") {
      expect(grouped[0].processMessageCount).toBe(1);
      expect(grouped[0].tools).toHaveLength(2);
    }
  });
});

describe("countLogicalToolSteps", () => {
  it("counts by tool_key not raw blocks", () => {
    const tools = [
      block("t1", "tool_call", { meta: { tool_key: "1:1" } }),
      block("t2", "tool_result", { meta: { tool_key: "1:1" } }),
      block("t3", "tool_call", { meta: { tool_key: "1:2" } }),
      block("t4", "tool_result", { meta: { tool_key: "1:2" } }),
    ];
    expect(countLogicalToolSteps(tools)).toBe(2);
  });
});
