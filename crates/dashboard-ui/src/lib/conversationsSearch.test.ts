import { describe, expect, it } from "vitest";
import {
  buildConversationsHref,
  conversationSearchParams,
  conversationsCanonicalHref,
  parseFilterFromSearchStr,
} from "./conversationsSearch";

describe("parseFilterFromSearchStr", () => {
  it("prefers filter param", () => {
    expect(parseFilterFromSearchStr("?filter=blocked&needs_approval=true")).toBe("blocked");
  });

  it("infers needs_approval from legacy params", () => {
    expect(parseFilterFromSearchStr("?status=running&needs_approval=true")).toBe("needs_approval");
  });

  it("infers blocked when no needs_approval", () => {
    expect(parseFilterFromSearchStr("?trusted=blocked")).toBe("blocked");
  });
});

describe("conversationSearchParams", () => {
  it("drops legacy keys when filter is set", () => {
    expect(
      conversationSearchParams({
        filter: "needs_approval",
        status: "running",
        needs_approval: true,
        trusted: "blocked",
      }),
    ).toEqual({ filter: "needs_approval" });
  });
});

describe("buildConversationsHref", () => {
  it("uses only filter in URL", () => {
    expect(buildConversationsHref({ filter: "needs_approval" })).toBe(
      "/conversations?filter=needs_approval",
    );
  });
});

describe("conversationsCanonicalHref", () => {
  it("returns null for clean filter URL", () => {
    expect(conversationsCanonicalHref("?filter=needs_approval")).toBeNull();
  });

  it("strips stale legacy params", () => {
    expect(conversationsCanonicalHref("?filter=needs_approval&trusted=blocked")).toBe(
      "/conversations?filter=needs_approval",
    );
  });

  it("rewrites legacy-only URLs", () => {
    expect(conversationsCanonicalHref("?status=running&needs_approval=true")).toBe(
      "/conversations?filter=needs_approval",
    );
  });
});
