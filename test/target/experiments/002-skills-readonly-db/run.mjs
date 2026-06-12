import fs from "node:fs";
import path from "node:path";
import { addArtifact, assert, guarded } from "../../shared/utils/assert.mjs";
import { copyFixture, read, spawn, targetRoot, write } from "../../shared/utils/files.mjs";

const id = "002-skills-readonly-db";
const outDir = path.join(targetRoot, "experiments", id, "out");

await guarded(id, outDir, async (result) => {
  fs.rmSync(outDir, { recursive: true, force: true });
  fs.mkdirSync(outDir, { recursive: true });
  const seed = copyFixture("readonly_seed.sql", outDir);
  const db = path.join(outDir, "office_readonly.sqlite");
  spawn("sqlite3", [db, `.read ${seed}`]);
  fs.chmodSync(db, 0o444);
  const selectSql = "SELECT c.region, ROUND(SUM(o.amount),2) AS sales, ROUND(COALESCE(SUM(r.amount),0),2) AS refunds, ROUND(COALESCE(SUM(r.amount),0) / SUM(o.amount) * 100, 2) AS refund_rate_pct FROM orders o JOIN customers c ON c.id=o.customer_id LEFT JOIN refunds r ON r.order_id=o.id WHERE o.created_at BETWEEN '2026-06-01' AND '2026-06-30' GROUP BY c.region ORDER BY sales DESC;";
  const select = spawn("sqlite3", ["-header", "-csv", db, selectSql]);
  write(path.join(outDir, "readonly_db_select.csv"), select.stdout);
  const update = spawn("sqlite3", [db, "UPDATE customers SET region='上海' WHERE id=1;"]);
  fs.chmodSync(db, 0o644);

  const csv = read(path.join(outDir, "readonly_db_select.csv"));
  assert(result, select.status === 0, "select succeeds", { stderr: select.stderr.trim() });
  assert(result, csv.includes("region,sales,refunds,refund_rate_pct"), "select csv header");
  assert(result, update.status !== 0, "update fails", { exitCode: update.status });
  assert(result, update.stderr.includes("attempt to write a readonly database"), "update blocked by readonly database", { stderr: update.stderr.trim() });
  write(path.join(outDir, "readonly_db_guard.md"), `# 只读数据库测试\n\n## UPDATE\n\n- exit_code: ${update.status}\n- stderr: ${update.stderr.trim()}\n`);
  addArtifact(result, path.join(outDir, "readonly_db_select.csv"), "readonly_db_select.csv");
  addArtifact(result, path.join(outDir, "readonly_db_guard.md"), "readonly_db_guard.md");
});
