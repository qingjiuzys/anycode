const ENGLISH_META_STARTERS = [
  /^The user (is|was|wants|asked|asked me to)\b/i,
  /^Let me\b/i,
  /^Now let me\b/i,
  /^Now I'll\b/i,
  /^I(?:'m| am) going to\b/i,
  /^Looking at\b/i,
  /^I (need to|should|will)\b/i,
  /^I'll\b/i,
  /^Actually,\b/i,
  /^Now I have\b/i,
  /^Based on (my|the)\b/i,
  /^I(?:'ve| have) (?:a good )?understanding\b/i,
  /^This seems like\b/i,
  /^The WeChat message\b/i,
  /^Done\.\s*$/i,
  /^Key findings\b/i,
  /^Summary\b/i,
  /^Next steps\b/i,
];

const ENGLISH_TAIL_STARTERS = [
  /^Now I have\b/i,
  /^Now let me\b/i,
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
  /^Key findings\b/i,
  /^Summary\b/i,
];

const CHINESE_SCAFFOLD_STARTERS = [
  /^关键发现/,
  /^总结/,
  /^下一步/,
  /^让我/,
  /^现在我/,
  /^基于/,
  /^用户(想|要|问|希望)/,
];

const CHINESE_TAIL_STARTERS = [
  /^关键发现/,
  /^总结/,
  /^下一步/,
  /^让我/,
  /^现在我/,
  /^基于/,
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

function isPureEnglishParagraph(paragraph: string): boolean {
  const t = paragraph.trim();
  if (!t) {
    return false;
  }
  return !hasCjkOutsideQuotes(t);
}

function isEnglishMetaParagraph(paragraph: string): boolean {
  const t = paragraph.trim();
  if (!t || hasCjkOutsideQuotes(t)) {
    return false;
  }
  return ENGLISH_META_STARTERS.some((re) => re.test(t));
}

function isChineseScaffoldParagraph(paragraph: string): boolean {
  const t = paragraph.trim();
  if (!t || !hasCjkOutsideQuotes(t)) {
    return false;
  }
  return CHINESE_SCAFFOLD_STARTERS.some((re) => re.test(t));
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

/** Drop pure-English paragraphs that precede the first Chinese paragraph. */
function stripLeadingEnglishScaffold(text: string): string {
  const parts = text.trim().split(/\n\n+/);
  const firstCjkIdx = parts.findIndex((p) => hasCjkOutsideQuotes(p.trim()));
  if (firstCjkIdx <= 0) {
    return text;
  }
  let start = 0;
  while (start < firstCjkIdx && isPureEnglishParagraph(parts[start]!)) {
    start += 1;
  }
  if (start === 0) {
    return text;
  }
  return parts.slice(start).join("\n\n").trim();
}

/** Drop pure-Chinese scaffold paragraphs that precede the first English paragraph. */
function stripLeadingChineseScaffold(text: string): string {
  const parts = text.trim().split(/\n\n+/);
  const firstEnglishIdx = parts.findIndex((p) => isPureEnglishParagraph(p));
  if (firstEnglishIdx <= 0) {
    return text;
  }
  let start = 0;
  while (start < firstEnglishIdx && isChineseScaffoldParagraph(parts[start]!)) {
    start += 1;
  }
  if (start === 0) {
    return text;
  }
  return parts.slice(start).join("\n\n").trim();
}

/** Trim English preamble in a single mixed paragraph before the first CJK run. */
function stripMixedEnglishPrefixInParagraph(text: string): string {
  const match = text.match(/^[\s\S]*?(?=[\u4e00-\u9fff])/);
  if (!match) {
    return text;
  }
  const prefix = match[0]!.trim();
  if (!prefix || hasCjkOutsideQuotes(prefix)) {
    return text;
  }
  if (
    !isEnglishMetaParagraph(prefix) &&
    !/^Key findings\b/i.test(prefix) &&
    !/^Let me\b/i.test(prefix) &&
    !/^Now let me\b/i.test(prefix)
  ) {
    return text;
  }
  return text.slice(match[0]!.length).trimStart();
}

function stripMixedParagraphPrefixes(text: string, locale: string): string {
  if (!locale.startsWith("zh")) {
    return text;
  }
  const parts = text.split(/\n\n+/);
  if (parts.length === 0) {
    return text;
  }
  const first = stripMixedEnglishPrefixInParagraph(parts[0]!);
  if (first === parts[0]) {
    return text;
  }
  parts[0] = first;
  return parts.join("\n\n").trim();
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

/** Drop trailing Chinese summary paragraphs when the message already contains English. */
export function stripTrailingChineseTail(text: string, locale: string): string {
  if (!locale.startsWith("en")) {
    return text;
  }
  const trimmed = text.trimEnd();
  if (hasCjk(trimmed) && !/[a-zA-Z]{3,}/.test(trimmed)) {
    return text;
  }

  const parts = trimmed.split(/\n\n+/);
  let cut = parts.length;
  while (cut > 1) {
    const p = parts[cut - 1]!.trim();
    if (!p || !hasCjkOutsideQuotes(p)) {
      break;
    }
    const earlierHasEnglish = parts
      .slice(0, cut - 1)
      .some((x) => /[a-zA-Z]{3,}/.test(x));
    if (!earlierHasEnglish) {
      break;
    }
    const isKnownTail = CHINESE_TAIL_STARTERS.some((re) => re.test(p));
    if (!isKnownTail && p.length < 80) {
      break;
    }
    cut -= 1;
  }
  const kept = parts.slice(0, cut).join("\n\n").trimEnd();
  return kept.length > 0 ? kept : text;
}

function sanitizeForZh(text: string): string {
  const trimmed = text.trim();
  if (!trimmed) {
    return "";
  }

  if (isEnglishMetaParagraph(trimmed)) {
    return "";
  }

  let out = stripLeadingEnglishMeta(trimmed);
  if (!out) {
    return "";
  }
  out = stripLeadingEnglishScaffold(out);
  if (!out) {
    return "";
  }
  out = stripMixedParagraphPrefixes(out, "zh");
  return stripTrailingEnglishTail(out, "zh");
}

function sanitizeForEn(text: string): string {
  const trimmed = text.trim();
  if (!trimmed) {
    return "";
  }

  if (isChineseScaffoldParagraph(trimmed) && !/[a-zA-Z]{3,}/.test(trimmed)) {
    return "";
  }

  let out = stripLeadingChineseScaffold(trimmed);
  if (!out) {
    return "";
  }
  return stripTrailingChineseTail(out, "en");
}

import { stripArtifactMarkers, isArtifactScaffoldOnly } from "@/lib/artifactMarker";

function stripLeadingProductEcho(text: string): string {
  return text.replace(/^anycode\s*\r?\n/i, "").trimStart();
}

/** Sanitize assistant text for the active UI locale (symmetric zh/en scaffold removal). */
export function sanitizeAssistantDisplay(text: string, locale: string): string {
  const normalized = normalizeDecorativeRules(
    stripLeadingProductEcho(stripArtifactMarkers(text)),
  );
  let result: string;
  if (locale.startsWith("zh")) {
    result = sanitizeForZh(normalized);
  } else if (locale.startsWith("en")) {
    result = sanitizeForEn(normalized);
  } else {
    result = normalized;
  }
  if (isArtifactScaffoldOnly(result)) {
    return /^anycode$/i.test(result.trim()) ? "anycode" : "";
  }
  return result;
}

/** Turn lone *** / --- lines into blank lines so GFM doesn't leave raw asterisks. */
function normalizeDecorativeRules(text: string): string {
  return text.replace(/^[ \t]*(\*{3,}|-{3,}|_{3,})[ \t]*$/gm, "");
}
