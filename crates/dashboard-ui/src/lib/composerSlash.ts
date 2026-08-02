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

/**
 * Active slash autocomplete filter, or `null` when the menu should stay closed.
 * Bare `/` returns `""` (show all commands). Multi-line input disables the menu.
 */
export function parseSlashQuery(text: string): string | null {
  const trimmed = text.trimStart();
  if (!trimmed.startsWith("/") || trimmed.includes("\n")) return null;
  const body = trimmed.slice(1);
  const token = body.split(/\s+/)[0] ?? "";
  if (token === "") return "";
  if (/^[\w.-]+$/.test(token)) return token.toLowerCase();
  return token;
}

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

/**
 * Text that should remain in the composer after applying a slash command.
 * Aligns with Codex-style behavior: switching mode keeps existing input —
 * when the current text is already the target command, keep its prompt;
 * otherwise (partial token like `/拷`, or a different slash like `/目标 …`)
 * strip the leading slash token but never discard the rest of the text.
 */
export function composerSlashKeepText(cmd: string, text: string): string {
  const parsed = parseComposerSlashInput(text);
  const targetIsGrill = isGrillSlashToken(cmd);
  const targetIsGoal = isGoalSlashToken(cmd);
  const matchesTarget =
    (targetIsGrill && parsed.mode === GRILL_COMPOSER_MODE) ||
    (targetIsGoal && parsed.mode === GOAL_AGENT_ID);
  if (matchesTarget) return parsed.prompt;
  return text.replace(/^\s*\/\S*\s*/, "");
}
