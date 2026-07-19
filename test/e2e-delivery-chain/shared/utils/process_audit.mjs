import { readFileSync, readdirSync, existsSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

function parseToolChain(text) {
  const chain = [];
  const errors = [];
  for (const m of text.matchAll(/\[tool_call_start\][^\n]*name=(\w+)/g)) {
    chain.push(m[1]);
  }
  for (const m of text.matchAll(/\[tool_call_end\][^\n]*name=(\w+)[^\n]*error=([^\n]+)/g)) {
    const err = m[2].trim();
    if (err && err !== "<none>") {
      errors.push({ tool: m[1], error: err });
    }
  }
  return { chain, errors };
}

export function extractTaskLogPath(stderrOrStdout) {
  const text = stderrOrStdout ?? "";
  const m = text.match(/Task output:\s*[^\w/]*([^\s⁩]+output\.log)/);
  if (m) return m[1].replace(/[⁨⁩]/g, "");
  const m2 = text.match(/(\/[^\s]+anycode\/tasks\/[^/\s]+\/output\.log)/);
  return m2?.[1] ?? null;
}

export function auditFromLogText(text) {
  if (!text) {
    return { skillFailed: false, skillRecovered: false, degraded: false, toolChain: [], errors: [], matchedLog: null };
  }
  const { chain, errors } = parseToolChain(text);
  const skillErrors = errors.filter((e) => e.tool === "Skill");
  const skillFailed = skillErrors.length > 0 || /skill exited non-zero/.test(text);
  const skillRecovered = skillFailed && /\[task_end\] status=completed/.test(text);
  return {
    skillFailed,
    skillRecovered,
    degraded: skillFailed && skillRecovered,
    toolChain: chain,
    errors,
    skillErrors,
    matchedLog: null,
  };
}

export function auditFromTaskLog(logPath) {
  if (!logPath || !existsSync(logPath)) {
    return auditFromLogText("");
  }
  const text = readFileSync(logPath, "utf8");
  const result = auditFromLogText(text);
  result.matchedLog = logPath;
  return result;
}

export function auditScenarioRun(runJson) {
  const combined = `${runJson?.stdoutTail ?? ""}\n${runJson?.stderrTail ?? ""}`;
  const logPath = runJson?.processAudit?.matchedLog ?? extractTaskLogPath(combined);
  if (logPath && existsSync(logPath)) {
    return auditFromTaskLog(logPath);
  }
  return auditFromLogText(combined);
}

export function auditProcessFromTasks({ sinceMs = 0 } = {}) {
  const tasksDir = join(process.env.HOME ?? "", ".anycode/tasks");
  if (!existsSync(tasksDir)) {
    return auditFromLogText("");
  }

  let best = null;
  let bestMtime = 0;
  for (const name of readdirSync(tasksDir)) {
    const p = join(tasksDir, name);
    try {
      const st = statSync(p);
      if (st.isDirectory()) {
        const out = join(p, "output.log");
        if (existsSync(out)) {
          const mt = statSync(out).mtimeMs;
          if (mt >= sinceMs && mt >= bestMtime) {
            bestMtime = mt;
            best = out;
          }
        }
      }
    } catch {
      /* skip */
    }
  }
  return best ? auditFromTaskLog(best) : auditFromLogText("");
}

/**
 * Harness-side process audit — does not rely on Agent writing audit.json.
 */
export function buildProcessAuditRecord({ scenarioId, process, artifactPath, workspace }) {
  const skillErr = process.skillErrors?.[0];
  return {
    scenario: scenarioId,
    generated_at: new Date().toISOString(),
    harness_captured: true,
    actual_path: artifactPath
      ? artifactPath.startsWith(workspace)
        ? artifactPath.slice(workspace.length).replace(/^\//, "")
        : artifactPath
      : null,
    degraded: Boolean(process.degraded),
    skill_failed: Boolean(process.skillFailed),
    skill_recovered: Boolean(process.skillRecovered),
    fallback_reason: skillErr?.error ?? (process.degraded ? "skill_failed_then_fallback" : null),
    tool_chain: process.toolChain ?? [],
    log_path: process.matchedLog,
  };
}

export function writeProcessAuditFile(auditPath, record) {
  writeFileSync(auditPath, JSON.stringify(record, null, 2) + "\n", "utf8");
  return auditPath;
}
