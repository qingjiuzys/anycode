/** Detect LLM task-summary / completion receipt markdown. */
export function isTaskSummaryReceipt(text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed) return false;
  if (/完成回执/.test(trimmed)) return true;
  return /\*\*(已完成|关键步骤|失败原因)[：:]?\*\*[：:]?/.test(trimmed);
}

/** Strip leading receipt heading so the card title can own it. */
export function stripTaskReceiptHeading(text: string): string {
  return text
    .replace(/^\s*#{1,3}\s*完成回执\s*\n+/m, "")
    .replace(/^\s*\*\*完成回执\*\*\s*\n+/m, "")
    .trim();
}
