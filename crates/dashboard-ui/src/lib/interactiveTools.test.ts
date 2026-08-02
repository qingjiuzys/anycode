import { describe, expect, it } from "vitest";
import type { TranscriptBlock } from "@/api/types";
import type { ToolStep } from "@/lib/transcriptGrouping";
import {
  interactiveStepHistoryLabel,
  isInteractiveToolCluster,
  shouldHideInteractiveCluster,
} from "@/lib/interactiveTools";

function step(name: string, body = ""): ToolStep {
  return {
    key: "1:1",
    call: {
      id: "c1",
      block_type: "tool_call",
      at: "2026-01-01T00:00:00Z",
      title: `${name} started`,
      body,
      meta: { name },
    } as TranscriptBlock,
    result: {
      id: "r1",
      block_type: "tool_result",
      at: "2026-01-01T00:00:01Z",
      title: `${name} finished`,
      body: "",
      meta: { name },
    } as TranscriptBlock,
  };
}

describe("interactiveTools", () => {
  it("detects AskUserQuestion clusters", () => {
    expect(isInteractiveToolCluster([step("AskUserQuestion")])).toBe(true);
    expect(isInteractiveToolCluster([step("Grep")])).toBe(false);
  });

  it("hides interactive cluster on last running turn with pending questions", () => {
    expect(
      shouldHideInteractiveCluster({
        isLast: true,
        isRunning: true,
        steps: [step("AskUserQuestion")],
        pendingQuestionsCount: 1,
        pendingApprovalsCount: 0,
      }),
    ).toBe(true);
    expect(
      shouldHideInteractiveCluster({
        isLast: false,
        isRunning: true,
        steps: [step("AskUserQuestion")],
        pendingQuestionsCount: 1,
        pendingApprovalsCount: 0,
      }),
    ).toBe(false);
  });

  it("keeps settled interactive cluster visible on last running turn", () => {
    expect(
      shouldHideInteractiveCluster({
        isLast: true,
        isRunning: true,
        steps: [step("AskUserQuestion")],
        pendingQuestionsCount: 0,
        pendingApprovalsCount: 0,
      }),
    ).toBe(false);
  });

  it("keeps interactive cluster visible on settled sessions", () => {
    expect(
      shouldHideInteractiveCluster({
        isLast: true,
        isRunning: false,
        steps: [step("AskUserQuestion")],
        pendingQuestionsCount: 0,
        pendingApprovalsCount: 0,
      }),
    ).toBe(false);
  });

  it("extracts question header from tool call body", () => {
    const label = interactiveStepHistoryLabel(
      step("AskUserQuestion", '{"header":"产品信息","question":"请选择"}'),
    );
    expect(label).toBe("产品信息");
  });
});
