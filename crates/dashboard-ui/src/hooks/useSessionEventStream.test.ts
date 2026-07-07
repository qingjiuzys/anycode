import { describe, expect, it } from "vitest";
import { shouldRebaseLiveOnSseReconnect } from "@/hooks/useSessionEventStream";

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
