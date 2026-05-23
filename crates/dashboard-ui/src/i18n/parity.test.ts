import { describe, expect, it } from "vitest";
import { en } from "./en";
import { zh } from "./zh";

function flattenKeys(obj: Record<string, unknown>, prefix = ""): string[] {
  const keys: string[] = [];
  for (const [k, v] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object" && !Array.isArray(v)) {
      keys.push(...flattenKeys(v as Record<string, unknown>, path));
    } else {
      keys.push(path);
    }
  }
  return keys.sort();
}

describe("i18n parity", () => {
  it("zh has the same keys as en", () => {
    const enKeys = flattenKeys(en as unknown as Record<string, unknown>);
    const zhKeys = flattenKeys(zh as unknown as Record<string, unknown>);
    const missingInZh = enKeys.filter((k) => !zhKeys.includes(k));
    const extraInZh = zhKeys.filter((k) => !enKeys.includes(k));
    expect(missingInZh, `missing in zh: ${missingInZh.join(", ")}`).toEqual([]);
    expect(extraInZh, `extra in zh: ${extraInZh.join(", ")}`).toEqual([]);
  });
});
