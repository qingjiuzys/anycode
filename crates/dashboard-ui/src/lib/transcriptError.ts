const BARE_FIELD = /^(path|command|query|duration_ms)$/i;
const MISSING_FIELD_RE = /missing field ['"]?(\w+)['"]?/i;
const TOOL_PARAM_RE = /^Tool parameter error: missing or invalid `(\w+)`$/i;

function firstLine(text: string): string {
  const first = text.split("\n").find((line) => line.trim())?.trim() ?? text.trim();
  if (first.length > 220) {
    return `${first.slice(0, 220)}…`;
  }
  return first;
}

/** Map cryptic transcript error bodies to user-facing summaries. */
export function humanizeTranscriptError(
  body: string,
  formatToolField: (field: string) => string,
  formatMissingField: (field: string) => string,
): { summary: string; raw: string } {
  const raw = body.trim();
  if (!raw) {
    return { summary: raw, raw };
  }

  const toolParam = raw.match(TOOL_PARAM_RE);
  if (toolParam) {
    return { summary: formatToolField(toolParam[1]!), raw };
  }
  if (BARE_FIELD.test(raw)) {
    return { summary: formatToolField(raw), raw };
  }
  const missing = raw.match(MISSING_FIELD_RE);
  if (missing) {
    return { summary: formatMissingField(missing[1]!), raw };
  }
  return { summary: firstLine(raw), raw };
}
