import { describe, expect, it } from "vitest";
import {
  isAbsolutePath,
  isProcessArtifactPath,
  resolveDeliverableAbsPath,
} from "./deliverablePath";

describe("resolveDeliverableAbsPath", () => {
  it("keeps absolute paths", () => {
    expect(resolveDeliverableAbsPath("/tmp/a.html", "/proj")).toBe("/tmp/a.html");
  });

  it("joins relative paths under project root", () => {
    expect(resolveDeliverableAbsPath("docs/a.md", "/Users/me/proj")).toBe(
      "/Users/me/proj/docs/a.md",
    );
  });
});

describe("isAbsolutePath", () => {
  it("detects posix and windows", () => {
    expect(isAbsolutePath("/a/b")).toBe(true);
    expect(isAbsolutePath("C:\\a\\b")).toBe(true);
    expect(isAbsolutePath("docs/a.md")).toBe(false);
  });
});

describe("isProcessArtifactPath", () => {
  it("filters trial suffixes and keeps final names", () => {
    expect(isProcessArtifactPath("mindmap-anycode-deep.md")).toBe(true);
    expect(isProcessArtifactPath("mindmap-anycode-depth.md")).toBe(true);
    expect(isProcessArtifactPath("mindmap-anycode-complex.md")).toBe(false);
    expect(isProcessArtifactPath("notes.md.anycode-artifact.json")).toBe(true);
  });
});
