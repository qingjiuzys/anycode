import { describe, expect, it } from "vitest";
import { CLOUD_PLATFORM, cloudLoginUrl } from "@/lib/cloudPlatform";
import { resolvePortalUrl } from "@/api/client/accountCloud";

describe("cloudPlatform", () => {
  it("defaults portal to anycode.work", () => {
    expect(CLOUD_PLATFORM.portalUrl).toBe("https://anycode.work");
    expect(cloudLoginUrl()).toBe("https://anycode.work/login");
  });

  it("resolvePortalUrl falls back to anycode.work", () => {
    expect(resolvePortalUrl(null)).toBe("https://anycode.work");
  });
});
