export type OptimisticQueueItem = {
  id: string;
  prompt: string;
  seq: number;
};

export function mergeQueueItems(
  serverItems: { id: string; prompt: string; seq: number }[],
  optimisticItems: OptimisticQueueItem[],
): OptimisticQueueItem[] {
  const serverIds = new Set(serverItems.map((i) => i.id));
  const merged: OptimisticQueueItem[] = serverItems.map((i) => ({
    id: i.id,
    prompt: i.prompt,
    seq: i.seq,
  }));
  for (const item of optimisticItems) {
    if (item.id.startsWith("opt-") && !serverIds.has(item.id)) {
      const duplicate = merged.some(
        (m) => m.prompt === item.prompt && Math.abs(m.seq - item.seq) <= 1,
      );
      if (!duplicate) {
        merged.push(item);
      }
    }
  }
  return merged.sort((a, b) => a.seq - b.seq);
}

export function nextOptimisticSeq(items: OptimisticQueueItem[]): number {
  if (items.length === 0) return 1;
  return Math.max(...items.map((i) => i.seq)) + 1;
}

export function replaceOptimisticId(
  items: OptimisticQueueItem[],
  tempId: string,
  serverId: string,
  seq: number,
): OptimisticQueueItem[] {
  return items.map((item) =>
    item.id === tempId ? { ...item, id: serverId, seq } : item,
  );
}

export function removeOptimisticId(
  items: OptimisticQueueItem[],
  id: string,
): OptimisticQueueItem[] {
  return items.filter((item) => item.id !== id);
}
