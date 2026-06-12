const ENGLISH_TAIL_STARTERS = [
  /^Now I have\b/i,
  /^Based on (my|the)\b/i,
  /^Let me\b/i,
  /^I(?:'ve| have) (?:a good )?understanding\b/i,
  /^The user is\b/i,
  /^I'll\b/i,
];

function hasCjk(text: string): boolean {
  return /[\u4e00-\u9fff]/.test(text);
}

/** Drop trailing English summary paragraphs when the message already contains Chinese. */
export function stripTrailingEnglishTail(text: string, locale: string): string {
  if (!locale.startsWith("zh")) {
    return text;
  }
  const trimmed = text.trimEnd();
  if (!hasCjk(trimmed)) {
    return text;
  }

  const parts = trimmed.split(/\n\n+/);
  let cut = parts.length;
  while (cut > 1) {
    const p = parts[cut - 1]!.trim();
    if (!p || hasCjk(p)) {
      break;
    }
    const earlierHasCjk = parts.slice(0, cut - 1).some((x) => hasCjk(x));
    if (!earlierHasCjk) {
      break;
    }
    const isKnownTail = ENGLISH_TAIL_STARTERS.some((re) => re.test(p));
    if (!isKnownTail && p.length < 80) {
      break;
    }
    cut -= 1;
  }
  const kept = parts.slice(0, cut).join("\n\n").trimEnd();
  return kept.length > 0 ? kept : text;
}
