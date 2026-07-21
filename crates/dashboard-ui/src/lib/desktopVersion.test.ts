import { describe, expect, it } from "vitest";
import { compareSemver } from "./desktopVersion";

describe("compareSemver", () => {
  it("orders versions", () => {
    expect(compareSemver("0.2.5", "0.2.4")).toBe(1);
    expect(compareSemver("0.2.4", "0.2.4")).toBe(0);
    expect(compareSemver("0.2.3", "0.2.4")).toBe(-1);
    expect(compareSemver("v0.3.0", "0.2.9")).toBe(1);
  });
});
