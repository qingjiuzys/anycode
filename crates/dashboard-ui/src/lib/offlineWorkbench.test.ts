import { afterEach, describe, expect, it } from "vitest";
import {
  isOfflineWorkbenchAllowed,
  setOfflineWorkbenchAllowed,
} from "./offlineWorkbench";

describe("offlineWorkbench", () => {
  afterEach(() => {
    setOfflineWorkbenchAllowed(false);
  });

  it("defaults to disallowed", () => {
    expect(isOfflineWorkbenchAllowed()).toBe(false);
  });

  it("persists allow and clear", () => {
    setOfflineWorkbenchAllowed(true);
    expect(isOfflineWorkbenchAllowed()).toBe(true);
    setOfflineWorkbenchAllowed(false);
    expect(isOfflineWorkbenchAllowed()).toBe(false);
  });
});
