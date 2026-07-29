import type { CaseKind } from "../lib/cases";

/** Compact, overflow-safe preview tiles for case cards. */
export function CaseThumb({ kind }: { kind: CaseKind }) {
  if (kind === "ppt") {
    return (
      <div className="nx-case-thumb nx-case-thumb--ppt" aria-hidden>
        <span>02</span>
        <b>…</b>
        <i />
        <i />
        <i />
      </div>
    );
  }
  if (kind === "doc") {
    return (
      <div className="nx-case-thumb nx-case-thumb--doc" aria-hidden>
        <span>DOCX</span>
        <b />
        <b />
        <b />
      </div>
    );
  }
  return (
    <div className="nx-case-thumb nx-case-thumb--sheet" aria-hidden>
      <span>XLSX</span>
      <div>
        <i />
        <i />
        <i />
        <i />
      </div>
    </div>
  );
}
