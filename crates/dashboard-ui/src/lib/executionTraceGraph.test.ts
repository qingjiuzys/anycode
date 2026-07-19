import { describe, expect, it } from "vitest";
import type { TranscriptBlock } from "@/api/types";
import {
  buildExecutionTraceGraph,
  findRunningToolNodeId,
} from "@/lib/executionTraceGraph";

function block(
  partial: Partial<TranscriptBlock> &
    Pick<TranscriptBlock, "id" | "block_type" | "title" | "body">,
): TranscriptBlock {
  return {
    at: "2026-01-01T00:00:00Z",
    ...partial,
  };
}

describe("buildExecutionTraceGraph", () => {
  it("merges tool call+result into one ok node", () => {
    const blocks: TranscriptBlock[] = [
      block({
        id: "u1",
        block_type: "user_message",
        title: "User",
        body: "list files",
      }),
      block({
        id: "a1",
        block_type: "assistant_message",
        title: "Assistant",
        body: "I will use Glob",
      }),
      block({
        id: "tc1",
        block_type: "tool_call",
        title: "Glob started",
        body: '{"pattern":"**/*.ts"}',
        meta: { name: "Glob", turn: "1", idx: "0" },
      }),
      block({
        id: "tr1",
        block_type: "tool_result",
        title: "Glob finished",
        body: "ok",
        meta: { name: "Glob", turn: "1", idx: "0", duration_ms: 12 },
      }),
      block({
        id: "a2",
        block_type: "assistant_message",
        title: "Assistant",
        body: "Found 3 files",
      }),
    ];

    const { nodes, edges } = buildExecutionTraceGraph(blocks);
    expect(nodes.map((n) => n.data.kind)).toEqual([
      "user",
      "assistant",
      "tool",
      "assistant",
    ]);
    const tool = nodes.find((n) => n.data.kind === "tool");
    expect(tool?.data.label).toBe("Glob");
    expect(tool?.data.status).toBe("ok");
    expect(tool?.data.block?.id).toBe("tr1");
    expect(edges).toHaveLength(3);
    expect(edges[0]?.source).toBe("u1");
    expect(edges[0]?.target).toBe("a1");
  });

  it("marks failed merged tool with status=failed", () => {
    const blocks: TranscriptBlock[] = [
      block({
        id: "u1",
        block_type: "user_message",
        title: "User",
        body: "run skill",
      }),
      block({
        id: "tc1",
        block_type: "tool_call",
        title: "Skill started",
        body: "",
        meta: { name: "Skill", turn: "1", idx: "0" },
      }),
      block({
        id: "tr1",
        block_type: "tool_result",
        title: "Skill failed",
        body: "skill exited non-zero",
        meta: { name: "Skill", turn: "1", idx: "0" },
      }),
    ];

    const { nodes } = buildExecutionTraceGraph(blocks);
    const tool = nodes.find((n) => n.data.kind === "tool");
    expect(tool?.data.status).toBe("failed");
    expect(tool?.data.failed).toBe(true);
    expect(tool?.data.label).toBe("Skill");
    expect(nodes.some((n) => n.data.kind === "result" as never)).toBe(false);
  });

  it("marks running tool without result", () => {
    const blocks: TranscriptBlock[] = [
      block({
        id: "u1",
        block_type: "user_message",
        title: "User",
        body: "search",
      }),
      block({
        id: "tc1",
        block_type: "tool_call",
        title: "Grep started",
        body: "",
        meta: { name: "Grep", turn: "1", idx: "0" },
      }),
    ];

    const { nodes } = buildExecutionTraceGraph(blocks);
    expect(findRunningToolNodeId(nodes)).toBe("tc1");
    const tool = nodes.find((n) => n.id === "tc1");
    expect(tool?.data.running).toBe(true);
    expect(tool?.data.status).toBe("running");
  });

  it("forks AskUserQuestion with chosen/skipped branches", () => {
    const blocks: TranscriptBlock[] = [
      block({
        id: "u1",
        block_type: "user_message",
        title: "User",
        body: "我要吃饭",
      }),
      block({
        id: "tc1",
        block_type: "tool_call",
        title: "AskUserQuestion started",
        body: JSON.stringify({
          question: "怎么吃？",
          header: "吃饭",
          options: [{ label: "在家做饭" }, { label: "去外面吃" }],
        }),
        meta: {
          name: "AskUserQuestion",
          turn: "1",
          idx: "0",
          options: [{ label: "在家做饭" }, { label: "去外面吃" }],
        },
      }),
      block({
        id: "tr1",
        block_type: "tool_result",
        title: "AskUserQuestion finished",
        body: JSON.stringify({
          selected: ["去外面吃"],
          status: "answered",
        }),
        meta: { name: "AskUserQuestion", turn: "1", idx: "0" },
      }),
      block({
        id: "tc2",
        block_type: "tool_call",
        title: "Bash started",
        body: '{"command":"echo out"}',
        meta: { name: "Bash", turn: "1", idx: "1" },
      }),
      block({
        id: "tr2",
        block_type: "tool_result",
        title: "Bash finished",
        body: "out",
        meta: { name: "Bash", turn: "1", idx: "1", duration_ms: 5 },
      }),
    ];

    const { nodes, edges } = buildExecutionTraceGraph(blocks);
    expect(nodes.map((n) => n.data.kind)).toEqual([
      "user",
      "decision",
      "branch",
      "branch",
      "tool",
    ]);

    const branches = nodes.filter((n) => n.data.kind === "branch");
    expect(branches).toHaveLength(2);
    const cook = branches.find((n) => n.data.label.includes("在家做饭"));
    const out = branches.find((n) => n.data.label.includes("去外面吃"));
    expect(cook?.data.chosen).toBeFalsy();
    expect(cook?.data.status).toBe("skipped");
    expect(out?.data.chosen).toBe(true);
    expect(out?.data.status).toBe("chosen");

    const branchEdges = edges.filter((e) => e.source === "tc1");
    expect(branchEdges).toHaveLength(2);
    const cookEdge = branchEdges.find((e) => e.target === cook?.id);
    const outEdge = branchEdges.find((e) => e.target === out?.id);
    expect(cookEdge?.style).toMatchObject({ strokeDasharray: "5 4" });
    expect(outEdge?.animated).toBe(true);

    const bash = nodes.find((n) => n.data.kind === "tool");
    expect(bash?.data.status).toBe("ok");
    expect(edges.some((e) => e.source === out?.id && e.target === bash?.id)).toBe(true);
    expect(edges.some((e) => e.source === cook?.id)).toBe(false);
  });

  it("forks approval_request with Allow/Deny and continues from Allow", () => {
    const blocks: TranscriptBlock[] = [
      block({
        id: "u1",
        block_type: "user_message",
        title: "User",
        body: "run bash",
      }),
      block({
        id: "ap1",
        block_type: "approval_request",
        title: "Approve Bash",
        body: "rm -rf /tmp/x",
        meta: {
          approval_id: "apr-1",
          tool: "Bash",
          input_preview: "rm -rf /tmp/x",
        },
      }),
      block({
        id: "ar1",
        block_type: "system_notice",
        title: "Approval resolved",
        body: "allow_once",
        meta: {
          source: "approval_resolved",
          approval_id: "apr-1",
          decision: "allow_once",
        },
      }),
      block({
        id: "tc1",
        block_type: "tool_call",
        title: "Bash started",
        body: '{"command":"rm -rf /tmp/x"}',
        meta: { name: "Bash", turn: "1", idx: "0" },
      }),
      block({
        id: "tr1",
        block_type: "tool_result",
        title: "Bash finished",
        body: "ok",
        meta: { name: "Bash", turn: "1", idx: "0", duration_ms: 3 },
      }),
    ];

    const { nodes, edges } = buildExecutionTraceGraph(blocks);
    expect(nodes.map((n) => n.data.kind)).toEqual([
      "user",
      "decision",
      "branch",
      "branch",
      "tool",
    ]);
    const branches = nodes.filter((n) => n.data.kind === "branch");
    const allow = branches.find((n) => n.data.branchKey === "allow");
    const deny = branches.find((n) => n.data.branchKey === "deny");
    expect(allow?.data.chosen).toBe(true);
    expect(deny?.data.status).toBe("skipped");
    const tool = nodes.find((n) => n.data.kind === "tool");
    expect(edges.some((e) => e.source === allow?.id && e.target === tool?.id)).toBe(true);
    expect(edges.some((e) => e.source === deny?.id)).toBe(false);
  });

  it("draws pending question_request fork when no AskUserQuestion tool", () => {
    const blocks: TranscriptBlock[] = [
      block({
        id: "u1",
        block_type: "user_message",
        title: "User",
        body: "pick one",
      }),
      block({
        id: "qr1",
        block_type: "question_request",
        title: "吃饭",
        body: "怎么吃？",
        meta: {
          question_id: "q-1",
          header: "吃饭",
          options: [{ label: "在家做饭" }, { label: "去外面吃" }],
        },
      }),
    ];

    const { nodes } = buildExecutionTraceGraph(blocks);
    expect(nodes.map((n) => n.data.kind)).toEqual([
      "user",
      "decision",
      "branch",
      "branch",
    ]);
    const decision = nodes.find((n) => n.data.kind === "decision");
    expect(decision?.data.label).toBe("吃饭");
    expect(decision?.data.running).toBe(true);
    expect(nodes.filter((n) => n.data.kind === "branch").every((n) => !n.data.chosen)).toBe(
      true,
    );
  });

  it("does not double-draw when question_request coexists with AskUserQuestion", () => {
    const blocks: TranscriptBlock[] = [
      block({
        id: "u1",
        block_type: "user_message",
        title: "User",
        body: "pick",
      }),
      block({
        id: "tc1",
        block_type: "tool_call",
        title: "AskUserQuestion started",
        body: JSON.stringify({
          header: "吃饭",
          options: [{ label: "A" }, { label: "B" }],
        }),
        meta: { name: "AskUserQuestion", turn: "1", idx: "0" },
      }),
      block({
        id: "qr1",
        block_type: "question_request",
        title: "吃饭",
        body: "?",
        meta: {
          question_id: "q-1",
          header: "吃饭",
          options: [{ label: "A" }, { label: "B" }],
        },
      }),
      block({
        id: "tr1",
        block_type: "tool_result",
        title: "AskUserQuestion finished",
        body: JSON.stringify({ selected: ["A"], status: "answered" }),
        meta: { name: "AskUserQuestion", turn: "1", idx: "0" },
      }),
    ];

    const { nodes } = buildExecutionTraceGraph(blocks);
    const decisions = nodes.filter((n) => n.data.kind === "decision");
    expect(decisions).toHaveLength(1);
    expect(decisions[0]?.id).toBe("tc1");
  });

  it("enriches Skill and Agent tool subtitles without forking", () => {
    const blocks: TranscriptBlock[] = [
      block({
        id: "u1",
        block_type: "user_message",
        title: "User",
        body: "use skill then agent",
      }),
      block({
        id: "tc1",
        block_type: "tool_call",
        title: "Skill started",
        body: JSON.stringify({ name: "daily-brief" }),
        meta: { name: "Skill", turn: "1", idx: "0" },
      }),
      block({
        id: "tr1",
        block_type: "tool_result",
        title: "Skill finished",
        body: JSON.stringify({ skill: "daily-brief", mode: "instructions" }),
        meta: { name: "Skill", turn: "1", idx: "0", duration_ms: 20 },
      }),
      block({
        id: "tc2",
        block_type: "tool_call",
        title: "Agent started",
        body: JSON.stringify({ agent_type: "explore", prompt: "look around" }),
        meta: { name: "Agent", turn: "1", idx: "1" },
      }),
      block({
        id: "tr2",
        block_type: "tool_result",
        title: "Agent finished",
        body: JSON.stringify({
          status: "completed",
          agent_type: "explore",
          nested_task_id: "abcd1234-ffff",
        }),
        meta: { name: "Agent", turn: "1", idx: "1", duration_ms: 100 },
      }),
    ];

    const { nodes } = buildExecutionTraceGraph(blocks);
    expect(nodes.filter((n) => n.data.kind === "branch")).toHaveLength(0);
    const skill = nodes.find((n) => n.data.label === "Skill");
    const agent = nodes.find((n) => n.data.label === "Agent");
    expect(skill?.data.subtitle).toContain("daily-brief");
    expect(skill?.data.subtitle).toContain("instructions");
    expect(agent?.data.subtitle).toContain("explore");
    expect(agent?.data.subtitle).toContain("completed");
    expect(agent?.data.subtitle).toMatch(/id abcd1234/);
  });

  it("returns empty graph for empty blocks", () => {
    expect(buildExecutionTraceGraph([])).toEqual({ nodes: [], edges: [] });
  });
});
