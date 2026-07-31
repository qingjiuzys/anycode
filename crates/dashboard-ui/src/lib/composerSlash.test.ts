import { describe, expect, it } from "vitest";
import { parseComposerSlashInput } from "./composerSlash";

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
