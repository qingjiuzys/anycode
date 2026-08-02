import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  clearComposerDraft,
  composerDraftKey,
  loadComposerDraft,
  saveComposerDraft,
} from "./composerDraft";

/** In-memory sessionStorage stand-in for the node test environment. */
function installSessionStorageMock() {
  const store = new Map<string, string>();
  const storage = {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => {
      store.set(key, String(value));
    },
    removeItem: (key: string) => {
      store.delete(key);
    },
    clear: () => store.clear(),
    get length() {
      return store.size;
    },
    key: (index: number) => [...store.keys()][index] ?? null,
  } as Storage;
  Object.defineProperty(globalThis, "sessionStorage", {
    value: storage,
    configurable: true,
    writable: true,
  });
  return storage;
}

let sessionStorageMock: Storage;

beforeEach(() => {
  sessionStorageMock = installSessionStorageMock();
});

afterEach(() => {
  sessionStorageMock.clear();
});

describe("composerDraft", () => {
  it("keys drafts per session scope", () => {
    expect(composerDraftKey("s1")).toBe("anycode.composer.draft:s1");
    expect(composerDraftKey("project:p1")).toBe("anycode.composer.draft:project:p1");
    expect(composerDraftKey(undefined)).toBeNull();
    expect(composerDraftKey("")).toBeNull();
  });

  it("restores a previously saved draft", () => {
    saveComposerDraft("s1", "请先审阅代码");
    expect(loadComposerDraft("s1")).toBe("请先审阅代码");
  });

  it("returns empty when no draft exists", () => {
    expect(loadComposerDraft("s-missing")).toBe("");
  });

  it("keeps drafts of different sessions isolated", () => {
    saveComposerDraft("s1", "draft one");
    saveComposerDraft("project:p1", "project draft");
    expect(loadComposerDraft("s1")).toBe("draft one");
    expect(loadComposerDraft("project:p1")).toBe("project draft");
    expect(loadComposerDraft("s2")).toBe("");
  });

  it("removes the key when saving an empty/whitespace draft", () => {
    saveComposerDraft("s1", "something");
    saveComposerDraft("s1", "   ");
    expect(loadComposerDraft("s1")).toBe("");
    expect(sessionStorage.getItem("anycode.composer.draft:s1")).toBeNull();
  });

  it("clears a draft on demand", () => {
    saveComposerDraft("s1", "keep or discard");
    clearComposerDraft("s1");
    expect(loadComposerDraft("s1")).toBe("");
    clearComposerDraft("s1");
  });
});