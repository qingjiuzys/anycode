import { describe, expect, it } from "vitest";
import { formatMoney } from "./money";

describe("formatMoney", () => {
  it("allows zero fractional digits when explicitly requested", () => {
    expect(() =>
      formatMoney(0, "zh-CN", { maximumFractionDigits: 0 }),
    ).not.toThrow();
    const text = formatMoney(360, "zh-CN", { maximumFractionDigits: 0 });
    expect(text).toMatch(/360/);
  });
});
