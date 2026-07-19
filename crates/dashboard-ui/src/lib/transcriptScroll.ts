import type { TranscriptBlock } from "@/api/types";

export const SCROLL_NEAR_BOTTOM_PX = 120;
/** Minimum ms between ResizeObserver-driven scroll follows during streaming. */
export const SCROLL_RESIZE_THROTTLE_MS = 120;

export function isScrollNearBottom(
  container: Pick<HTMLElement, "scrollHeight" | "scrollTop" | "clientHeight">,
  thresholdPx = SCROLL_NEAR_BOTTOM_PX,
): boolean {
  const distance = container.scrollHeight - container.scrollTop - container.clientHeight;
  return distance < thresholdPx;
}

/** Skip redundant scrollTo when already pinned to bottom (reduces jitter). */
export function shouldSkipScrollToBottom(
  container: Pick<HTMLElement, "scrollHeight" | "scrollTop" | "clientHeight">,
): boolean {
  const targetTop = container.scrollHeight - container.clientHeight;
  if (targetTop <= 0) {
    return true;
  }
  const distance = targetTop - container.scrollTop;
  return distance >= 0 && distance < 4;
}

export function lastAssistantBodyLength(blocks: TranscriptBlock[]): number {
  for (let i = blocks.length - 1; i >= 0; i -= 1) {
    const block = blocks[i];
    if (block.block_type === "assistant_message") {
      return block.body.length;
    }
  }
  return 0;
}

/** Signature for structural stream changes (not raw SSE text length — scroll follows visible DOM). */
export function streamFollowSignature(input: {
  running: boolean;
  streamLive: boolean;
  blocksLength: number;
  liveEventsLength: number;
  turnHasActivity: boolean;
  turnPhase?: string | null;
  liveBlocksLength: number;
}): string {
  if (!input.running && !input.streamLive) return "";
  return [
    input.blocksLength,
    input.liveEventsLength,
    input.turnHasActivity ? 1 : 0,
    input.turnPhase ?? "",
    input.liveBlocksLength,
  ].join("|");
}
