import fs from "node:fs";
import path from "node:path";
import { addArtifact, assert, guarded } from "../../shared/utils/assert.mjs";
import { anycodeBin, run, targetRoot, write } from "../../shared/utils/files.mjs";
import { hasOkWechatStatus, readOutboundRecords, redactedTargetState, wechatStatus } from "../../shared/utils/wechat.mjs";

const id = "006-wechat-real-send";
const outDir = path.join(targetRoot, "experiments", id, "out");

await guarded(id, outDir, async (result) => {
  fs.rmSync(outDir, { recursive: true, force: true });
  fs.mkdirSync(outDir, { recursive: true });
  const runId = `${Date.now()}-${Math.random().toString(16).slice(2, 8)}`;
  const marker = `anycode-e2e:${runId}`;
  const message = `[${marker}] 请回复收到，并忽略其它内容。这是自动化点对点测试。`;
  const before = readOutboundRecords();
  const status = wechatStatus();
  write(path.join(outDir, "wechat_status_redacted.json"), JSON.stringify({ status, target: redactedTargetState() }, null, 2));
  assert(result, hasOkWechatStatus(status), "wechat status ok", { checks: status });

  const raw = run(anycodeBin, ["channel", "wechat-send-test", "--message", message, "--json"]);
  const send = JSON.parse(raw.slice(raw.indexOf("{")));
  assert(result, send.ok === true, "wechat send command ok", { output: send });
  assert(result, send.marker === marker, "send output marker matches");
  assert(result, !raw.includes("contextToken") && !raw.includes("bot_token") && !raw.includes("fromUserId"), "send output redacted");

  const after = readOutboundRecords();
  const added = after.slice(before.length);
  write(path.join(outDir, "outbound_delta_redacted.json"), JSON.stringify(added, null, 2));
  const markerRecords = after.filter((record) => record.marker === marker);
  assert(result, !markerRecords.some((record) => record.status === "failed"), "ledger has no failed marker", {
    records: markerRecords,
  });
  assert(result, markerRecords.some((record) => record.status === "pending"), "ledger has pending marker");
  assert(result, markerRecords.some((record) => record.status === "sent"), "ledger has sent marker");
  addArtifact(result, path.join(outDir, "wechat_status_redacted.json"), "wechat_status_redacted.json");
  addArtifact(result, path.join(outDir, "outbound_delta_redacted.json"), "outbound_delta_redacted.json");
});
