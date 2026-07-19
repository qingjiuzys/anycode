#!/usr/bin/env python3
"""Generate ~5000 lines of Rust handler code for ultra-long-context fixtures."""
from pathlib import Path

OUT = Path(__file__).parent / "src" / "handlers.rs"
OUT.parent.mkdir(parents=True, exist_ok=True)

lines = ["// Auto-generated large-repo fixture\n"]
for i in range(1, 501):
    lines.append(f"pub fn handler_{i}(x: i64) -> i64 {{ x + {i % 7} }}\n")
    lines.append(f"pub fn handler_{i}_aux(y: i64) -> i64 {{ y - {i % 3} }}\n")
    lines.append(f"#[cfg(test)] mod test_{i} {{ use super::*; #[test] fn t() {{ assert_eq!(handler_{i}(1), 1 + {i % 7}); }} }}\n")
    lines.append(f"// padding block {i}\n" + ("// line\n" * 6))
OUT.write_text("".join(lines), encoding="utf-8")
print(f"wrote {len(lines)} lines to {OUT}")
