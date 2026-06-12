const ENGLISH_META_STARTERS = [
  /^The user (is|was|wants|asked|asked me to)\b/i,
  /^Let me\b/i,
  /^I (need to|should|will)\b/i,
  /^I'll\b/i,
  /^Actually,\b/i,
  /^Now I have\b/i,
  /^Based on (my|the)\b/i,
  /^I(?:'ve| have) (?:a good )?understanding\b/i,
  /^This seems like\b/i,
  /^The WeChat message\b/i,
  /^Done\.\s*$/i,
];

const ENGLISH_TAIL_STARTERS = [
  /^Now I have\b/i,
  /^Based on (my|the)\b/i,
  /^Let me\b/i,
  /^I(?:'ve| have) (?:a good )?understanding\b/i,
  /^The user is\b/i,
  /^The user wants\b/i,
  /^The user asked\b/i,
  /^I'll\b/i,
  /^The WeChat message\b/i,
  /^This seems like\b/i,
  /^Done\.\s*$/i,
];

function hasCjk(text: string): boolean {
  return /[\u4e00-\u9fff]/.test(text);
}

function hasCjkOutsideQuotes(text: string): boolean {
  const withoutQuotes = text
    .replace(/"[^"]*"/g, "")
    .replace(/'[^']*'/g, "");
  return hasCjk(withoutQuotes);
}

function isEnglishMetaParagraph(paragraph: string): boolean {
  const t = paragraph.trim();
  if (!t || hasCjkOutsideQuotes(t)) {
    return false;
  }
  return ENGLISH_META_STARTERS.some((re) => re.test(t));
}

function stripLeadingEnglishMeta(text: string): string {
  const parts = text.trim().split(/\n\n+/);
  let start = 0;
  while (start < parts.length) {
    const p = parts[start]!.trim();
    if (!p || hasCjkOutsideQuotes(p)) {
      break;
    }
    if (!isEnglishMetaParagraph(p)) {
      break;
    }
    start += 1;
  }
  return parts.slice(start).join("\n\n").trim();
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
    if (!p || hasCjkOutsideQuotes(p)) {
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

/** Sanitize assistant text for zh UI: hide English meta narration and mixed-language clutter. */
export function sanitizeAssistantDisplay(text: string, locale: string): string {
  if (!locale.startsWith("zh")) {
    return text;
  }
  const trimmed = text.trim();
  if (!trimmed) {
    return text;
  }

  if (isEnglishMetaParagraph(trimmed)) {
    return "";
  }

  const withoutLeading = stripLeadingEnglishMeta(trimmed);
  if (!withoutLeading) {
    return "";
  }

  return stripTrailingEnglishTail(withoutLeading, locale);
}
