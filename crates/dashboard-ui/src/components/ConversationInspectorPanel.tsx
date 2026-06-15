import type { ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { ArtifactRecord, TranscriptBlock } from "@/api/types";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { ToolDetailPanel } from "@/components/TranscriptToolBlock";
import { useT } from "@/i18n/context";
import { sessionArtifactsQueryOptions } from "@/lib/sessionQuery";

type Props = {
  sessionId: string | null;
  className?: string;
  live?: boolean;
  isRunning?: boolean;
  selectedTool?: TranscriptBlock | null;
  onSelectTool?: (tool: TranscriptBlock | null) => void;
};

type ArtifactGroup = {
  id: string;
  label: string;
  icon: string;
  items: ArtifactRecord[];
};

export function ConversationInspectorPanel({
  sessionId,
  className = "",
  live,
  isRunning = false,
  selectedTool,
}: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const running = Boolean(isRunning);

  const artifacts = useQuery({
    ...sessionArtifactsQueryOptions(sessionId!, running),
    enabled: Boolean(sessionId),
    refetchInterval: live ? false : false,
    refetchIntervalInBackground: false,
    placeholderData: (prev) => prev,
  });

  const trace = useQuery({
    queryKey: ["session-trace-inspector", sessionId],
    queryFn: () => api.sessionTrace(sessionId!),
    enabled: Boolean(sessionId),
    staleTime: running ? 3_000 : 15_000,
    refetchInterval: running && !live ? 6_000 : false,
    refetchIntervalInBackground: false,
    placeholderData: (prev) => prev,
  });

  const scan = useMutation({
    mutationFn: () => api.scanSessionArtifacts(sessionId!),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["session-artifacts", sessionId] });
    },
  });

  if (!sessionId) {
    return (
      <aside className={`flex flex-col h-full min-h-0 bg-surface-container-lowest ${className}`}>
        <PanelHeader title={t("conversations.inspectorTitle")} />
        <div className="flex-1 flex items-center justify-center p-4">
          <p className="text-sm text-secondary m-0 text-center">{t("conversations.selectSession")}</p>
        </div>
      </aside>
    );
  }

  const showColdLoading = artifacts.isPending && !artifacts.data;
  if (showColdLoading) {
    return (
      <aside className={`flex flex-col h-full min-h-0 bg-surface-container-lowest ${className}`}>
        <PanelHeader title={t("conversations.inspectorTitle")} />
        <p className="text-sm text-secondary px-4 py-6 m-0">{t("common.loading")}</p>
      </aside>
    );
  }

  const rows = artifacts.data?.artifacts ?? [];
  const groups = groupArtifacts(rows, t);
  const traceEvents = (trace.data?.trace.events ?? []).filter((evt) =>
    evt.event_type.startsWith("tool_call"),
  );
  const recentTrace = traceEvents.slice(-12).reverse();

  return (
    <aside
      className={`flex flex-col h-full min-h-0 bg-surface-container-lowest ${className} ${
        artifacts.isFetching && artifacts.data ? "opacity-90" : ""
      }`}
    >
      <PanelHeader title={t("conversations.inspectorTitle")} />
      <div className="flex-1 min-h-0 overflow-y-auto">
        <InspectorSection
          title={t("conversations.inspectorTimeline")}
          icon="timeline"
          count={recentTrace.length}
        >
          {recentTrace.length === 0 ? (
            <p className="text-xs text-secondary m-0 px-3 py-2">
              {t("conversations.inspectorTimelineEmpty")}
            </p>
          ) : (
            <ul className="m-0 p-0 list-none">
              {recentTrace.map((evt, index) => (
                <li
                  key={`${evt.event_type}-${evt.occurred_at}-${index}`}
                  className="px-3 py-1.5 text-xs border-b border-outline-variant/30 last:border-0"
                >
                  <span className="font-medium text-on-surface block truncate">
                    {(evt.payload?.name as string | undefined) ?? evt.title}
                  </span>
                  <span className="text-secondary font-code truncate block">
                    {evt.event_type.replace("tool_call_", "")}
                    {typeof evt.payload?.command === "string"
                      ? ` · ${evt.payload.command}`
                      : ""}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </InspectorSection>

        <InspectorSection title={t("conversations.inspectorDetail")} icon="build">
          <ToolDetailPanel tool={selectedTool ?? null} />
        </InspectorSection>

        <InspectorSection
          title={t("conversations.artifactsPanel")}
          icon="inventory_2"
          count={rows.length}
        >
          {rows.length === 0 ? (
            <div className="p-3">
              <EmptyState
                title={t("conversations.artifactsEmpty")}
                description={t("conversations.inspectorArtifactsEmptyDesc")}
                icon="inventory_2"
              />
              <div className="text-center mt-3">
                <button
                  type="button"
                  className="dw-btn-secondary text-xs"
                  disabled={scan.isPending}
                  onClick={() => scan.mutate()}
                >
                  <Icon name="document_scanner" size={14} className="inline mr-1" />
                  {scan.isPending ? t("conversations.artifactsScanning") : t("conversations.artifactsScan")}
                </button>
              </div>
            </div>
          ) : (
            <div className="py-1">
              {groups.map((group) => (
                <section key={group.id} className="mb-3">
                  <h4 className="px-3 py-1 text-[10px] font-semibold uppercase tracking-wide text-secondary m-0 flex items-center gap-1.5">
                    <Icon name={group.icon} size={14} />
                    {group.label}
                    <span className="text-outline">({group.items.length})</span>
                  </h4>
                  <ul className="m-0 p-0 list-none">
                    {group.items.map((item) => (
                      <li key={item.id}>
                        <Link
                          to="/assets/$artifactId"
                          params={{ artifactId: item.id }}
                          className="flex items-start gap-2 px-3 py-2 no-underline hover:bg-surface-container-low transition-colors"
                        >
                          <Icon
                            name={artifactIcon(item.kind)}
                            size={16}
                            className="text-secondary shrink-0 mt-0.5"
                          />
                          <span className="min-w-0 flex-1">
                            <span className="block text-sm font-medium text-on-surface truncate">
                              {item.title || item.path.split("/").pop() || item.path}
                            </span>
                            <span className="block text-[11px] text-secondary truncate font-code">
                              {item.path}
                            </span>
                          </span>
                        </Link>
                      </li>
                    ))}
                  </ul>
                </section>
              ))}
            </div>
          )}
        </InspectorSection>

        <div className="px-3 pt-2 pb-4 border-t border-outline-variant/60 mt-2">
          <Link
            to="/sessions/$sessionId"
            params={{ sessionId }}
            className="text-xs text-secondary no-underline hover:text-primary inline-flex items-center gap-1"
          >
            <Icon name="timeline" size={14} />
            {t("conversations.openDetail")}
          </Link>
        </div>
      </div>
    </aside>
  );
}

function PanelHeader({ title }: { title: string }) {
  return (
    <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-secondary border-b border-outline-variant bg-surface-container-low shrink-0">
      <span className="inline-flex items-center gap-1.5">
        <Icon name="view_sidebar" size={14} />
        {title}
      </span>
    </div>
  );
}

function InspectorSection({
  title,
  icon,
  count,
  children,
}: {
  title: string;
  icon: string;
  count?: number;
  children: ReactNode;
}) {
  return (
    <section className="border-b border-outline-variant/60">
      <h3 className="px-3 py-2 text-[10px] font-semibold uppercase tracking-wide text-secondary m-0 flex items-center gap-1.5 bg-surface-container-low/50">
        <Icon name={icon} size={14} />
        {title}
        {typeof count === "number" && count > 0 && (
          <span className="text-outline normal-case">({count})</span>
        )}
      </h3>
      {children}
    </section>
  );
}

function groupArtifacts(rows: ArtifactRecord[], t: ReturnType<typeof useT>): ArtifactGroup[] {
  const report: ArtifactRecord[] = [];
  const file: ArtifactRecord[] = [];
  const other: ArtifactRecord[] = [];

  for (const row of rows) {
    const kind = row.kind.toLowerCase();
    if (kind.includes("report")) {
      report.push(row);
    } else if (kind.includes("file") || kind === "output" || kind === "artifact") {
      file.push(row);
    } else {
      other.push(row);
    }
  }

  const groups: ArtifactGroup[] = [];
  if (report.length > 0) {
    groups.push({
      id: "report",
      label: t("conversations.artifactsGroupReport"),
      icon: "description",
      items: report,
    });
  }
  if (file.length > 0) {
    groups.push({
      id: "file",
      label: t("conversations.artifactsGroupFile"),
      icon: "folder",
      items: file,
    });
  }
  if (other.length > 0) {
    groups.push({
      id: "other",
      label: t("conversations.artifactsGroupOther"),
      icon: "category",
      items: other,
    });
  }
  return groups;
}

function artifactIcon(kind: string): string {
  const lower = kind.toLowerCase();
  if (lower.includes("report")) return "description";
  if (lower.includes("image")) return "image";
  return "insert_drive_file";
}

/** Back-compat alias for older imports. */
export { ConversationInspectorPanel as ConversationArtifactsPanel };
