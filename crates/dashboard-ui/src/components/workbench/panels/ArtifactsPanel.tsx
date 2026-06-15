import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { ArtifactRecord } from "@/api/types";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import { sessionArtifactsQueryOptions } from "@/lib/sessionQuery";

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
  const queryClient = useQueryClient();
  const running = Boolean(isRunning);

  const artifacts = useQuery({
    ...sessionArtifactsQueryOptions(sessionId, running),
    enabled: Boolean(sessionId),
    refetchInterval: live ? false : false,
    placeholderData: (prev) => prev,
  });

  const scan = useMutation({
    mutationFn: () => api.scanSessionArtifacts(sessionId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["session-artifacts", sessionId] });
    },
  });

  const rows = artifacts.data?.artifacts ?? [];
  const groups = groupArtifacts(rows, t);

  if (artifacts.isPending && !artifacts.data) {
    return <p className="text-sm text-secondary px-4 py-6 m-0">{t("common.loading")}</p>;
  }

  if (rows.length === 0) {
    return (
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
    );
  }

  return (
    <div className="py-1 overflow-y-auto min-h-0 flex-1">
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
