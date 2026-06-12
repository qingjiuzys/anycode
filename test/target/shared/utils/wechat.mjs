import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { anycodeBin, run } from "./files.mjs";

export const wechatDir = path.join(os.homedir(), ".anycode/wechat");
export const outboundLog = path.join(wechatDir, "outbound.jsonl");
export const cronTarget = path.join(wechatDir, "cron_notify_target.json");

export function readOutboundRecords() {
  if (!fs.existsSync(outboundLog)) return [];
  return fs
    .readFileSync(outboundLog, "utf8")
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => {
      try {
        const record = JSON.parse(line);
        delete record.to_user_id;
        return record;
      } catch {
        return { status: "parse_error" };
      }
    });
}

export function wechatStatus() {
  return JSON.parse(run(anycodeBin, ["channel", "status", "wechat", "--json"]));
}

export function hasOkWechatStatus(rows) {
  return rows
    .filter((row) => row.name !== "channel.wechat.outbound")
    .every((row) => row.status === "ok");
}

export function redactedTargetState() {
  return {
    data_dir_exists: fs.existsSync(wechatDir),
    cron_target_exists: fs.existsSync(cronTarget),
    outbound_log_exists: fs.existsSync(outboundLog),
  };
}
