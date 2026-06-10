import type { ProjectEvent, ProjectStatsFailure, TranscriptBlock } from "@/api/types";

type EventLike = {
  title: string;
  event_type: string;
  payload?: Record<string, unknown> | null;
};

type ExecutionLogLine = {
  event_type?: string | null;
  title?: string | null;
  raw: string;
};

/** Map internal event_type to i18n key under `eventTypes.*` */
const EVENT_TYPE_KEYS: Record<string, string> = {
  turn_start: "turn_start",
  turn_end: "turn_end",
  llm_request_start: "llm_request_start",
  llm_response_end: "llm_response_end",
  llm_response: "llm_response_end",
  tool_call_start: "tool_call_start",
  tool_call_end: "tool_call_end",
  tool_call_input: "tool_call",
  tool_denied: "tool_denied",
  tool_approval_pending: "tool_approval_pending",
  tool_approval_resolved: "tool_approval_resolved",
  task_start: "task_start",
  task_end: "task_end",
  user_prompt: "user_prompt",
  assistant_response: "assistant_response",
  gate: "gate",
  gate_executed: "gate_executed",
  session_created: "session_created",
  session_completed: "session_completed",
  session_blocked: "session_blocked",
  artifact_created: "artifact_created",
  budget_warning: "budget_warning",
  budget_degrade: "budget_degrade",
  budget_exceeded: "budget_exceeded",
  approval_required: "approval_required",
  approval_granted: "approval_granted",
  approval_denied: "approval_denied",
  project_root_created: "project_root_created",
  prompt: "user_prompt",
};

const STATUS_KEYS = new Set([
  "ok",
  "running",
  "passed",
  "verified",
  "completed",
  "warn",
  "warning",
  "unverified",
  "pending",
  "error",
  "failed",
  "blocked",
  "cancelled",
  "info",
]);

const TRUSTED_STATUS_KEYS = new Set(["verified", "unverified", "blocked"]);

export function eventTypeI18nKey(eventType: string): string {
  const id = eventType.trim().toLowerCase();
  if (EVENT_TYPE_KEYS[id]) {
    return `eventTypes.${EVENT_TYPE_KEYS[id]}`;
  }
  if (id.startsWith("tool_call")) {
    return "eventTypes.tool_call";
  }
  return `eventTypes.other`;
}

export function formatEventTypeLabel(eventType: string, t: (key: string) => string): string {
  const key = eventTypeI18nKey(eventType);
  const label = t(key);
  return label !== key ? label : eventType;
}

function interpolate(template: string, vars: Record<string, string>): string {
  return Object.entries(vars).reduce(
    (acc, [k, v]) => acc.replaceAll(`{${k}}`, v),
    template,
  );
}

function payloadField(
  payload: Record<string, unknown> | null | undefined,
  key: string,
): string | undefined {
  if (!payload) {
    return undefined;
  }
  const v = payload[key];
  return typeof v === "string" && v.trim() ? v.trim() : undefined;
}

/** Map backend English log titles to localized labels (presentation layer only). */
export function localizeLogTitle(
  rawTitle: string,
  eventType: string,
  t: (key: string) => string,
  payload?: Record<string, unknown> | null,
): string | null {
  const raw = rawTitle.trim();
  if (!raw) {
    return null;
  }
  const lower = raw.toLowerCase();

  if (lower.includes("imported task")) {
    return t("eventTitles.importedTask");
  }
  if (lower.includes("manual gate")) {
    return t("eventTitles.gateRunGeneric");
  }

  const exactTitles: Record<string, string> = {
    "user prompt": "eventTitles.userPrompt",
    "llm response": "eventTitles.llmResponse",
    "budget warning": "eventTitles.budgetWarning",
    "budget degradation": "eventTitles.budgetDegrade",
    "budget exceeded": "eventTitles.budgetExceeded",
    "task failed": "eventTitles.taskFailed",
    "no reply recorded": "eventTitles.noReplyRecorded",
    "tool denied": "eventTitles.toolDeniedBare",
    "task start": "eventTitles.taskStart",
    error: "eventTitles.error",
    assistant: "eventTitles.assistant",
    you: "eventTitles.you",
  };
  if (exactTitles[lower]) {
    return t(exactTitles[lower]);
  }

  let   m = raw.match(/^Turn (\d+)$/i);
  if (m) {
    return interpolate(t("eventTitles.turnStart"), { turn: m[1] });
  }
  m = raw.match(/^Turn (\d+) finished$/i);
  if (m) {
    return interpolate(t("eventTitles.turnEnd"), { turn: m[1] });
  }
  m = raw.match(/^(.+?) started$/i);
  if (m) {
    return interpolate(t("eventTitles.toolStarted"), { tool: m[1].trim() });
  }
  m = raw.match(/^(.+?) finished$/i);
  if (m) {
    return interpolate(t("eventTitles.toolFinished"), { tool: m[1].trim() });
  }
  m = raw.match(/^(.+?) failed$/i);
  if (m) {
    return interpolate(t("eventTitles.toolFailed"), { tool: m[1].trim() });
  }
  m = raw.match(/^(.+?) denied$/i);
  if (m) {
    return interpolate(t("eventTitles.toolDenied"), { tool: m[1].trim() });
  }
  m = raw.match(/^(.+?) awaiting approval$/i);
  if (m) {
    return interpolate(t("eventTitles.toolAwaitingApproval"), { tool: m[1].trim() });
  }
  m = raw.match(/^(.+?) approved$/i);
  if (m) {
    return interpolate(t("eventTitles.toolApproved"), { tool: m[1].trim() });
  }
  m = raw.match(/^Task started \((.+)\)$/i);
  if (m) {
    return interpolate(t("eventTitles.taskStarted"), { agent: m[1].trim() });
  }
  m = raw.match(/^Task (completed|failed|cancelled|running|unknown)$/i);
  if (m) {
    const statusKey = m[1].toLowerCase();
    const statusLabel = STATUS_KEYS.has(statusKey)
      ? t(`status.${statusKey}`)
      : m[1];
    return interpolate(t("eventTitles.taskEnd"), { status: statusLabel });
  }
  m = raw.match(/^LLM (.+?) turn (\d+)$/i);
  if (m) {
    return interpolate(t("eventTitles.llmRequest"), { model: m[1].trim(), turn: m[2] });
  }
  m = raw.match(/^LLM response \((\d+) in \/ (\d+) out tokens\)$/i);
  if (m) {
    return interpolate(t("eventTitles.llmResponseTokens"), {
      input: m[1],
      output: m[2],
    });
  }
  m = raw.match(/^Gate: (.+)$/i);
  if (m) {
    return interpolate(t("eventTitles.gateColon"), { name: m[1].trim() });
  }
  m = raw.match(/^Assistant \(turn (\d+)\)$/i);
  if (m) {
    return interpolate(t("eventTitles.assistantTurn"), { turn: m[1] });
  }

  if (eventType === "gate_executed") {
    const name = payloadField(payload, "name");
    if (name) {
      return interpolate(t("eventTitles.gateRun"), { name });
    }
    return t("eventTitles.gateRunGeneric");
  }

  return null;
}

/** Friendlier display title for timeline rows */
export function formatEventTitle(
  event: ProjectEvent | ProjectStatsFailure | EventLike,
  t: (key: string) => string,
): string {
  const raw = event.title?.trim() ?? "";
  const payload =
    "payload" in event && event.payload && typeof event.payload === "object"
      ? (event.payload as Record<string, unknown>)
      : undefined;

  const localized = localizeLogTitle(raw, event.event_type, t, payload);
  if (localized) {
    return localized;
  }

  if (raw && !looksLikeInternalSlug(raw)) {
    return raw;
  }

  const typeLabel = formatEventTypeLabel(event.event_type, t);
  if (typeLabel && typeLabel !== event.event_type) {
    return typeLabel;
  }
  return raw || typeLabel || event.event_type;
}

export function formatExecutionLogLine(
  line: ExecutionLogLine,
  t: (key: string) => string,
): { eventType?: string; title?: string } {
  const eventType = line.event_type?.trim();
  const rawTitle = line.title?.trim() ?? "";
  const localized = eventType
    ? localizeLogTitle(rawTitle, eventType, t)
    : rawTitle
      ? localizeLogTitle(rawTitle, "", t)
      : null;

  return {
    eventType: eventType ? formatEventTypeLabel(eventType, t) : undefined,
    title: localized ?? (rawTitle || undefined),
  };
}

export function formatTranscriptBlockTitle(
  block: Pick<TranscriptBlock, "block_type" | "title">,
  t: (key: string) => string,
): string {
  const raw = block.title?.trim() ?? "";
  const localized = localizeLogTitle(raw, block.block_type, t);
  if (localized) {
    return localized;
  }

  switch (block.block_type) {
    case "user_message":
      return t("eventTitles.you");
    case "assistant_message":
      return raw || t("conversations.assistantReply");
    case "system_notice":
      return raw || t("conversations.systemNotice");
    case "session_error":
      return raw || t("eventTitles.taskFailed");
    case "tool_call":
    case "tool_result":
      return raw || t("conversations.toolActivity");
    default:
      return raw || t("conversations.toolActivity");
  }
}

export function formatStatusLabel(status: string, t: (key: string) => string): string {
  const key = status.trim().toLowerCase();
  if (STATUS_KEYS.has(key)) {
    const label = t(`status.${key}`);
    return label !== `status.${key}` ? label : status;
  }
  return status;
}

export function formatTrustedStatusLabel(status: string, t: (key: string) => string): string {
  const key = status.trim().toLowerCase();
  if (TRUSTED_STATUS_KEYS.has(key)) {
    const label = t(`trustedStatus.${key}`);
    return label !== `trustedStatus.${key}` ? label : status;
  }
  return formatStatusLabel(status, t);
}

export function formatLiveToolLabel(toolName: string, t: (key: string) => string): string {
  return interpolate(t("conversations.liveToolRunning"), { tool: toolName });
}

function looksLikeInternalSlug(s: string): boolean {
  return /^[a-z][a-z0-9_]*$/.test(s) && s.includes("_");
}

export function isImportedSessionTitle(title: string): boolean {
  return title.toLowerCase().includes("imported task");
}

export function formatSessionDisplayTitle(
  title: string,
  kind: string,
  t: (key: string) => string,
): string {
  if (isImportedSessionTitle(title)) {
    return t("sessionTitles.importedTask");
  }
  if (title.toLowerCase().includes("manual gate")) {
    return t("sessionTitles.manualGate");
  }
  const kindKey = `sessionFlow.${kind}`;
  const kindLabel = t(kindKey);
  const k = kindLabel !== kindKey ? kindLabel : kind;
  if (title.length > 48) {
    return `${k} · ${title.slice(0, 45)}…`;
  }
  return `${k} · ${title}`;
}

export function formatSessionFlowStatusLine(
  status: string,
  trustedStatus: string,
  t: (key: string) => string,
): string {
  return `${formatStatusLabel(status, t)} · ${formatTrustedStatusLabel(trustedStatus, t)}`;
}
