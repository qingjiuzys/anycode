import { describe, expect, it } from "vitest";
import type { TranscriptBlock } from "@/api/types";
import type { TurnReplyItem } from "@/lib/transcriptGrouping";
import {
  deriveTurnLiveStatus,
  hideComposerWaitingFromSession,
  STALL_WARN_SECONDS,
  turnEndedAtFromReplies,
} from "@/lib/turnLiveStatus";

function block(id: string, type: string, body = "", meta?: Record<string, unknown>): TurnReplyItem {
  return {
    kind: "block",
    block: {
      id,
      block_type: type,
      at: "2026-01-01T00:00:10Z",
      title: id,
      body,
      meta,
    } as TranscriptBlock,
  };
}

describe("deriveTurnLiveStatus", () => {
  it("hides typing indicator when turn phase is active", () => {
    const status = deriveTurnLiveStatus({
      isLast: true,
      isRunning: true,
      streamHasActivity: false,
      turnStartedAt: "2026-01-01T00:00:00Z",
      replyItems: [],
      turnPhase: "waiting_first_token",
    });
    expect(status.showLiveRecap).toBe(true);
    expect(status.showTypingIndicator).toBe(false);
    expect(status.hideComposerWaiting).toBe(true);
  });

  it("shows typing indicator only before any tool or phase activity", () => {
    const status = deriveTurnLiveStatus({
      isLast: true,
      isRunning: true,
      streamHasActivity: false,
      turnStartedAt: "2026-01-01T00:00:00Z",
      replyItems: [],
      turnPhase: null,
    });
    expect(status.showTypingIndicator).toBe(true);
    expect(status.showLiveRecap).toBe(false);
    expect(status.hideComposerWaiting).toBe(false);
  });

  it("suppresses typing when assistant text is streaming", () => {
    const status = deriveTurnLiveStatus({
      isLast: true,
      isRunning: true,
      streamHasActivity: true,
      turnStartedAt: "2026-01-01T00:00:00Z",
      replyItems: [block("a1", "assistant_message", "Hello")],
      turnPhase: "streaming",
    });
    expect(status.showTypingIndicator).toBe(false);
    expect(status.hasAssistantText).toBe(true);
    expect(status.hideComposerWaiting).toBe(true);
  });

  it("uses stall actions when live without active tools", () => {
    const status = deriveTurnLiveStatus({
      isLast: true,
      isRunning: true,
      streamHasActivity: false,
      turnStartedAt: "2026-01-01T00:00:00Z",
      replyItems: [],
      turnPhase: "waiting_first_token",
    });
    expect(status.showStallActions).toBe(true);
  });

  it("suppresses stall actions while waiting for user input", () => {
    const status = deriveTurnLiveStatus({
      isLast: true,
      isRunning: true,
      streamHasActivity: false,
      turnStartedAt: "2026-01-01T00:00:00Z",
      replyItems: [],
      turnPhase: "running_tools",
      pendingQuestionsCount: 1,
    });
    expect(status.waitingForUser).toBe(true);
    expect(status.showStallActions).toBe(false);
    expect(status.showClusterActivityLine).toBe(false);
  });

  it("uses compact recap when progress content is visible", () => {
    const status = deriveTurnLiveStatus({
      isLast: true,
      isRunning: true,
      streamHasActivity: false,
      turnStartedAt: "2026-01-01T00:00:00Z",
      replyItems: [
        {
          kind: "block",
          block: {
            id: "p1",
            block_type: "progress_update",
            at: "",
            title: "",
            body: "Checking",
            meta: { phase: "execute" },
          },
        },
      ],
      turnPhase: null,
    });
    expect(status.recapCompact).toBe(true);
    expect(status.hasProgressContent).toBe(true);
  });

  it("keeps full recap for final assistant without progress cards", () => {
    const status = deriveTurnLiveStatus({
      isLast: true,
      isRunning: true,
      streamHasActivity: false,
      turnStartedAt: "2026-01-01T00:00:00Z",
      replyItems: [block("a1", "assistant_message", "Hello")],
      turnPhase: null,
    });
    expect(status.recapCompact).toBe(false);
    expect(status.hasFinalAssistantText).toBe(true);
  });
});

describe("hideComposerWaitingFromSession", () => {
  it("hides composer waiting when phase or stream activity is present", () => {
    expect(
      hideComposerWaitingFromSession({
        running: true,
        streamHasActivity: false,
        turnPhase: "running_tools",
        liveBlocksActive: false,
      }),
    ).toBe(true);
    expect(
      hideComposerWaitingFromSession({
        running: true,
        streamHasActivity: true,
        turnPhase: null,
        liveBlocksActive: false,
      }),
    ).toBe(true);
    expect(
      hideComposerWaitingFromSession({
        running: false,
        streamHasActivity: true,
        turnPhase: "streaming",
        liveBlocksActive: true,
      }),
    ).toBe(false);
  });
});

describe("turnEndedAtFromReplies", () => {
  it("picks the latest reply timestamp", () => {
    const ended = turnEndedAtFromReplies(
      [
        { at: "2026-01-01T00:00:05Z" },
        { at: "2026-01-01T00:01:00Z" },
        { at: "2026-01-01T00:00:30Z" },
      ],
      "2026-01-01T00:00:00Z",
    );
    expect(ended).toBe("2026-01-01T00:01:00Z");
  });
});

describe("STALL_WARN_SECONDS", () => {
  it("uses 60s threshold for stall UX", () => {
    expect(STALL_WARN_SECONDS).toBe(60);
  });
});
