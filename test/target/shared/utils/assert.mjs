import fs from "node:fs";

export function createResult(id) {
  return {
    id,
    status: "pass",
    started_at: new Date().toISOString(),
    finished_at: null,
    artifacts: [],
    assertions: [],
    errors: [],
  };
}

export function pass(result, name, details = {}) {
  result.assertions.push({ name, status: "pass", ...details });
}

export function fail(result, name, error, details = {}) {
  const message = error instanceof Error ? error.message : String(error);
  result.status = "fail";
  result.assertions.push({ name, status: "fail", message, ...details });
  result.errors.push({ name, message });
}

export function assert(result, condition, name, details = {}) {
  if (condition) {
    pass(result, name, details);
    return;
  }
  fail(result, name, details.message ?? "assertion failed", details);
}

export function addArtifact(result, file, label = file) {
  result.artifacts.push({ label, file });
}

export function finish(result, outDir) {
  result.finished_at = new Date().toISOString();
  fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(`${outDir}/result.json`, `${JSON.stringify(result, null, 2)}\n`);
  return result;
}

export async function guarded(id, outDir, fn) {
  const result = createResult(id);
  try {
    await fn(result);
  } catch (error) {
    fail(result, "uncaught", error);
  }
  return finish(result, outDir);
}
