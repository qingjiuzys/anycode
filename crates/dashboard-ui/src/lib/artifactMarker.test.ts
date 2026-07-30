import { describe, expect, it } from "vitest";
import {
  parseArtifactMarkers,
  stripArtifactMarkers,
  markerShouldInline,
  isArtifactScaffoldOnly,
} from "./artifactMarker";

describe("parseArtifactMarkers", () => {
  it("parses a mindmap marker line", () => {
    const text =
      '/tmp/mindmap-foo.md\nANYCODE_ARTIFACT:{"path":"/tmp/mindmap-foo.md","kind":"mindmap","title":"Foo","inline":true}';
    const markers = parseArtifactMarkers(text);
    expect(markers).toHaveLength(1);
    expect(markers[0]).toMatchObject({
      path: "/tmp/mindmap-foo.md",
      kind: "mindmap",
      title: "Foo",
      inline: true,
    });
  });
});

describe("stripArtifactMarkers", () => {
  it("removes marker lines and echoed paths but keeps product echo", () => {
    const text =
      'anycode\n/tmp/mindmap-foo.md\nANYCODE_ARTIFACT:{"path":"/tmp/mindmap-foo.md","kind":"mindmap","title":"Foo","inline":true}';
    expect(stripArtifactMarkers(text)).toBe("anycode");
  });

  it("returns empty string when only marker remains", () => {
    const text =
      'ANYCODE_ARTIFACT:{"path":"/tmp/x.md","kind":"mindmap","inline":true}';
    expect(stripArtifactMarkers(text)).toBe("");
  });
});

describe("isArtifactScaffoldOnly", () => {
  it("detects product echo without marker", () => {
    expect(isArtifactScaffoldOnly("anycode")).toBe(true);
    expect(isArtifactScaffoldOnly("mindmap-foo.md")).toBe(true);
    expect(isArtifactScaffoldOnly("这是正文")).toBe(false);
  });
});

describe("markerShouldInline", () => {
  it("respects explicit inline flag", () => {
    expect(
      markerShouldInline({
        path: "/tmp/x.txt",
        inline: false,
      }),
    ).toBe(false);
    expect(
      markerShouldInline({
        path: "/tmp/mindmap-x.md",
        kind: "mindmap",
        inline: true,
      }),
    ).toBe(true);
  });
});
