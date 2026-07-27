import type { TranscriptBlock } from "@/api/types";

export type AgentPhaseKind = "intent" | "execute" | "discovery" | "deliver";

export function isProgressBlock(block: TranscriptBlock): boolean {
  return (
    block.block_type === "progress_update" ||
    (block.block_type === "assistant_message" &&
      (block.meta?.narration === true || block.meta?.message_role === "status"))
  );
}

export function progressPhase(block: TranscriptBlock): AgentPhaseKind {
  const raw = block.meta?.phase;
  if (typeof raw === "string") {
    if (raw === "intent" || raw === "execute" || raw === "discovery" || raw === "deliver") {
      return raw;
    }
    // Compile / gate / skill markers are preflight (intent).
    if (raw === "gate" || raw === "skill" || raw === "compile") {
      return "intent";
    }
  }
  if (block.meta?.work_stage === "compile") {
    return "intent";
  }
  if (block.meta?.narration === true || block.meta?.message_role === "status") {
    return "execute";
  }
  return "execute";
}

/** Format `[delivery_preflight] …` for Workbench display. */
export function formatDeliveryPreflight(summary: string): string | null {
  if (!summary.includes("[delivery_preflight]")) return null;
  const family = summary.match(/family=([^\s]+)/)?.[1] ?? "—";
  const skill = summary.match(/skill=([^\s]+)/)?.[1] ?? "—";
  const brand = summary.match(/brand=([^\s]+)/)?.[1] ?? "—";
  const scenario = summary.match(/scenario=([^\s]+)/)?.[1] ?? "—";
  const artifacts = summary.match(/artifacts=\[([^\]]*)\]/)?.[1] ?? "";
  const gates = summary.match(/gates=(\d+)/)?.[1] ?? "0";
  return `交付预检 · ${family} → ${skill} · brand ${brand} · scenario ${scenario} · [${artifacts}] · ${gates} gates`;
}

export function progressSummary(block: TranscriptBlock, localeBody?: string): string {
  const metaSummary = block.meta?.summary;
  const raw =
    typeof metaSummary === "string" && metaSummary.trim()
      ? metaSummary.trim()
      : (localeBody ?? block.body ?? "").trim();
  return formatDeliveryPreflight(raw) ?? raw;
}

export function progressNext(block: TranscriptBlock): string | null {
  const raw = block.meta?.next;
  return typeof raw === "string" && raw.trim() ? raw.trim() : null;
}

export function progressDiscovery(block: TranscriptBlock): string | null {
  const raw = block.meta?.discovery;
  return typeof raw === "string" && raw.trim() ? raw.trim() : null;
}

export function progressWorkStage(block: TranscriptBlock): string | null {
  const raw = block.meta?.work_stage;
  return typeof raw === "string" && raw.trim() ? raw.trim() : null;
}

export function progressEvidenceRefs(block: TranscriptBlock): string[] {
  const raw = block.meta?.evidence_refs;
  if (!Array.isArray(raw)) return [];
  return raw.filter((v): v is string => typeof v === "string");
}

export function phaseTitleKey(phase: AgentPhaseKind): string {
  switch (phase) {
    case "intent":
      return "conversations.progressPhaseIntent";
    case "execute":
      return "conversations.progressPhaseExecute";
    case "discovery":
      return "conversations.progressPhaseDiscovery";
    case "deliver":
      return "conversations.progressPhaseDeliver";
  }
}

export function workStageLabelKey(stage: string): string | null {
  switch (stage) {
    case "inspect":
      return "conversations.progressWorkInspect";
    case "analyze":
      return "conversations.progressWorkAnalyze";
    case "implement":
      return "conversations.progressWorkImplement";
    case "verify":
      return "conversations.progressWorkVerify";
    case "compile":
      return "conversations.progressWorkCompile";
    default:
      return null;
  }
}
