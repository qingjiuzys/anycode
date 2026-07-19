import { describe, expect, it } from "vitest";
import { smoothTextStep, STREAM_REVEAL_CHARS_PER_SEC } from "@/lib/smoothTextCore";

describe("smoothTextStep", () => {
  it("reveals text gradually", () => {
    const target = "hello world";
    const displayed = smoothTextStep("", target, 50);
    expect(displayed.length).toBeGreaterThan(0);
    expect(displayed.length).toBeLessThan(target.length);
    expect(target.startsWith(displayed)).toBe(true);
  });

  it("snaps when already at target", () => {
    expect(smoothTextStep("done", "done", 16)).toBe("done");
  });

  it("resets when target no longer extends displayed prefix", () => {
    const next = smoothTextStep("old text", "new text", 16);
    expect(next.startsWith("n")).toBe(true);
    expect(next.length).toBeGreaterThan(0);
  });

  it("reveals at a fixed rate regardless of backlog size", () => {
    const deltaMs = 50;
    const expectedChars = Math.max(
      1,
      Math.floor((STREAM_REVEAL_CHARS_PER_SEC * deltaMs) / 1000),
    );
    const smallBacklog = smoothTextStep("a".repeat(10), "a".repeat(20), deltaMs).length - 10;
    const largeBacklog = smoothTextStep("", "a".repeat(500), deltaMs).length;
    expect(smallBacklog).toBe(expectedChars);
    expect(largeBacklog).toBe(expectedChars);
  });
});
