import { describe, expect, it } from "vitest";
import {
  extractMarkdownTables,
  isLargeMarkdownTable,
  splitMarkdownWithTables,
  tableToCsv,
} from "./markdownTable";

describe("markdownTable", () => {
  const sample = `
Intro text

| A | B | C |
|---|---|---|
| 1 | 2 | 3 |
| 4 | 5 | 6 |
| 7 | 8 | 9 |

Tail
`.trim();

  it("extracts GFM tables", () => {
    const tables = extractMarkdownTables(sample);
    expect(tables).toHaveLength(1);
    expect(tables[0]?.headers).toEqual(["A", "B", "C"]);
    expect(tables[0]?.rows).toHaveLength(3);
  });

  it("detects large tables", () => {
    const tables = extractMarkdownTables(sample);
    expect(isLargeMarkdownTable(tables[0]!)).toBe(true);
  });

  it("splits markdown around large tables", () => {
    const segments = splitMarkdownWithTables(sample);
    expect(segments.some((segment) => segment.type === "table")).toBe(true);
    expect(segments.some((segment) => segment.type === "markdown")).toBe(true);
  });

  it("exports csv", () => {
    const tables = extractMarkdownTables(sample);
    expect(tableToCsv(tables[0]!)).toContain("A,B,C");
  });
});
