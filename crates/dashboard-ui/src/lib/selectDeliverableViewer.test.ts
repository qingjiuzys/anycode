import { describe, expect, it } from "vitest";
import { selectDeliverableViewer } from "./selectDeliverableViewer";

describe("selectDeliverableViewer", () => {
  it("routes spreadsheet csv to SpreadsheetViewer", () => {
    const node = selectDeliverableViewer({
      path: "/proj/report.csv",
      title: "report.csv",
      projectId: "p1",
      variant: "compact",
    });
    expect(node).toBeTruthy();
  });

  it("routes html report to PreviewHtmlViewer", () => {
    const node = selectDeliverableViewer({
      path: "/proj/index.html",
      kind: "report",
      projectId: "p1",
      variant: "compact",
    });
    expect(node).toBeTruthy();
  });

  it("routes presentation with preview to PresentationThumbViewer", () => {
    const node = selectDeliverableViewer({
      path: "/proj/deck/index.html",
      kind: "presentation",
      projectId: "p1",
      variant: "full",
    });
    expect(node).toBeTruthy();
  });
});
