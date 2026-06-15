import { describe, expect, it } from "vitest";
import { isQuotaNearLimit, quotaPercent } from "@/lib/planCatalog";

describe("planCatalog helpers", () => {
  it("quotaPercent caps at 100", () => {
    expect(quotaPercent(500, 1000)).toBe(50);
    expect(quotaPercent(1500, 1000)).toBe(100);
  });

  it("isQuotaNearLimit detects threshold", () => {
    expect(isQuotaNearLimit(800, 1000)).toBe(true);
    expect(isQuotaNearLimit(500, 1000)).toBe(false);
  });
});
