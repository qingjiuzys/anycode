import { describe, expect, it } from "vitest";
import {
  collectInlineDeliverables,
  parseDeliverablePathMentions,
  pathFromWriteToolStep,
} from "./deliverableInference";
import type { TranscriptBlock } from "@/api/types";

describe("parseDeliverablePathMentions", () => {
  it("finds mindmap filenames in prose", () => {
    const text =
      "一共 13 个一级节点，已写入 mindmap-anycode-complex.md。";
    const markers = parseDeliverablePathMentions(text);
    expect(markers).toHaveLength(1);
    expect(markers[0]?.path).toBe("mindmap-anycode-complex.md");
    expect(markers[0]?.kind).toBe("mindmap");
  });

  it("finds csv paths in prose", () => {
    const markers = parseDeliverablePathMentions("已写入 sales-report.csv。");
    expect(markers).toHaveLength(1);
    expect(markers[0]?.path).toBe("sales-report.csv");
    expect(markers[0]?.kind).toBe("spreadsheet");
  });
});

describe("pathFromWriteToolStep", () => {
  it("reads path from FileWrite result meta", () => {
    const path = pathFromWriteToolStep({
      key: "1",
      call: {
        id: "c1",
        block_type: "tool_call",
        at: "",
        title: "FileWrite started",
        body: "",
        meta: { name: "FileWrite" },
      },
      result: {
        id: "r1",
        block_type: "tool_result",
        at: "",
        title: "FileWrite finished",
        body: '{"success":true}',
        meta: { name: "FileWrite", path: "/tmp/mindmap-anycode-complex.md" },
      },
    });
    expect(path).toBe("/tmp/mindmap-anycode-complex.md");
  });
});

describe("collectInlineDeliverables", () => {
  it("infers mindmap card from FileWrite when marker is missing", () => {
    const assistant: TranscriptBlock = {
      id: "a1",
      block_type: "assistant_message",
      at: "2026-01-01T00:00:00Z",
      title: "",
      body: "anycode\n已写入 mindmap-anycode-complex.md。",
    };
    const byBlock = collectInlineDeliverables(
      [
        {
          kind: "tool_cluster",
          id: "tools-1",
          steps: [
            {
              key: "t1",
              call: {
                id: "c1",
                block_type: "tool_call",
                at: "",
                title: "FileWrite started",
                body: "",
                meta: { name: "FileWrite" },
              },
              result: {
                id: "r1",
                block_type: "tool_result",
                at: "",
                title: "FileWrite finished",
                body: "",
                meta: {
                  name: "FileWrite",
                  path: "/proj/mindmap-anycode-complex.md",
                },
              },
            },
          ],
          processMessageCount: 0,
          processSnippets: [],
        },
        { kind: "block", block: assistant },
      ],
      "project-1",
    );
    const cards = byBlock.get("a1") ?? [];
    expect(cards).toHaveLength(1);
    expect(cards[0]?.path).toBe("/proj/mindmap-anycode-complex.md");
    expect(cards[0]?.kind).toBe("mindmap");
    expect(cards[0]?.projectId).toBe("project-1");
  });

  it("shows only the file named in the final assistant message when multiple writes exist", () => {
    const assistant: TranscriptBlock = {
      id: "a1",
      block_type: "assistant_message",
      at: "2026-01-01T00:00:00Z",
      title: "",
      body: "已写入 mindmap-anycode-complex.md。",
    };
    const writeStep = (path: string) => ({
      key: path,
      call: {
        id: `c-${path}`,
        block_type: "tool_call" as const,
        at: "",
        title: "FileWrite started",
        body: "",
        meta: { name: "FileWrite" },
      },
      result: {
        id: `r-${path}`,
        block_type: "tool_result" as const,
        at: "",
        title: "FileWrite finished",
        body: "",
        meta: { name: "FileWrite", path },
      },
    });
    const byBlock = collectInlineDeliverables(
      [
        {
          kind: "tool_cluster",
          id: "tools-1",
          steps: [
            writeStep("/proj/mindmap-anycode-deep.md"),
            writeStep("/proj/mindmap-anycode-depth.md"),
            writeStep("/proj/mindmap-anycode-complex.md"),
          ],
          processMessageCount: 0,
          processSnippets: [],
        },
        { kind: "block", block: assistant },
      ],
      "project-1",
    );
    const cards = byBlock.get("a1") ?? [];
    expect(cards).toHaveLength(1);
    expect(cards[0]?.path).toBe("/proj/mindmap-anycode-complex.md");
  });

  it("does not surface plain process markdown notes as deliverable cards", () => {
    const assistant: TranscriptBlock = {
      id: "a1",
      block_type: "assistant_message",
      at: "2026-01-01T00:00:00Z",
      title: "",
      body: "已写入 docs/architecture.md 与 docs/roadmap.md。",
    };
    const writeStep = (path: string) => ({
      key: path,
      call: {
        id: `c-${path}`,
        block_type: "tool_call" as const,
        at: "",
        title: "FileWrite started",
        body: "",
        meta: { name: "FileWrite" },
      },
      result: {
        id: `r-${path}`,
        block_type: "tool_result" as const,
        at: "",
        title: "FileWrite finished",
        body: "",
        meta: { name: "FileWrite", path },
      },
    });
    const byBlock = collectInlineDeliverables(
      [
        {
          kind: "tool_cluster",
          id: "tools-1",
          steps: [
            writeStep("/proj/docs/architecture.md"),
            writeStep("/proj/docs/roadmap.md"),
          ],
          processMessageCount: 0,
          processSnippets: [],
        },
        { kind: "block", block: assistant },
      ],
      "project-1",
    );
    expect(byBlock.get("a1") ?? []).toHaveLength(0);
  });
});
