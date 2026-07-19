import { describe, expect, it } from "vitest";
import {
  isScrollNearBottom,
  lastAssistantBodyLength,
  shouldSkipScrollToBottom,
  streamFollowSignature,
} from "@/lib/transcriptScroll";
import type { TranscriptBlock } from "@/api/types";

describe("isScrollNearBottom", () => {
  it("returns true when within threshold of bottom", () => {
    expect(
      isScrollNearBottom({ scrollHeight: 1000, scrollTop: 880, clientHeight: 100 }, 120),
    ).toBe(true);
    expect(
      isScrollNearBottom({ scrollHeight: 1000, scrollTop: 700, clientHeight: 100 }, 120),
    ).toBe(false);
  });
});

describe("shouldSkipScrollToBottom", () => {
  it("returns true when already at bottom", () => {
    expect(
      shouldSkipScrollToBottom({ scrollHeight: 1000, scrollTop: 900, clientHeight: 100 }),
    ).toBe(true);
  });

  it("returns false when not near bottom", () => {
    expect(
      shouldSkipScrollToBottom({ scrollHeight: 1000, scrollTop: 800, clientHeight: 100 }),
    ).toBe(false);
  });
});

describe("lastAssistantBodyLength", () => {
  it("reads trailing assistant block body length", () => {
    const blocks = [
      { block_type: "user_message", body: "hi" },
      { block_type: "assistant_message", body: "hello world" },
    ] as TranscriptBlock[];
    expect(lastAssistantBodyLength(blocks)).toBe(11);
  });
});

describe("streamFollowSignature", () => {
  it("is empty when not streaming", () => {
    expect(
      streamFollowSignature({
        running: false,
        streamLive: false,
        blocksLength: 3,
        liveEventsLength: 1,
        turnHasActivity: true,
        liveBlocksLength: 2,
      }),
    ).toBe("");
  });

  it("changes on phase transition but not on raw assistant length alone", () => {
    const base = {
      running: true,
      streamLive: true,
      blocksLength: 3,
      liveEventsLength: 4,
      turnHasActivity: true,
      liveBlocksLength: 2,
    };
    const planning = streamFollowSignature({ ...base, turnPhase: "waiting_first_token" });
    const streaming = streamFollowSignature({ ...base, turnPhase: "streaming" });
    expect(planning).not.toBe(streaming);
  });
});
