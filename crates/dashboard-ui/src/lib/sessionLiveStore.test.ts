import { describe, expect, it } from "vitest";
import { deriveSessionLiveState, deriveTurnPhase } from "@/lib/sessionLiveStore";
import type { ChatStreamEvent } from "@/lib/liveTranscript";

describe("sessionLiveStore", () => {
  it("derives waiting_first_token from llm_start event", () => {
    const events: ChatStreamEvent[] = [
      {
        session_id: "s1",
        project_id: "p1",
        kind: "llm_start",
        turn: 1,
        at: "2026-01-01T00:00:00Z",
      },
    ];
    const { phase, startedAt } = deriveTurnPhase([], events);
    expect(phase).toBe("waiting_first_token");
    expect(startedAt).toBe("2026-01-01T00:00:00Z");
  });

  it("collects pending questions from question_request blocks", () => {
    const state = deriveSessionLiveState(
      [
        {
          id: "q1",
          block_type: "question_request",
          at: "t",
          title: "Choice",
          body: "Pick one?",
          meta: {
            question_id: "q_1",
            session_id: "s1",
            options: [{ label: "A", description: "first" }],
            multi_select: false,
          },
        },
      ],
      [],
      [],
      [],
      "s1",
    );
    expect(state.pendingQuestions).toHaveLength(1);
    expect(state.pendingQuestions[0]?.question_id).toBe("q_1");
  });

  it("removes questions after question_resolved", () => {
    const state = deriveSessionLiveState(
      [
        {
          id: "q1",
          block_type: "question_request",
          at: "t",
          title: "Choice",
          body: "Pick one?",
          meta: {
            question_id: "q_1",
            session_id: "s1",
            options: [{ label: "A" }],
            multi_select: false,
          },
        },
        {
          id: "qr1",
          block_type: "system_notice",
          at: "t2",
          title: "Question answered",
          body: "",
          meta: { source: "question_resolved", question_id: "q_1" },
        },
      ],
      [],
      [],
      [],
      "s1",
    );
    expect(state.pendingQuestions).toHaveLength(0);
  });

  it("upgrades turn phase to running_tools on tool_start", () => {
    const events: ChatStreamEvent[] = [
      {
        session_id: "s1",
        project_id: "p1",
        kind: "turn_phase",
        at: "2026-01-01T00:00:00Z",
        payload: { phase: "waiting_first_token" },
      },
      {
        session_id: "s1",
        project_id: "p1",
        kind: "tool_start",
        at: "2026-01-01T00:00:05Z",
      },
    ];
    expect(deriveTurnPhase([], events).phase).toBe("running_tools");
  });

  it("clears turn phase when session is not running", () => {
    const events: ChatStreamEvent[] = [
      {
        session_id: "s1",
        project_id: "p1",
        kind: "tool_start",
        at: "2026-01-01T00:00:05Z",
      },
    ];
    const running = deriveSessionLiveState([], events, [], [], "s1", true);
    expect(running.turnPhase).toBe("running_tools");
    const idle = deriveSessionLiveState([], events, [], [], "s1", false);
    expect(idle.turnPhase).toBeNull();
  });

  it("collects pending approvals and drops them after approval_resolved", () => {
    const request = {
      id: "ap1",
      block_type: "approval_request" as const,
      at: "t",
      title: "Approve Bash",
      body: "rm -rf /tmp/x",
      meta: {
        approval_id: "apr-1",
        session_id: "s1",
        tool: "Bash",
        input_preview: "rm -rf /tmp/x",
      },
    };
    const pending = deriveSessionLiveState([request], [], [], [], "s1");
    expect(pending.pendingApprovals).toHaveLength(1);
    expect(pending.pendingApprovals[0]?.approval_id).toBe("apr-1");

    const resolved = deriveSessionLiveState(
      [
        request,
        {
          id: "ar1",
          block_type: "system_notice",
          at: "t2",
          title: "Approval resolved",
          body: "allow_once",
          meta: {
            source: "approval_resolved",
            approval_id: "apr-1",
            decision: "allow_once",
          },
        },
      ],
      [],
      [],
      [],
      "s1",
    );
    expect(resolved.pendingApprovals).toHaveLength(0);
  });

  it("honors optimistic resolved approval ids before SSE catches up", () => {
    const state = deriveSessionLiveState(
      [
        {
          id: "ap1",
          block_type: "approval_request",
          at: "t",
          title: "Approve Bash",
          body: "echo hi",
          meta: {
            approval_id: "apr-2",
            session_id: "s1",
            tool: "Bash",
          },
        },
      ],
      [],
      [],
      [],
      "s1",
      true,
      ["apr-2"],
    );
    expect(state.pendingApprovals).toHaveLength(0);
  });
});
