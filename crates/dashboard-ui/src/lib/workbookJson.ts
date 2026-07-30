type WorkbookSheet = {
  name?: string;
  rows?: unknown[];
};

type WorkbookJson = {
  title?: string;
  sheets?: WorkbookSheet[];
};

function isWorkbookJson(value: unknown): value is WorkbookJson {
  if (!value || typeof value !== "object") return false;
  const sheets = (value as WorkbookJson).sheets;
  return Array.isArray(sheets) && sheets.length > 0;
}

function normalizeRows(rows: unknown[]): string[][] {
  return rows
    .filter(Array.isArray)
    .map((row) => (row as unknown[]).map((cell) => String(cell ?? "")));
}

export function parseWorkbookJson(text: string): {
  title?: string;
  sheets: Array<{ name: string; headers: string[]; rows: string[][] }>;
} | null {
  try {
    const parsed = JSON.parse(text) as unknown;
    if (!isWorkbookJson(parsed)) return null;
    const sheets = (parsed.sheets ?? []).map((sheet, index) => {
      const rows = normalizeRows(Array.isArray(sheet.rows) ? sheet.rows : []);
      const headers = rows.length > 0 ? rows[0]! : [];
      const body = rows.slice(1);
      return {
        name: sheet.name?.trim() || `Sheet ${index + 1}`,
        headers,
        rows: body,
      };
    });
    return { title: parsed.title, sheets };
  } catch {
    return null;
  }
}
