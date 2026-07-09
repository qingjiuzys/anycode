import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { ArtifactRecord, ReportDocument } from "@/api/types";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { ReportPreview } from "@/components/ReportPreview";
import { useI18n, useT } from "@/i18n/context";
import { SESSION_QUERY_GC_MS, TRANSCRIPT_STALE_RUNNING_MS } from "@/lib/sessionQuery";

type Props = {
  sessionId: string;
  live?: boolean;
  isRunning?: boolean;
};

type ArtifactGroup = {
  id: string;
  label: string;
  icon: string;
  items: ArtifactRecord[];
};

export function ArtifactsPanel({ sessionId, live, isRunning = false }: Props) {
  const t = useT();
  const { locale } = useI18n();
  const queryClient = useQueryClient();
  const running = Boolean(isRunning);
  const [showScanned, setShowScanned] = useState(false);
  const [summaryOpen, setSummaryOpen] = useState(false);
  const [summaryReport, setSummaryReport] = useState<ReportDocument | null>(null);

  const artifacts = useQuery({
    queryKey: ["session-artifacts", sessionId, showScanned ? "all" : "final"],
    queryFn: () =>
      api.sessionArtifacts(sessionId, showScanned ? { limit: 100 } : { finalOnly: true, limit: 100 }),
    enabled: Boolean(sessionId),
    staleTime: running ? TRANSCRIPT_STALE_RUNNING_MS : Number.POSITIVE_INFINITY,
    gcTime: SESSION_QUERY_GC_MS,
    refetchInterval: live ? false : false,
    placeholderData: (prev) => prev,
  });

  const scan = useMutation({
    mutationFn: () => api.scanSessionArtifacts(sessionId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["session-artifacts", sessionId] });
    },
  });

  const exportSummary = useMutation({
    mutationFn: () => api.sessionReport(sessionId, locale),
    onSuccess: (data) => {
      setSummaryReport(data.report);
      setSummaryOpen(true);
    },
  });

  const rows = artifacts.data?.artifacts ?? [];
  const groups = groupArtifacts(rows, t);

  const exportSummaryButton = (
    <div className="px-3 py-2 border-t border-outline-variant/40 shrink-0">
      <button
        type="button"
        className="dw-btn-secondary text-xs w-full"
        disabled={exportSummary.isPending || running}
        title={t("conversations.artifactsExportSummaryHint")}
        onClick={() => exportSummary.mutate()}
      >
        <Icon name="description" size={14} className="inline mr-1" />
        {exportSummary.isPending
          ? t("reports.generating")
          : t("conversations.artifactsExportSummary")}
      </button>
      {exportSummary.isError && (
        <p className="text-xs text-error m-0 mt-2">{(exportSummary.error as Error).message}</p>
      )}
      {summaryOpen && summaryReport && (
        <div className="mt-3 max-h-[min(50vh,24rem)] overflow-y-auto rounded-lg border border-outline-variant/50">
          <ReportPreview report={summaryReport} loading={exportSummary.isPending} />
        </div>
      )}
    </div>
  );

  if (artifacts.isPending && !artifacts.data) {
    return <p className="text-sm text-secondary px-4 py-6 m-0">{t("common.loading")}</p>;
  }

  if (rows.length === 0) {
    return (
      <div className="flex flex-col min-h-0 flex-1">
        <div className="p-3 flex-1">
          <EmptyState
            title={t("conversations.artifactsEmpty")}
            description={t("conversations.inspectorArtifactsEmptyDesc")}
            icon="inventory_2"
          />
          <div className="text-center mt-3 flex flex-col items-center gap-2">
            <button
              type="button"
              className="dw-btn-secondary text-xs"
              disabled={scan.isPending}
              onClick={() => scan.mutate()}
            >
              <Icon name="document_scanner" size={14} className="inline mr-1" />
              {scan.isPending ? t("conversations.artifactsScanning") : t("conversations.artifactsScan")}
            </button>
            {!showScanned && (
              <button
                type="button"
                className="text-xs text-secondary underline border-0 bg-transparent cursor-pointer"
                onClick={() => setShowScanned(true)}
              >
                {t("conversations.artifactsShowScanned")}
              </button>
            )}
          </div>
        </div>
        {exportSummaryButton}
      </div>
    );
  }

  return (
    <div className="flex flex-col min-h-0 flex-1">
      <div className="py-1 overflow-y-auto min-h-0 flex-1">
        <div className="px-3 pb-2 flex items-center justify-end">
          <button
            type="button"
            className="text-[11px] text-secondary underline border-0 bg-transparent cursor-pointer"
            onClick={() => setShowScanned((v) => !v)}
          >
            {showScanned
              ? t("conversations.artifactsHideScanned")
              : t("conversations.artifactsShowScanned")}
          </button>
        </div>
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
                      name={artifactIcon(item.kind, item.path)}
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
      {exportSummaryButton}
    </div>
  );
}

function groupArtifacts(rows: ArtifactRecord[], t: ReturnType<typeof useT>): ArtifactGroup[] {
  const presentation: ArtifactRecord[] = [];
  const document: ArtifactRecord[] = [];
  const report: ArtifactRecord[] = [];
  const media: ArtifactRecord[] = [];
  const file: ArtifactRecord[] = [];
  const other: ArtifactRecord[] = [];

  for (const row of rows) {
    const kind = row.kind.toLowerCase();
    const ext = row.path.split(".").pop()?.toLowerCase() ?? "";
    if (kind.includes("presentation") || ext === "pptx" || ext === "ppt") {
      presentation.push(row);
    } else if (kind.includes("document") || ext === "docx" || ext === "doc" || ext === "xlsx") {
      document.push(row);
    } else if (kind.includes("report")) {
      report.push(row);
    } else if (kind.includes("media") || kind.includes("image")) {
      media.push(row);
    } else if (kind.includes("file") || kind === "output" || kind === "artifact" || kind === "notebook") {
      file.push(row);
    } else {
      other.push(row);
    }
  }

  const groups: ArtifactGroup[] = [];
  if (presentation.length > 0) {
    groups.push({
      id: "presentation",
      label: t("conversations.artifactsGroupPresentation"),
      icon: "slideshow",
      items: presentation,
    });
  }
  if (document.length > 0) {
    groups.push({
      id: "document",
      label: t("conversations.artifactsGroupDocument"),
      icon: "description",
      items: document,
    });
  }
  if (report.length > 0) {
    groups.push({
      id: "report",
      label: t("conversations.artifactsGroupReport"),
      icon: "description",
      items: report,
    });
  }
  if (media.length > 0) {
    groups.push({
      id: "media",
      label: t("conversations.artifactsGroupMedia"),
      icon: "image",
      items: media,
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

function artifactIcon(kind: string, path: string): string {
  const lower = kind.toLowerCase();
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  if (lower.includes("presentation") || ext === "pptx" || ext === "ppt") return "slideshow";
  if (lower.includes("report")) return "description";
  if (lower.includes("image") || lower.includes("media")) return "image";
  return "insert_drive_file";
}
