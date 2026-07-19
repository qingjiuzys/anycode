import { readdirSync, statSync, readFileSync, existsSync } from "node:fs";
import { join, extname } from "node:path";

export function findNewest(root, ext) {
  if (!existsSync(root)) return null;
  let best = null;
  let bestMtime = 0;
  function walk(dir) {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      const st = statSync(p);
      if (st.isDirectory()) {
        walk(p);
        continue;
      }
      if (extname(p).toLowerCase() !== ext) continue;
      if (st.mtimeMs >= bestMtime) {
        bestMtime = st.mtimeMs;
        best = p;
      }
    }
  }
  walk(root);
  return best;
}

export function readText(path) {
  return readFileSync(path, "utf8");
}
