import { describe, expect, it } from "vitest";
import {
  SESSION_QUERY_GC_MS,
  transcriptQueryOptions,
  sessionArtifactsQueryOptions,
  transcriptStaleTime,
} from "./sessionQuery";

describe("sessionQuery", () => {
  it("uses infinite stale time for completed sessions", () => {
    expect(transcriptStaleTime(false)).toBe(Number.POSITIVE_INFINITY);
    expect(transcriptStaleTime(true)).toBe(3_000);
  });

  it("suppresses transcript refetch while chat stream is live", () => {
    expect(transcriptStaleTime(true, true)).toBe(Number.POSITIVE_INFINITY);
    expect(transcriptStaleTime(false, true)).toBe(Number.POSITIVE_INFINITY);
  });

  it("suppresses transcript refetch while streamLive is active", () => {
    expect(transcriptStaleTime(true, false, true)).toBe(Number.POSITIVE_INFINITY);
    expect(transcriptStaleTime(false, false, true)).toBe(Number.POSITIVE_INFINITY);
  });

  it("builds stable query keys for prefetch", () => {
    expect(transcriptQueryOptions("sess-1", false)).toMatchObject({
      queryKey: ["session-transcript", "sess-1"],
      gcTime: SESSION_QUERY_GC_MS,
    });
    expect(sessionArtifactsQueryOptions("sess-1", true)).toMatchObject({
      queryKey: ["session-artifacts", "sess-1"],
      gcTime: SESSION_QUERY_GC_MS,
    });
  });
});
