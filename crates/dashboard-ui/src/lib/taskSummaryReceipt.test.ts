import { describe, expect, it } from "vitest";
import { isTaskSummaryReceipt, stripTaskReceiptHeading } from "./taskSummaryReceipt";

describe("taskSummaryReceipt", () => {
  it("detects structured receipt markdown", () => {
    expect(isTaskSummaryReceipt("**已完成：** slides\n**关键步骤：** Bash")).toBe(true);
    expect(isTaskSummaryReceipt("### 完成回执\n**已完成：** ok")).toBe(true);
    expect(isTaskSummaryReceipt("普通回复")).toBe(false);
  });

  it("strips duplicate heading", () => {
    expect(stripTaskReceiptHeading("### 完成回执\n**已完成：** ok")).toBe("**已完成：** ok");
  });
});
