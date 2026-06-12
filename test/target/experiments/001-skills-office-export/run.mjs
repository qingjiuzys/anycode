import fs from "node:fs";
import path from "node:path";
import { addArtifact, assert, guarded, pass } from "../../shared/utils/assert.mjs";
import { anycodeBin, copyFixture, parseCsv, read, run, targetRoot, toCsv, write } from "../../shared/utils/files.mjs";

const id = "001-skills-office-export";
const outDir = path.join(targetRoot, "experiments", id, "out");

function salesReportMarkdown() {
  const rows = parseCsv(read(path.join(targetRoot, "shared/fixtures/sales_june.csv")));
  const header = rows.shift();
  const idx = Object.fromEntries(header.map((h, i) => [h, i]));
  const totals = { sales: 0, orders: 0, refunds: 0, regions: new Map() };
  for (const row of rows) {
    const region = row[idx.region];
    const sales = Number(row[idx.sales]);
    const orders = Number(row[idx.orders]);
    const refunds = Number(row[idx.refunds]);
    totals.sales += sales;
    totals.orders += orders;
    totals.refunds += refunds;
    const r = totals.regions.get(region) ?? { sales: 0, orders: 0, refunds: 0 };
    r.sales += sales;
    r.orders += orders;
    r.refunds += refunds;
    totals.regions.set(region, r);
  }
  const regionRows = Array.from(totals.regions.entries())
    .map(([region, r]) => ({ region, ...r, refundRate: r.refunds / r.orders }))
    .sort((a, b) => b.sales - a.sales);
  return [
    "# 六月销售日报分析",
    "",
    `- 总销售额：${totals.sales}`,
    `- 总订单数：${totals.orders}`,
    `- 总退款数：${totals.refunds}`,
    `- 退款率：${((totals.refunds / totals.orders) * 100).toFixed(2)}%`,
    "",
    "| 排名 | 区域 | 销售额 | 订单数 | 退款数 | 退款率 |",
    "| ---: | --- | ---: | ---: | ---: | ---: |",
    ...regionRows.map((r, i) => `| ${i + 1} | ${r.region} | ${r.sales} | ${r.orders} | ${r.refunds} | ${(r.refundRate * 100).toFixed(2)}% |`),
    "",
    "## 异常点",
    "",
    "- 华北退款率 6.65%，明显高于 3% 风险线，应优先复盘渠道 A 的履约和售后原因。",
    "",
  ].join("\n");
}

await guarded(id, outDir, async (result) => {
  fs.rmSync(outDir, { recursive: true, force: true });
  fs.mkdirSync(outDir, { recursive: true });
  const skillsList = run(anycodeBin, ["skills", "list"]);
  assert(result, skillsList.includes("report-to-csv"), "skills list includes report-to-csv");
  assert(result, skillsList.includes("md-to-pdf"), "skills list includes md-to-pdf");
  run(anycodeBin, ["skills", "vet", "md-to-pdf"]);
  pass(result, "skills vet md-to-pdf passed");

  const reportTable = copyFixture("report_table.md", outDir);
  const reportMd = path.join(outDir, "sales_daily_analysis.md");
  write(reportMd, salesReportMarkdown());

  const csvOut = path.join(outDir, "report_table.csv");
  const pdfOut = path.join(outDir, "sales_daily_analysis.pdf");
  run(path.join(process.env.HOME, ".anycode/skills/report-to-csv/run"), [reportTable, csvOut]);
  run(path.join(process.env.HOME, ".anycode/skills/md-to-pdf/run"), [reportMd, pdfOut]);

  const csv = read(csvOut);
  assert(result, csv.includes("区域,销售额,订单数,退款数,退款率"), "csv header matches");
  assert(result, fs.statSync(pdfOut).size > 10_000, "pdf larger than 10KB", { bytes: fs.statSync(pdfOut).size });
  const fileType = run("file", [pdfOut]);
  assert(result, fileType.includes("PDF document"), "pdf file type", { fileType: fileType.trim() });
  addArtifact(result, csvOut, "report_table.csv");
  addArtifact(result, pdfOut, "sales_daily_analysis.pdf");
  write(path.join(outDir, "summary.csv"), toCsv([["artifact", "status"], ["report_table.csv", "PASS"], ["sales_daily_analysis.pdf", "PASS"]]));
});
