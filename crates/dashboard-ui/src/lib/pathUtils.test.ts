import { describe, expect, it } from "vitest";
import { basename, extension } from "./pathUtils";

describe("pathUtils", () => {
  it("extracts basename and extension", () => {
    expect(basename("/tmp/report.csv")).toBe("report.csv");
    expect(extension("/tmp/report.csv")).toBe("csv");
    expect(extension("workbook.json")).toBe("json");
  });
});
