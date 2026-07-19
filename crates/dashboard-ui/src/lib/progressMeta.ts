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
  }
  if (block.meta?.narration === true || block.meta?.message_role === "status") {
    return "execute";
  }
  return "execute";
}

export function progressSummary(block: TranscriptBlock, localeBody?: string): string {
  const metaSummary = block.meta?.summary;
  if (typeof metaSummary === "string" && metaSummary.trim()) {
    return metaSummary.trim();
  }
  return (localeBody ?? block.body ?? "").trim();
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
    default:
      return null;
  }
}
