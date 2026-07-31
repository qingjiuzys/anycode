export type TextAttachment = {
  filename: string;
  content: string;
};

/** Paste longer than this becomes a reference card instead of inline textarea text. */
export const PASTE_AS_CARD_MIN_CHARS = 4000;
export const MAX_TEXT_FILES = 3;
export const MAX_TEXT_FILE_BYTES = 1024 * 1024;

export function plainTextFromPasteEvent(event: ClipboardEvent): string {
  return event.clipboardData?.getData("text/plain") ?? "";
}

export function shouldPasteAsTextCard(text: string): boolean {
  return text.length > PASTE_AS_CARD_MIN_CHARS;
}

export function utf8ByteLength(text: string): number {
  return new TextEncoder().encode(text).length;
}

export function makePastedTextAttachment(
  content: string,
  existingFilenames: string[],
): TextAttachment {
  let n = existingFilenames.length + 1;
  let filename = `paste-${n}.txt`;
  const used = new Set(existingFilenames);
  while (used.has(filename)) {
    n += 1;
    filename = `paste-${n}.txt`;
  }
  return { filename, content };
}

export function textPayloadsForApi(
  files: TextAttachment[],
): { filename: string; content: string }[] {
  return files.map(({ filename, content }) => ({ filename, content }));
}

export function formatTextAttachmentMeta(content: string, locale: string): string {
  const n = content.length;
  try {
    return new Intl.NumberFormat(locale === "zh" ? "zh-CN" : "en-US").format(n);
  } catch {
    return String(n);
  }
}
