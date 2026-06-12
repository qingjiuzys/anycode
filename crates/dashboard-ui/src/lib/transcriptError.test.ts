import { describe, expect, it } from "vitest";
import { humanizeTranscriptError } from "./transcriptError";

describe("humanizeTranscriptError", () => {
  const tool = (f: string) => `tool:${f}`;
  const missing = (f: string) => `missing:${f}`;

  it("humanizes bare path", () => {
    expect(humanizeTranscriptError("path", tool, missing).summary).toBe("tool:path");
  });

  it("humanizes backend tool parameter message", () => {
    expect(
      humanizeTranscriptError(
        "Tool parameter error: missing or invalid `path`",
        tool,
        missing,
      ).summary,
    ).toBe("tool:path");
  });

  it("passes through normal errors", () => {
    const msg = "LLM error: rate limit";
    expect(humanizeTranscriptError(msg, tool, missing).summary).toBe(msg);
  });
});
