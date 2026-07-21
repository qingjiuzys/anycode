import { describe, expect, it } from "vitest";
import { isInlineKind, kindForPath, mimeForPath } from "./artifactKind";

describe("artifactKind", () => {
  it("maps extensions to kinds", () => {
    expect(kindForPath("/tmp/a.png")).toBe("image");
    expect(kindForPath("/tmp/a.mp4")).toBe("video");
    expect(kindForPath("/tmp/deck.pptx")).toBe("presentation");
    expect(kindForPath("/tmp/notes.mindmap.md")).toBe("mindmap");
    expect(kindForPath("/tmp/x.pdf")).toBe("pdf");
  });

  it("respects kind hints", () => {
    expect(kindForPath("/tmp/outline.md", "mindmap")).toBe("mindmap");
  });

  it("guesses mime", () => {
    expect(mimeForPath("/tmp/a.png")).toBe("image/png");
    expect(mimeForPath("/tmp/a.pptx")).toContain("presentation");
  });

  it("marks rich kinds inline", () => {
    expect(isInlineKind("image")).toBe(true);
    expect(isInlineKind("presentation")).toBe(true);
    expect(isInlineKind("file")).toBe(false);
  });
});
