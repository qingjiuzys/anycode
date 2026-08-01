import { describe, expect, it } from "vitest";
import { connectorPreviewErrorKey } from "./connectorErrors";

describe("connectorPreviewErrorKey", () => {
  it("maps github repo format errors", () => {
    expect(
      connectorPreviewErrorKey("github", 'expected owner/repo, got "not-a-repo"'),
    ).toBe("settings.connectorErrGithubRepo");
  });

  it("maps linear api key errors", () => {
    expect(connectorPreviewErrorKey("linear", "Linear API key required")).toBe(
      "settings.connectorErrLinearKey",
    );
  });

  it("falls back to generic keys", () => {
    expect(connectorPreviewErrorKey("github", "timeout")).toBe(
      "settings.connectorErrGithubGeneric",
    );
    expect(connectorPreviewErrorKey("linear", "timeout")).toBe(
      "settings.connectorErrLinearGeneric",
    );
  });
});
