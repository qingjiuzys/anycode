import { describe, expect, it } from "vitest";
import { formatToolFailureRecovery } from "@/lib/agentActivitySummary";
import type { ToolStep } from "@/lib/transcriptGrouping";

describe("formatToolFailureRecovery", () => {
  it("returns recovery line for failed tool step", () => {
    const step: ToolStep = {
      key: "1:1",
      call: {
        id: "c1",
        block_type: "tool_call",
        at: "",
        title: "Bash started",
        body: "",
        meta: { name: "Bash" },
      },
      result: {
        id: "r1",
        block_type: "tool_result",
        at: "",
        title: "Bash failed",
        body: "exit code 1",
        meta: { name: "Bash" },
      },
    };
    const line = formatToolFailureRecovery(step, (k) =>
      k === "conversations.toolFailureRecovery" ? "{tool} failed · recovering" : k,
    );
    expect(line).toContain("Bash");
  });
});
