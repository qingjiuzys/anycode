import { describe, expect, it } from "vitest";
import { CLOUD_PLATFORM, cloudLoginUrl } from "@/lib/cloudPlatform";
import { resolvePortalUrl } from "@/api/client/accountCloud";
import { SITE_ORIGIN, legalUrls, siteUrl } from "@anycode/site-urls";

describe("cloudPlatform", () => {
  it("defaults portal to SITE_ORIGIN", () => {
    expect(CLOUD_PLATFORM.portalUrl).toBe(SITE_ORIGIN);
    expect(cloudLoginUrl()).toBe(siteUrl("login"));
  });

  it("resolvePortalUrl falls back to SITE_ORIGIN", () => {
    expect(resolvePortalUrl(null)).toBe(SITE_ORIGIN);
  });
});

describe("siteUrls", () => {
  it("builds legal absolute URLs from one origin", () => {
    expect(legalUrls.userAgreement()).toBe(`${SITE_ORIGIN}/legal/user-agreement`);
    expect(legalUrls.privacy()).toBe(`${SITE_ORIGIN}/legal/privacy`);
  });
});
