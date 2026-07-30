export type MarkdownTable = {
  headers: string[];
  rows: string[][];
  raw: string;
};

export type MarkdownSegment =
  | { type: "markdown"; content: string }
  | { type: "table"; table: MarkdownTable };

function splitRow(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

function isSeparatorRow(line: string): boolean {
  const trimmed = line.trim();
  if (!trimmed.startsWith("|")) return false;
  return trimmed
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .every((cell) => /^:?-{3,}:?$/.test(cell.trim()));
}

function parseTableBlock(lines: string[]): MarkdownTable | null {
  if (lines.length < 2) return null;
  const headers = splitRow(lines[0]!);
  const bodyStart = isSeparatorRow(lines[1]!) ? 2 : 1;
  const rows = lines.slice(bodyStart).map(splitRow);
  if (headers.length < 2 && rows.every((row) => row.length < 2)) return null;
  return {
    headers,
    rows,
    raw: lines.join("\n"),
  };
}

function scanPipeTableBlock(
  lines: string[],
  startIndex: number,
): { table: MarkdownTable | null; nextIndex: number; block: string[] } {
  const line = lines[startIndex];
  if (!line?.trim().startsWith("|") || !line.includes("|", 1)) {
    return { table: null, nextIndex: startIndex + 1, block: [] };
  }
  const block: string[] = [];
  let i = startIndex;
  while (i < lines.length) {
    const row = lines[i]!.trim();
    if (!row.startsWith("|")) break;
    block.push(lines[i]!);
    i += 1;
  }
  return { table: parseTableBlock(block), nextIndex: i, block };
}

/** @internal Test helper for pipe-table extraction. */
export function extractMarkdownTables(text: string): MarkdownTable[] {
  const tables: MarkdownTable[] = [];
  const lines = text.split("\n");
  let i = 0;
  while (i < lines.length) {
    const { table, nextIndex } = scanPipeTableBlock(lines, i);
    if (table) tables.push(table);
    i = nextIndex;
  }
  return tables;
}

export function isLargeMarkdownTable(table: MarkdownTable): boolean {
  const colCount = Math.max(table.headers.length, ...table.rows.map((row) => row.length));
  const rowCount = table.rows.length + (table.headers.length > 0 ? 1 : 0);
  if (rowCount >= 3 && colCount >= 3) return true;
  return table.raw.length > 480;
}

/** Split markdown into inline segments; large tables become standalone cards. */
export function splitMarkdownWithTables(text: string): MarkdownSegment[] {
  const segments: MarkdownSegment[] = [];
  const lines = text.split("\n");
  let buffer: string[] = [];
  let i = 0;

  const flushMarkdown = () => {
    if (buffer.length === 0) return;
    segments.push({ type: "markdown", content: buffer.join("\n") });
    buffer = [];
  };

  while (i < lines.length) {
    const { table, nextIndex, block } = scanPipeTableBlock(lines, i);
    if (block.length > 0) {
      if (table && isLargeMarkdownTable(table)) {
        flushMarkdown();
        segments.push({ type: "table", table });
      } else {
        buffer.push(...block);
      }
      i = nextIndex;
      continue;
    }
    buffer.push(lines[i]!);
    i += 1;
  }

  flushMarkdown();
  if (segments.length === 0) {
    return [{ type: "markdown", content: text }];
  }
  return segments;
}

export function tableToCsv(table: MarkdownTable): string {
  const rows = table.headers.length > 0 ? [table.headers, ...table.rows] : table.rows;
  return rows
    .map((row) =>
      row
        .map((cell) => {
          const escaped = cell.replace(/"/g, '""');
          return /[",\n]/.test(escaped) ? `"${escaped}"` : escaped;
        })
        .join(","),
    )
    .join("\n");
}
