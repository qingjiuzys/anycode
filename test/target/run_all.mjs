import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { repoRoot, targetRoot, write } from "./shared/utils/files.mjs";

const experiments = [
  "001-skills-office-export",
  "002-skills-readonly-db",
  "003-web-workbench",
  "004-web-agents-skills",
  "005-macos-client-channels",
  "006-wechat-real-send",
];

const outDir = path.join(targetRoot, "out");
fs.mkdirSync(outDir, { recursive: true });

function runStep(label, cmd, args) {
  const started = new Date().toISOString();
  try {
    const stdout = execFileSync(cmd, args, {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return { label, status: "pass", started_at: started, finished_at: new Date().toISOString(), stdout };
  } catch (error) {
    return {
      label,
      status: "fail",
      started_at: started,
      finished_at: new Date().toISOString(),
      stdout: error.stdout?.toString() ?? "",
      stderr: error.stderr?.toString() ?? error.message,
    };
  }
}

const build = process.env.ANYCODE_E2E_SKIP_BUILD === "1"
  ? { label: "cargo build -p anycode", status: "skipped", started_at: new Date().toISOString(), finished_at: new Date().toISOString() }
  : runStep("cargo build -p anycode", "cargo", ["build", "-p", "anycode"]);

const results = [];
if (build.status === "fail") {
  results.push({
    id: "build",
    status: "fail",
    started_at: build.started_at,
    finished_at: build.finished_at,
    artifacts: [],
    assertions: [{ name: build.label, status: "fail", message: build.stderr }],
    errors: [{ name: build.label, message: build.stderr }],
  });
} else {
  for (const id of experiments) {
    const script = path.join(targetRoot, "experiments", id, "run.mjs");
    const step = runStep(id, "node", [script]);
    const resultPath = path.join(targetRoot, "experiments", id, "out", "result.json");
    if (fs.existsSync(resultPath)) {
      results.push(JSON.parse(fs.readFileSync(resultPath, "utf8")));
    } else {
      results.push({
        id,
        status: "fail",
        started_at: step.started_at,
        finished_at: step.finished_at,
        artifacts: [],
        assertions: [{ name: "result.json exists", status: "fail", message: step.stderr }],
        errors: [{ name: "missing result.json", message: step.stderr }],
      });
    }
  }
}

const summary = {
  status: results.every((r) => r.status === "pass") ? "pass" : "fail",
  generated_at: new Date().toISOString(),
  build,
  results,
};

write(path.join(outDir, "summary.json"), JSON.stringify(summary, null, 2));

const lines = [
  "# 点对点系统测试汇总",
  "",
  `生成时间：${summary.generated_at}`,
  "",
  `整体结果：${summary.status.toUpperCase()}`,
  "",
  `构建：${build.status}`,
  "",
  "| 实验 | 结果 | 断言 | 错误 |",
  "| --- | --- | ---: | --- |",
  ...results.map((r) => {
    const assertionCount = r.assertions?.length ?? 0;
    const errors = (r.errors ?? []).map((e) => `${e.name}: ${e.message}`).join("; ");
    return `| ${r.id} | ${r.status.toUpperCase()} | ${assertionCount} | ${errors.replaceAll("|", "\\|")} |`;
  }),
  "",
];
write(path.join(outDir, "summary.md"), lines.join("\n"));
console.log(path.join(outDir, "summary.md"));
process.exit(summary.status === "pass" ? 0 : 1);
