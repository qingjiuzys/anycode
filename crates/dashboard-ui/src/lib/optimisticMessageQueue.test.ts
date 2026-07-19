import { describe, expect, it } from "vitest";
import {
  mergeQueueItems,
  nextOptimisticSeq,
  removeOptimisticId,
  replaceOptimisticId,
} from "./optimisticMessageQueue";

describe("optimisticMessageQueue", () => {
  it("merges server and optimistic items without duplicates", () => {
    const merged = mergeQueueItems(
      [{ id: "mq_1", prompt: "hello", seq: 1 }],
      [{ id: "opt-1", prompt: "world", seq: 2 }],
    );
    expect(merged).toHaveLength(2);
    expect(merged[1]?.prompt).toBe("world");
  });

  it("replaces optimistic id with server id", () => {
    const next = replaceOptimisticId(
      [{ id: "opt-1", prompt: "hi", seq: 1 }],
      "opt-1",
      "mq_real",
      1,
    );
    expect(next[0]?.id).toBe("mq_real");
  });

  it("computes next seq", () => {
    expect(nextOptimisticSeq([{ id: "a", prompt: "x", seq: 2 }])).toBe(3);
    expect(removeOptimisticId([{ id: "a", prompt: "x", seq: 1 }], "a")).toEqual([]);
  });
});
