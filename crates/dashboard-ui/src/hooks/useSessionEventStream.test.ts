import { describe, expect, it } from "vitest";
import {
  conversationStreamLive,
  conversationThreadRunning,
  shouldClearOptimisticOnRunningHandoff,
  shouldRebaseLiveOnSseReconnect,
  shouldSuppressTranscriptInvalidate,
  shouldTrackChatEventAsLive,
} from "@/hooks/useSessionEventStream";

describe("shouldRebaseLiveOnSseReconnect", () => {
  it("rebases only after reconnecting back to live", () => {
    expect(shouldRebaseLiveOnSseReconnect("reconnecting", "live")).toBe(true);
  });

  it("does not rebase on first connect", () => {
    expect(shouldRebaseLiveOnSseReconnect("connecting", "live")).toBe(false);
  });

  it("does not rebase while offline or reconnecting", () => {
    expect(shouldRebaseLiveOnSseReconnect("live", "reconnecting")).toBe(false);
    expect(shouldRebaseLiveOnSseReconnect("offline", "connecting")).toBe(false);
  });
});

describe("shouldTrackChatEventAsLive", () => {
  it("tracks live events only for running sessions", () => {
    expect(shouldTrackChatEventAsLive("running", false)).toBe(true);
    expect(shouldTrackChatEventAsLive("completed", false)).toBe(false);
    expect(shouldTrackChatEventAsLive("pending", false)).toBe(false);
  });

  it("tracks during optimistic streaming before list status updates", () => {
    expect(shouldTrackChatEventAsLive("completed", true)).toBe(true);
    expect(shouldTrackChatEventAsLive(undefined, true)).toBe(true);
  });
});

describe("conversationThreadRunning", () => {
  it("is running when session status is running", () => {
    expect(conversationThreadRunning("running", "s1", null)).toBe(true);
    expect(conversationThreadRunning("completed", "s1", null)).toBe(false);
  });

  it("is running during optimistic streaming even if list status is stale", () => {
    expect(conversationThreadRunning("completed", "s1", "s1")).toBe(true);
    expect(conversationThreadRunning("completed", "s1", "s2")).toBe(false);
  });

  it("does not treat replayed live blocks as running", () => {
    expect(
      conversationThreadRunning("completed", "s1", null),
    ).toBe(false);
  });
});

describe("conversationStreamLive", () => {
  it("is false when SSE connected but session is idle", () => {
    expect(conversationStreamLive(false, true, false)).toBe(false);
  });

  it("is true during active chat stream", () => {
    expect(conversationStreamLive(true, false, false)).toBe(true);
  });

  it("is true when SSE connected and session is running", () => {
    expect(conversationStreamLive(false, true, true)).toBe(true);
  });
});

describe("shouldSuppressTranscriptInvalidate", () => {
  it("suppresses when chat is live or trackLive is active", () => {
    expect(shouldSuppressTranscriptInvalidate(true, false)).toBe(true);
    expect(shouldSuppressTranscriptInvalidate(false, true)).toBe(true);
    expect(shouldSuppressTranscriptInvalidate(true, true)).toBe(true);
    expect(shouldSuppressTranscriptInvalidate(false, false)).toBe(false);
  });
});

describe("shouldClearOptimisticOnRunningHandoff", () => {
  it("clears only when server status is running for the optimistic session", () => {
    expect(
      shouldClearOptimisticOnRunningHandoff("running", "s1", "s1"),
    ).toBe(true);
    expect(
      shouldClearOptimisticOnRunningHandoff("completed", "s1", "s1"),
    ).toBe(false);
    expect(
      shouldClearOptimisticOnRunningHandoff("running", "s1", "s2"),
    ).toBe(false);
    expect(
      shouldClearOptimisticOnRunningHandoff("running", null, "s1"),
    ).toBe(false);
  });
});
