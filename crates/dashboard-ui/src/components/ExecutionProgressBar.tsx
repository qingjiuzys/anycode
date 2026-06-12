import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type ProgressStep = {
  id: string;
  label: string;
  status: "pending" | "running" | "done" | "failed";
  icon: string;
};

type Props = {
  sessionId: string;
  isRunning?: boolean;
  sseLive?: boolean;
  /** When true, omit outer chrome (parent row provides border/padding). */
  embedded?: boolean;
};

export function ExecutionProgressBar({
  sessionId,
  isRunning,
  sseLive = false,
  embedded = false,
}: Props) {
  const t = useT();
  const [collapsed, setCollapsed] = useState(false);
  const pollWhileRunning = Boolean(isRunning) && !sseLive;

  const trace = useQuery({
    queryKey: ["session-trace-progress", sessionId],
    queryFn: () => api.sessionTrace(sessionId),
    staleTime: 5_000,
    refetchInterval: pollWhileRunning ? 6_000 : false,
    refetchIntervalInBackground: false,
    enabled: Boolean(sessionId) && Boolean(isRunning),
  });

  const workflowEvents = useQuery({
    queryKey: ["session-workflow-events", sessionId],
    queryFn: () =>
      api.sessionEvents(sessionId, { limit: 100, eventType: "workflow_step" }),
    staleTime: 5_000,
    refetchInterval: pollWhileRunning ? 8_000 : false,
    refetchIntervalInBackground: false,
    enabled: Boolean(sessionId) && Boolean(isRunning),
  });

  const planEvents = useQuery({
    queryKey: ["session-plan-events", sessionId],
    queryFn: () => api.sessionEvents(sessionId, { limit: 100, eventType: "plan_step" }),
    staleTime: 5_000,
    refetchInterval: pollWhileRunning ? 8_000 : false,
    refetchIntervalInBackground: false,
    enabled: Boolean(sessionId) && Boolean(isRunning),
  });

  const steps = useMemo(() => {
    const traceSteps = deriveTraceSteps(trace.data?.trace.events ?? [], isRunning);
    const wfSteps = (workflowEvents.data?.events ?? []).map((evt, i) => ({
      id: `wf-${i}-${evt.title}`,
      label: evt.title,
      status: mapWorkflowStatus((evt.payload as { status?: string })?.status),
      icon: "account_tree" as const,
    }));
    const planSteps = (planEvents.data?.events ?? []).map((evt, i) => ({
      id: `plan-${i}-${evt.title}`,
      label: evt.title,
      status: mapWorkflowStatus((evt.payload as { status?: string })?.status),
      icon: "checklist" as const,
    }));
    return [...traceSteps, ...wfSteps, ...planSteps].slice(-24);
  }, [
    isRunning,
    planEvents.data?.events,
    trace.data?.trace.events,
    workflowEvents.data?.events,
  ]);

  if (steps.length === 0) return null;

  const current = steps.find((s) => s.status === "running") ?? steps[steps.length - 1];
  const doneCount = steps.filter((s) => s.status === "done").length;

  return (
    <div
      className={
        embedded
          ? "min-w-0"
          : "px-4 py-2 border-b border-outline-variant bg-surface-container-low shrink-0"
      }
    >
      <div className="flex items-center justify-between gap-2 mb-1.5">
        <span className="text-xs font-semibold uppercase tracking-wide text-secondary inline-flex items-center gap-1.5">
          <Icon name="timeline" size={14} />
          {t("conversations.progressTitle")}
          {!isRunning && (
            <span className="font-normal normal-case text-outline">
              {doneCount}/{steps.length}
            </span>
          )}
        </span>
        {!isRunning && steps.length > 3 && (
          <button
            type="button"
            className="dw-btn-ghost text-[10px] p-1"
            onClick={() => setCollapsed((v) => !v)}
          >
            {collapsed ? t("conversations.progressExpand") : t("conversations.progressCollapse")}
          </button>
        )}
      </div>
      {isRunning && current && (
        <p className="text-xs text-on-surface m-0 mb-1.5 flex items-center gap-1.5">
          <span className="inline-block w-1.5 h-1.5 rounded-full bg-primary animate-pulse" />
          {current.label}
        </p>
      )}
      {!collapsed && (
        <div className="flex flex-wrap gap-1">
          {steps.map((step) => (
            <span
              key={step.id}
              title={step.label}
              className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] border ${
                step.status === "running"
                  ? "border-primary bg-primary/10 text-primary"
                  : step.status === "done"
                    ? "border-outline-variant bg-surface-container-lowest text-secondary"
                    : step.status === "failed"
                      ? "border-error/40 bg-error/10 text-error"
                      : "border-outline-variant/50 text-outline"
              }`}
            >
              <Icon name={step.icon} size={12} />
              <span className="max-w-[8rem] truncate">{step.label}</span>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

type TraceEvent = {
  event_type: string;
  severity: string;
  title: string;
  body: string;
  payload: Record<string, unknown>;
};

function deriveTraceSteps(events: TraceEvent[], isRunning?: boolean): ProgressStep[] {
  const steps: ProgressStep[] = [];
  let turn = 0;
  let toolIdx = 0;

  for (const evt of events) {
    if (evt.event_type === "turn_start") {
      turn += 1;
      steps.push({
        id: `turn-${turn}`,
        label: `Turn ${turn}`,
        status: "running",
        icon: "sync",
      });
    } else if (evt.event_type === "turn_end") {
      const idx = steps.map((s) => s.id).lastIndexOf(`turn-${turn}`);
      if (idx >= 0) steps[idx].status = evt.severity === "error" ? "failed" : "done";
    } else if (evt.event_type === "tool_call_start" || evt.event_type === "tool_call_input") {
      toolIdx += 1;
      const tool =
        (evt.payload?.name as string | undefined) ??
        evt.title.replace(/^Tool\s*/i, "") ??
        "tool";
      steps.push({
        id: `tool-${toolIdx}`,
        label: tool,
        status: "running",
        icon: "build",
      });
    } else if (evt.event_type === "tool_call_end") {
      for (let i = steps.length - 1; i >= 0; i -= 1) {
        if (steps[i].id.startsWith("tool-") && steps[i].status === "running") {
          steps[i].status = evt.severity === "error" ? "failed" : "done";
          break;
        }
      }
    }
  }

  if (isRunning) {
    const hasRunning = steps.some((s) => s.status === "running");
    if (!hasRunning && steps.length > 0) {
      steps[steps.length - 1].status = "running";
    }
  } else {
    for (const s of steps) {
      if (s.status === "running") s.status = "done";
    }
  }

  return steps;
}

function mapWorkflowStatus(status?: string): ProgressStep["status"] {
  if (status === "failed") return "failed";
  if (status === "running" || status === "in_progress") return "running";
  if (status === "completed" || status === "done") return "done";
  return "pending";
}
