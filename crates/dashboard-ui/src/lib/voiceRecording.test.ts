import { describe, expect, it } from "vitest";
import { encodeWav16kMono, extensionForMime } from "./voiceRecording";

describe("voiceRecording", () => {
  it("extensionForMime maps types", () => {
    expect(extensionForMime("audio/webm")).toBe("webm");
    expect(extensionForMime("audio/ogg")).toBe("ogg");
    expect(extensionForMime("audio/wav")).toBe("wav");
  });

  it("encodeWav16kMono produces RIFF WAVE header", async () => {
    const samples = new Float32Array(16);
    const blob = encodeWav16kMono(samples);
    const buf = await blob.arrayBuffer();
    const bytes = new Uint8Array(buf);
    expect(String.fromCharCode(...bytes.slice(0, 4))).toBe("RIFF");
    expect(String.fromCharCode(...bytes.slice(8, 12))).toBe("WAVE");
    expect(blob.size).toBe(44 + 16 * 2);
  });
});
