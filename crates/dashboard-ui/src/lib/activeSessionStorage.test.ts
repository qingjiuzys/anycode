import { describe, expect, it } from "vitest";
import { resolveShellSessionId } from "@/lib/activeSessionStorage";

describe("resolveShellSessionId", () => {
  it("prefers explicit URL session", () => {
    expect(
      resolveShellSessionId({
        pathname: "/conversations",
        urlSession: "s-url",
        pinnedSessionId: "s-pin",
        fallbackSessionId: "s-fallback",
      }),
    ).toBe("s-url");
  });

  it("uses pinned session when URL omits session", () => {
    expect(
      resolveShellSessionId({
        pathname: "/conversations",
        urlSession: undefined,
        pinnedSessionId: "s-pin",
        fallbackSessionId: "s-fallback",
      }),
    ).toBe("s-pin");
  });

  it("does not auto-pick sidebar fallback on home", () => {
    expect(
      resolveShellSessionId({
        pathname: "/",
        urlSession: undefined,
        pinnedSessionId: null,
        fallbackSessionId: "s-fallback",
      }),
    ).toBeNull();
  });

  it("keeps pinned session on home for background SSE", () => {
    expect(
      resolveShellSessionId({
        pathname: "/",
        urlSession: undefined,
        pinnedSessionId: "s-pin",
        fallbackSessionId: "s-fallback",
      }),
    ).toBe("s-pin");
  });
});
