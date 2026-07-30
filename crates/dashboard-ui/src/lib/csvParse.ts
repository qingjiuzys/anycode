/** Minimal CSV row parser for preview tables (handles quoted fields). */
function parseCsvRow(line: string): string[] {
  const out: string[] = [];
  let cell = "";
  let inQuotes = false;
  for (let i = 0; i < line.length; i += 1) {
    const ch = line[i]!;
    if (inQuotes) {
      if (ch === '"') {
        if (line[i + 1] === '"') {
          cell += '"';
          i += 1;
        } else {
          inQuotes = false;
        }
      } else {
        cell += ch;
      }
      continue;
    }
    if (ch === '"') {
      inQuotes = true;
    } else if (ch === ",") {
      out.push(cell.trim());
      cell = "";
    } else {
      cell += ch;
    }
  }
  out.push(cell.trim());
  return out;
}

export type CsvPreview = {
  headers: string[];
  rows: string[][];
};

export function parseCsvPreview(text: string, maxRows = 8): CsvPreview {
  const lines = text
    .replace(/^\uFEFF/, "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .slice(0, maxRows + 1);
  if (lines.length === 0) {
    return { headers: [], rows: [] };
  }
  const headers = parseCsvRow(lines[0]!);
  const rows = lines.slice(1).map(parseCsvRow);
  return { headers, rows };
}
