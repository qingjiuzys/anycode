import { describe, expect, it } from "vitest";
import { formatDeliveryPreflight, progressPhase, progressSummary } from "./progressMeta";
import type { TranscriptBlock } from "@/api/types";

describe("delivery preflight formatting", () => {
  it("formats delivery_preflight marker", () => {
    const raw =
      "[delivery_preflight] family=office_delivery skill=anycode-ppt brand=fde-editorial scenario=anycode-ppt artifacts=[deck_html_slides:html] gates=2";
    expect(formatDeliveryPreflight(raw)).toContain("交付预检");
    expect(formatDeliveryPreflight(raw)).toContain("anycode-ppt");
  });

  it("maps compile work_stage to intent phase", () => {
    const block = {
      id: "1",
      block_type: "progress_update",
      body: "",
      meta: { phase: "gate", work_stage: "compile", summary: "x" },
    } as TranscriptBlock;
    expect(progressPhase(block)).toBe("intent");
  });

  it("progressSummary rewrites preflight", () => {
    const block = {
      id: "1",
      block_type: "progress_update",
      body: "",
      meta: {
        summary:
          "[delivery_preflight] family=office_delivery skill=anycode-docx brand=fde-editorial scenario=work-report artifacts=[report_docx:docx] gates=3",
      },
    } as TranscriptBlock;
    expect(progressSummary(block)).toContain("交付预检");
  });
});
