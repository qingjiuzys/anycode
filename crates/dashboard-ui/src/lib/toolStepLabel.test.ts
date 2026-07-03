import { describe, expect, it } from "vitest";
import type { TranscriptBlock } from "@/api/types";
import {
  extractToolCommand,
  formatDurationMs,
  formatToolStepLabel,
} from "@/lib/toolStepLabel";
import type { ToolStep } from "@/lib/transcriptGrouping";

describe("extractToolCommand", () => {
  it("prefers meta.command", () => {
    const block: TranscriptBlock = {
      id: "1",
      block_type: "tool_call",
      at: "t",
      title: "Bash",
      body: "",
      meta: { command: "npm test" },
    };
    expect(extractToolCommand(block)).toBe("npm test");
  });

  it("parses bash command from JSON body", () => {
    const block: TranscriptBlock = {
      id: "1",
      block_type: "tool_call",
      at: "t",
      title: "Bash",
      body: '{"command":"cd foo && npm test"}',
    };
    expect(extractToolCommand(block)).toBe("cd foo && npm test");
  });

  it("summarizes TodoWrite todos", () => {
    const block: TranscriptBlock = {
      id: "1",
      block_type: "tool_call",
      at: "t",
      title: "TodoWrite",
      body: '{"todos":[{"id":"a","content":"x","status":"pending"}]}',
    };
    expect(extractToolCommand(block)).toBe("1 todos");
  });
});

describe("formatDurationMs", () => {
  it("formats seconds for large values", () => {
    expect(formatDurationMs({ duration_ms: "1200" })).toBe("1.2s");
  });
});

describe("formatToolStepLabel", () => {
  it("builds bash label with command and duration", () => {
    const step: ToolStep = {
      key: "1:1",
      call: {
        id: "c1",
        block_type: "tool_call",
        at: "t0",
        title: "Bash started",
        body: '{"command":"ls -la"}',
        meta: { name: "Bash" },
      },
      result: {
        id: "r1",
        block_type: "tool_result",
        at: "t1",
        title: "Bash finished",
        body: "ok",
        meta: { duration_ms: "850", name: "Bash" },
      },
    };
    const label = formatToolStepLabel(step, (block) => block.title.replace(/ started$/, ""));
    expect(label).toContain("Bash");
    expect(label).toContain("ls -la");
    expect(label).toContain("850ms");
    expect(label).toContain("✓");
  });
});
