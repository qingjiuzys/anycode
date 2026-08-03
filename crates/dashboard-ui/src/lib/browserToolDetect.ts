import type { TranscriptBlock } from "@/api/types";
import { toolStepKey } from "@/lib/transcriptGrouping";

/** Keep in sync with `BROWSER_TOOL_IDS` in crates/tools/src/browser_tools.rs */
export const BROWSER_TOOL_IDS = new Set([
  "BrowserTabs",
  "BrowserNavigate",
  "BrowserSnapshot",
  "BrowserClick",
  "BrowserType",
  "BrowserPressKey",
  "BrowserScroll",
  "BrowserScreenshot",
  "BrowserCdp",
]);

function toolNameFromBlock(block: TranscriptBlock): string | null {
  const metaName = block.meta?.name;
  if (typeof metaName === "string") {
    const trimmed = metaName.trim();
    if (BROWSER_TOOL_IDS.has(trimmed)) return trimmed;
  }
  const title = block.title?.trim() ?? "";
  const match = title.match(/^(Browser[A-Za-z]+)\b/);
  if (match && BROWSER_TOOL_IDS.has(match[1]!)) {
    return match[1]!;
  }
  return null;
}

/** True only for Browser* tool_call / tool_result blocks — never plain chat text. */
export function isBrowserToolBlock(block: TranscriptBlock): boolean {
  if (block.block_type !== "tool_call" && block.block_type !== "tool_result") {
    return false;
  }
  return toolNameFromBlock(block) !== null;
}

export function browserToolDedupeKey(block: TranscriptBlock): string {
  return toolStepKey(block) ?? block.id;
}

/** Auto-open only for a newly started live Browser tool call during an active stream. */
export function shouldAutoOpenBrowserForBlock(
  block: TranscriptBlock,
  opts: { streamLive: boolean },
): boolean {
  if (!opts.streamLive) return false;
  if (block.block_type !== "tool_call") return false;
  if (!isBrowserToolBlock(block)) return false;
  if (block.meta?.live === false) return false;
  return true;
}

export function collectBrowserToolCallKeys(blocks: TranscriptBlock[]): Set<string> {
  const keys = new Set<string>();
  for (const block of blocks) {
    if (block.block_type !== "tool_call") continue;
    if (!isBrowserToolBlock(block)) continue;
    keys.add(browserToolDedupeKey(block));
  }
  return keys;
}
