import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dir = dirname(fileURLToPath(import.meta.url));
const qualityRoot = join(__dir, "../quality");

function loadContracts() {
  return JSON.parse(readFileSync(join(qualityRoot, "artifact_contracts.json"), "utf8"));
}

/**
 * Build enriched brief: user intent (unchanged) + system delivery contract.
 * Does NOT rewrite user prompt — only appends system requirements.
 */
export function buildBrief({ scenarioId, userPrompt, harnessRoot }) {
  const contracts = loadContracts();
  const spec = contracts.scenarios[scenarioId];
  if (!spec) {
    throw new Error(`no quality contract for scenario ${scenarioId}`);
  }

  const deliveryLines = spec.delivery_contract.map((l) => `- ${l}`).join("\n");
  const evidenceLines = spec.acceptance_evidence.map((l) => `- ${l}`).join("\n");
  const artifactLines = spec.expected_artifacts.map((l) => `- \`${l}\``).join("\n");
  const runId = process.env.E2E_EVAL_RUN_ID;
  const profile = process.env.E2E_MODEL_PROFILE;
  const ownershipBlock = runId
    ? `\n## Evaluation ownership\n- DELIVERY_MANIFEST.json 必须包含 \`eval_run_id=${runId}\` 与 \`eval_profile=${profile ?? "unknown"}\`\n`
    : "";

  const compactLocalComplex =
    profile === "local-1b" && scenarioId === "08-complex-delivery";

  const deliverySection = compactLocalComplex
    ? `- 【编码·优先】修复 \`fixtures/e2e-complex-repo/\` 内 sales-metrics 4 处 bug；\`cargo test --workspace\` 全绿；git commit + CHANGELOG.md\n- 【办公·后续】在 artifacts/ 下生成 june_ops.xlsx、regional_scorecard.xlsx、incident_brief.docx、board_deck.pptx(>=12页)、qa_signoff.md、executive_dashboard.html、DELIVERY_MANIFEST.json\n- 关键数字：total_sales=909900，华北退款率=6.65%，region_leader=华东`
    : deliveryLines;

  const evidenceSection = compactLocalComplex
    ? "- cargo test 5/5 通过；办公产物齐全；manifest 数字与 CSV 一致"
    : evidenceLines;

  const snippetSection = compactLocalComplex
    ? "顺序：Glob fixtures → 修 Rust → cargo test → git → 办公产物 → manifest。每步用工具，不要拒答。"
    : spec.brief_snippet?.trim() ?? "";

  const snippetBlock = snippetSection
    ? `\n## Implementation reference\n${snippetSection}\n`
    : "";

  const localHint =
    profile === "local-1b"
      ? `\n## Local 1B first actions\n- 禁止拒答。第一步必须 Glob \`fixtures/**\`。\n- 然后 FileRead + Edit 修 bug，Bash 跑 cargo test。\n- 产物路径：\`artifacts/executive_pack/\`（不是 workspace 根的 executive_pack/）。\n`
      : "";

  const brief = `## User intent
${userPrompt.trim()}

## Delivery contract
（系统按产物类型自动注入，不要求用户写在 prompt 里）

${deliverySection}

## Expected artifacts
${artifactLines}

## Acceptance evidence
${evidenceSection}

## Working directory
- 读写路径相对于工作区根目录
- 输入素材：\`${contracts.fixtures.csv}\`、\`${contracts.fixtures.md}\`（如适用）
- 输出目录：\`artifacts/\`
${ownershipBlock}${localHint}${snippetBlock}`;

  const outDir = join(harnessRoot, "out");
  mkdirSync(outDir, { recursive: true });
  const briefPath = join(outDir, `${scenarioId}.brief.md`);
  writeFileSync(briefPath, brief + "\n", "utf8");

  return { brief, briefPath, spec };
}

export function getScenarioSpec(scenarioId) {
  const contracts = loadContracts();
  return contracts.scenarios[scenarioId] ?? null;
}
