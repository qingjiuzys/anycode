import { GRILL_COMPOSER_MODE, isGrillSlashToken } from "./grillMode";
import { GOAL_AGENT_ID, isGoalSlashToken } from "./goalMode";

export type ComposerSlashMode = typeof GRILL_COMPOSER_MODE | typeof GOAL_AGENT_ID;

export type ComposerSlashParse = {
  mode: ComposerSlashMode | null;
  /** User text after the slash token (trimmed). */
  prompt: string;
  /** Input was only `/拷问` or `/目标` with no trailing text. */
  bareSlash: boolean;
};

/** Parse a leading `/拷问` or `/目标` (and English aliases) out of the composer text. */
export function parseComposerSlashInput(text: string): ComposerSlashParse {
  const trimmed = text.trim();
  if (!trimmed.startsWith("/")) {
    return { mode: null, prompt: trimmed, bareSlash: false };
  }

  const body = trimmed.slice(1).trimStart();
  if (!body) {
    return { mode: null, prompt: "", bareSlash: false };
  }

  const match = body.match(/^(\S+)(?:\s+([\s\S]*))?$/);
  if (!match) {
    return { mode: null, prompt: trimmed, bareSlash: false };
  }

  const token = match[1] ?? "";
  const remainder = (match[2] ?? "").trim();

  if (isGrillSlashToken(token)) {
    return {
      mode: GRILL_COMPOSER_MODE,
      prompt: remainder,
      bareSlash: remainder.length === 0,
    };
  }
  if (isGoalSlashToken(token)) {
    return {
      mode: GOAL_AGENT_ID,
      prompt: remainder,
      bareSlash: remainder.length === 0,
    };
  }

  return { mode: null, prompt: trimmed, bareSlash: false };
}
