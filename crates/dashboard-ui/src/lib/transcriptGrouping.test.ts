import { describe, expect, it } from "vitest";
import type { TranscriptBlock } from "@/api/types";
import {
  countLogicalToolSteps,
  groupTurnReplies,
  mergeFinalAssistantBlocks,
} from "@/lib/transcriptGrouping";

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
  it("keeps agent narration visible before its tool evidence", () => {
    const replies = [
      block("a1", "assistant_message", {
        body: "planning",
        meta: { narration: true, message_role: "status" },
      }),
      block("t1", "tool_call", { meta: { tool_key: "1:1", phase: "start" } }),
      block("t2", "tool_result", { meta: { tool_key: "1:1", phase: "end" } }),
      block("t3", "tool_call", { meta: { tool_key: "1:2", phase: "start" } }),
      block("f1", "assistant_message", { body: "done" }),
    ];
    const grouped = groupTurnReplies(replies);
    expect(grouped.map((item) => item.kind)).toEqual(["block", "tool_cluster", "block"]);
    if (grouped[0]?.kind === "block") {
      expect(grouped[0].block.body).toBe("planning");
    }
    if (grouped[1]?.kind === "tool_cluster") {
      expect(grouped[1].processSnippets).not.toContain("planning");
    }
    if (grouped[3]?.kind === "block") {
      expect(grouped[3].block.body).toBe("done");
    }
  });

  it("keeps all intermediate_assistant narration on the timeline before tools", () => {
    const replies = [
      block("n0", "system_notice", {
        meta: { source: "intermediate_assistant" },
        body: "oldest step",
      }),
      block("n1", "system_notice", {
        meta: { source: "intermediate_assistant" },
        body: "checking env",
      }),
      block("t1", "tool_call", { meta: { tool_key: "1:1" } }),
      block("t2", "tool_result", { meta: { tool_key: "1:1" } }),
      block("mid", "system_notice", {
        meta: { source: "intermediate_assistant" },
        body: "now edit files",
      }),
      block("t3", "tool_call", { meta: { tool_key: "1:2" } }),
      block("delta", "system_notice", {
        meta: { source: "thinking_delta" },
        body: "internal thought",
      }),
    ];
    const grouped = groupTurnReplies(replies);
    expect(grouped.map((item) => item.kind)).toEqual([
      "block",
      "block",
      "tool_cluster",
      "block",
      "tool_cluster",
    ]);
    if (grouped[0]?.kind === "block") {
      expect(grouped[0].block.body).toBe("oldest step");
    }
    if (grouped[1]?.kind === "block") {
      expect(grouped[1].block.body).toBe("checking env");
    }
    if (grouped[3]?.kind === "block") {
      expect(grouped[3].block.body).toBe("now edit files");
    }
    if (grouped[4]?.kind === "tool_cluster") {
      expect(grouped[4].processSnippets).toContain("internal thought");
    }
  });

  it("merges tool clusters separated only by system notices", () => {
    const replies = [
      block("t1", "tool_call", { meta: { tool_key: "1:1", name: "Grep" } }),
      block("t2", "tool_result", { meta: { tool_key: "1:1", name: "Grep" } }),
      block("n1", "system_notice", { body: "scanning", meta: {} }),
      block("t3", "tool_call", { meta: { tool_key: "1:2", name: "Glob" } }),
      block("t4", "tool_result", { meta: { tool_key: "1:2", name: "Glob" } }),
    ];
    const grouped = groupTurnReplies(replies);
    expect(grouped.filter((item) => item.kind === "tool_cluster")).toHaveLength(1);
    const cluster = grouped.find((item) => item.kind === "tool_cluster");
    if (cluster?.kind === "tool_cluster") {
      expect(cluster.steps.length).toBe(2);
      expect(cluster.processSnippets).toContain("scanning");
    }
  });

  it("keeps live assistant bubbles even when body is still empty", () => {
    const replies = [
      block("live", "assistant_message", { body: "", meta: { live: true } }),
      block("t1", "tool_call", { meta: { tool_key: "1:1" } }),
    ];
    const grouped = groupTurnReplies(replies);
    expect(grouped[0]?.kind).toBe("block");
    if (grouped[0]?.kind === "block") {
      expect(grouped[0].block.meta?.live).toBe(true);
    }
  });

  it("interleaves mid-turn assistant text with following tools", () => {
    const replies = [
      block("a1", "assistant_message", { body: "先读取 HTML" }),
      block("t1", "tool_call", { meta: { tool_key: "1:1", name: "Read" } }),
      block("t2", "tool_result", { meta: { tool_key: "1:1", name: "Read" } }),
      block("a2", "assistant_message", { body: "开始修改 Hero" }),
      block("t3", "tool_call", { meta: { tool_key: "1:2", name: "Edit" } }),
      block("t4", "tool_result", { meta: { tool_key: "1:2", name: "Edit" } }),
    ];
    const grouped = groupTurnReplies(replies);
    expect(grouped.map((item) => item.kind)).toEqual([
      "block",
      "tool_cluster",
      "block",
      "tool_cluster",
    ]);
    if (grouped[0]?.kind === "block") {
      expect(grouped[0].block.body).toBe("先读取 HTML");
    }
    if (grouped[2]?.kind === "block") {
      expect(grouped[2].block.body).toBe("开始修改 Hero");
    }
  });
});

describe("mergeFinalAssistantBlocks", () => {
  it("merges multiple assistant messages into one bubble", () => {
    const merged = mergeFinalAssistantBlocks([
      block("a1", "assistant_message", { body: "part 1" }),
      block("a2", "assistant_message", { body: "part 2", meta: { live: true } }),
    ]);
    expect(merged).toHaveLength(1);
    expect(merged[0]?.body).toBe("part 1\n\npart 2");
    expect(merged[0]?.meta?.live).toBe(true);
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
