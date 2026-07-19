import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import type { ApprovalSummaryResponse, PendingApprovalsResponse } from "@/api/types";
import {
  applyApprovalResolvedToCaches,
  removeApprovalFromPending,
  removeApprovalFromSummary,
  restoreApprovalResolvedCaches,
} from "./approvalCache";

const pendingFixture = (): PendingApprovalsResponse => ({
  web_enabled: true,
  respond_allowed: true,
  pending: [
    {
      approval_id: "a1",
      session_id: "s1",
      tool: "BrowserNavigate",
      input_preview: "https://www.baidu.com",
      created_at: "2026-01-01T00:00:00Z",
      status: "pending",
    },
    {
      approval_id: "a2",
      session_id: "s1",
      tool: "Bash",
      input_preview: "ls",
      created_at: "2026-01-01T00:01:00Z",
      status: "pending",
    },
  ],
});

const summaryFixture = (): ApprovalSummaryResponse => ({
  web_enabled: true,
  respond_allowed: true,
  summary: {
    pending_total: 2,
    by_session: [{ session_id: "s1", count: 2 }],
  },
});

describe("approvalCache", () => {
  it("removes approval from pending list", () => {
    const next = removeApprovalFromPending(pendingFixture(), "a1");
    expect(next?.pending.map((p) => p.approval_id)).toEqual(["a2"]);
  });

  it("decrements summary session count and total", () => {
    const next = removeApprovalFromSummary(summaryFixture(), "s1");
    expect(next?.summary.pending_total).toBe(1);
    expect(next?.summary.by_session).toEqual([{ session_id: "s1", count: 1 }]);
  });

  it("drops session row when count reaches zero", () => {
    const one: ApprovalSummaryResponse = {
      ...summaryFixture(),
      summary: { pending_total: 1, by_session: [{ session_id: "s1", count: 1 }] },
    };
    const next = removeApprovalFromSummary(one, "s1");
    expect(next?.summary.pending_total).toBe(0);
    expect(next?.summary.by_session).toEqual([]);
  });

  it("applyApprovalResolvedToCaches updates summary and rehydrate; restore rolls back", () => {
    const qc = new QueryClient();
    qc.setQueryData(["security-approvals-summary"], summaryFixture());
    qc.setQueryData(["pending-approvals-rehydrate", "s1"], pendingFixture());
    qc.setQueryData(["security-approvals-pending", "s1"], pendingFixture());

    const snap = applyApprovalResolvedToCaches(qc, {
      approvalId: "a1",
      sessionId: "s1",
    });

    expect(qc.getQueryData<ApprovalSummaryResponse>(["security-approvals-summary"])?.summary)
      .toEqual({ pending_total: 1, by_session: [{ session_id: "s1", count: 1 }] });
    expect(
      qc.getQueryData<PendingApprovalsResponse>(["pending-approvals-rehydrate", "s1"])?.pending
        .map((p) => p.approval_id),
    ).toEqual(["a2"]);

    restoreApprovalResolvedCaches(qc, snap);
    expect(qc.getQueryData<ApprovalSummaryResponse>(["security-approvals-summary"])).toEqual(
      summaryFixture(),
    );
    expect(
      qc.getQueryData<PendingApprovalsResponse>(["pending-approvals-rehydrate", "s1"])?.pending
        .length,
    ).toBe(2);
  });
});
