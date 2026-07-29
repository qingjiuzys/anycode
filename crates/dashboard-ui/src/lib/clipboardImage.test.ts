import { describe, expect, it } from "vitest";
import { imageFilesFromPasteEvent, pastedImageFromBase64 } from "./clipboardImage";

describe("clipboardImage", () => {
  it("imageFilesFromPasteEvent collects image items", () => {
    const file = new File(["x"], "shot.png", { type: "image/png" });
    const event = {
      clipboardData: {
        items: [{ type: "image/png", getAsFile: () => file }],
      },
    } as unknown as ClipboardEvent;
    expect(imageFilesFromPasteEvent(event)).toHaveLength(1);
    expect(imageFilesFromPasteEvent(event)[0]?.type).toBe("image/png");
  });

  it("pastedImageFromBase64 defaults mime", () => {
    expect(pastedImageFromBase64(null, "abc").mime_type).toBe("image/png");
    expect(pastedImageFromBase64("image/jpeg", "abc").mime_type).toBe("image/jpeg");
  });
});
