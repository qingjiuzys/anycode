import { describe, expect, it, beforeEach } from "vitest";
import {
  clearOptimisticResolvedApprovals,
  getOptimisticResolvedApprovalIds,
  markApprovalResolvedOptimistic,
  unmarkApprovalResolvedOptimistic,
} from "@/lib/approvalOptimisticStore";

describe("approvalOptimisticStore", () => {
  beforeEach(() => {
    clearOptimisticResolvedApprovals("s1");
    clearOptimisticResolvedApprovals(undefined);
  });

  it("marks and unmarks per session", () => {
    markApprovalResolvedOptimistic("s1", "a1");
    expect(getOptimisticResolvedApprovalIds("s1").has("a1")).toBe(true);
    expect(getOptimisticResolvedApprovalIds("s2").has("a1")).toBe(false);
    unmarkApprovalResolvedOptimistic("s1", "a1");
    expect(getOptimisticResolvedApprovalIds("s1").has("a1")).toBe(false);
  });
});
