import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import type { PlanNode, PlanStatus } from "@/api/types/workbench";
import { Icon } from "@/components/Icon";
import { ModelPicker } from "@/components/ModelPicker";
import { useT } from "@/i18n/context";
import {
  markPlanBuilt,
  planAwaitingBuild,
} from "@/lib/planBuildState";
import { SESSION_QUERY_GC_MS } from "@/lib/sessionQuery";

const STATUS_GLYPH: Record<PlanStatus, string> = {
  pending: "[ ]",
  in_progress: "[~]",
  completed: "[x]",
  blocked: "[!]",
  failed: "[X]",
  cancelled: "[-]",
};

type Props = {
  sessionId: string;
  isRunning?: boolean;
  onBuildStarted?: () => void;
};

function PlanNodeRow({ node, depth }: { node: PlanNode; depth: number }) {
  const [expanded, setExpanded] = useState(true);
  const hasChildren = (node.children?.length ?? 0) > 0;
  const glyph = STATUS_GLYPH[node.status] ?? "[ ]";

  return (
    <div>
      <button
        type="button"
        className="w-full flex items-start gap-1 py-0.5 pr-2 text-left text-xs border-0 bg-transparent cursor-pointer hover:bg-surface-container-low text-on-surface"
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
        onClick={() => {
          if (hasChildren) setExpanded((v) => !v);
        }}
      >
        {hasChildren ? (
          <Icon
            name={expanded ? "expand_more" : "chevron_right"}
            size={14}
            className="shrink-0 text-secondary mt-0.5"
          />
        ) : (
          <span className="w-[14px] shrink-0" />
        )}
        <span className="font-mono text-secondary shrink-0 mt-0.5">{glyph}</span>
        <span className="min-w-0">
          <span className="truncate block">{node.title}</span>
          {node.detail ? (
            <span className="block text-[11px] text-secondary truncate">{node.detail}</span>
          ) : null}
        </span>
      </button>
      {expanded &&
        hasChildren &&
        node.children!.map((child) => (
          <PlanNodeRow key={child.id} node={child} depth={depth + 1} />
        ))}
    </div>
  );
}

export function PlanTreePanel({ sessionId, isRunning = false, onBuildStarted }: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const query = useQuery({
    queryKey: ["session-plan-tree", sessionId],
    queryFn: () => api.sessionPlanTree(sessionId),
    enabled: Boolean(sessionId),
    staleTime: 5_000,
    gcTime: SESSION_QUERY_GC_MS,
  });

  const buildPlan = useMutation({
    mutationFn: () =>
      api.sendSessionMessage(sessionId, {
        prompt: t("workbench.planBuildPrompt"),
        enqueue: isRunning,
      }),
    onSuccess: () => {
      const updatedAt = query.data?.updated_at;
      if (updatedAt) {
        markPlanBuilt(sessionId, updatedAt);
      }
      void queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
      void queryClient.invalidateQueries({ queryKey: ["session-transcript", sessionId] });
      onBuildStarted?.();
    },
  });

  if (query.isPending) {
    return <p className="text-xs text-secondary px-3 py-2 m-0">Loading…</p>;
  }
  if (query.error) {
    return (
      <p className="text-xs text-error px-3 py-2 m-0">
        {(query.error as Error).message}
      </p>
    );
  }

  const roots = query.data?.tree?.roots ?? [];
  const updatedAt = query.data?.updated_at ?? null;
  const awaitingBuild =
    query.data?.tree && planAwaitingBuild(query.data.tree, updatedAt, sessionId);

  if (roots.length === 0) {
    return (
      <div className="px-4 py-6 text-center">
        <Icon name="account_tree" size={28} className="text-secondary mb-2" />
        <p className="text-sm text-secondary m-0">{t("workbench.planEmpty")}</p>
        <p className="text-xs text-secondary m-0 mt-2">{t("workbench.planHint")}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col min-h-0 h-full">
      {awaitingBuild ? (
        <div className="conv-plan-build-bar shrink-0 mx-2 mt-2 mb-1 rounded-xl border border-primary/25 bg-primary/5 px-3 py-3 space-y-2.5">
          <div>
            <p className="text-sm font-medium text-on-surface m-0">
              {t("workbench.planBuildTitle")}
            </p>
            <p className="text-xs text-secondary m-0 mt-1 leading-relaxed">
              {t("workbench.planBuildHint")}
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <ModelPicker compact disabled={buildPlan.isPending} />
            <button
              type="button"
              className="dw-btn-primary text-sm ml-auto shrink-0"
              disabled={buildPlan.isPending}
              onClick={() => buildPlan.mutate()}
            >
              {buildPlan.isPending ? t("common.loading") : t("workbench.planBuildAction")}
            </button>
          </div>
          {buildPlan.error ? (
            <p className="text-xs text-error m-0">{(buildPlan.error as Error).message}</p>
          ) : null}
        </div>
      ) : null}
      <div className="py-1 min-h-0 flex-1 overflow-y-auto">
        {roots.map((node) => (
          <PlanNodeRow key={node.id} node={node} depth={0} />
        ))}
      </div>
    </div>
  );
}
