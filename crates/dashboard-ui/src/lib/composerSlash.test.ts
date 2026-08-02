import { describe, expect, it } from "vitest";
import {
  composerSlashKeepText,
  parseComposerSlashInput,
  parseSlashQuery,
} from "./composerSlash";

describe("parseSlashQuery", () => {
  it("opens the menu on bare slash", () => {
    expect(parseSlashQuery("/")).toBe("");
  });

  it("filters by partial token", () => {
    expect(parseSlashQuery("/拷")).toBe("拷");
    expect(parseSlashQuery("/grill")).toBe("grill");
  });

  it("closes on multi-line or non-slash input", () => {
    expect(parseSlashQuery("hello")).toBeNull();
    expect(parseSlashQuery("/拷问\n更多")).toBeNull();
  });
});

describe("parseComposerSlashInput", () => {
  it("passes through normal text", () => {
    expect(parseComposerSlashInput("做个小程序")).toEqual({
      mode: null,
      prompt: "做个小程序",
      bareSlash: false,
    });
  });

  it("strips bare grill slash", () => {
    expect(parseComposerSlashInput("/拷问")).toEqual({
      mode: "grill",
      prompt: "",
      bareSlash: true,
    });
  });

  it("strips grill slash with trailing prompt", () => {
    expect(parseComposerSlashInput("/拷问  做个拼豆小程序")).toEqual({
      mode: "grill",
      prompt: "做个拼豆小程序",
      bareSlash: false,
    });
  });

  it("strips goal slash", () => {
    expect(parseComposerSlashInput("/目标 三个月上线")).toEqual({
      mode: "goal",
      prompt: "三个月上线",
      bareSlash: false,
    });
  });

  it("ignores unknown slash commands", () => {
    expect(parseComposerSlashInput("/help me")).toEqual({
      mode: null,
      prompt: "/help me",
      bareSlash: false,
    });
  });
});

describe("composerSlashKeepText", () => {
  it("keeps prompt when the target command is already active", () => {
    expect(composerSlashKeepText("/拷问", "/拷问 帮我分析下当前项目")).toBe(
      "帮我分析下当前项目",
    );
    expect(composerSlashKeepText("/grill-me", "/grill-me review this")).toBe(
      "review this",
    );
  });

  it("keeps existing text when switching to grill from a partial token", () => {
    expect(composerSlashKeepText("/拷问", "/拷")).toBe("");
    expect(composerSlashKeepText("/拷问", "/拷 保留这段")).toBe("保留这段");
  });

  it("keeps prompt when switching from another mode", () => {
    expect(composerSlashKeepText("/拷问", "/目标 三个月上线")).toBe("三个月上线");
    expect(composerSlashKeepText("/目标", "/拷问 帮我审阅代码")).toBe("帮我审阅代码");
  });

  it("keeps the full text for non-slash input", () => {
    expect(composerSlashKeepText("/拷问", "帮我分析下当前项目")).toBe(
      "帮我分析下当前项目",
    );
  });

  it("keeps goal prompt when target command is already goal", () => {
    expect(composerSlashKeepText("/目标", "/目标 三个月上线")).toBe("三个月上线");
  });
});
