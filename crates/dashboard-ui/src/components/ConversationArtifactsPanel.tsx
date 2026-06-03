import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { ArtifactRecord } from "@/api/types";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type Props = {
  sessionId: string | null;
  className?: string;
  live?: boolean;
};

type ArtifactGroup = {
  id: string;
  label: string;
  icon: string;
  items: ArtifactRecord[];
};

export function ConversationArtifactsPanel({ sessionId, className = "", live }: Props) {
  const t = useT();

  const artifacts = useQuery({
    queryKey: ["session-artifacts", sessionId],
    queryFn: () => api.sessionArtifacts(sessionId!),
    enabled: Boolean(sessionId),
    refetchInterval: live ? 5_000 : false,
  });

  if (!sessionId) {
    return (
      <aside className={`flex flex-col h-full min-h-0 bg-surface-container-lowest ${className}`}>
        <PanelHeader count={0} />
        <div className="flex-1 flex items-center justify-center p-4">
          <p className="text-sm text-secondary m-0 text-center">{t("conversations.selectSession")}</p>
        </div>
      </aside>
    );
  }

  if (artifacts.isLoading) {
    return (
      <aside className={`flex flex-col h-full min-h-0 bg-surface-container-lowest ${className}`}>
        <PanelHeader count={0} />
        <p className="text-sm text-secondary px-4 py-6 m-0">{t("common.loading")}</p>
      </aside>
    );
  }

  const rows = artifacts.data?.artifacts ?? [];
  const groups = groupArtifacts(rows, t);

  return (
    <aside className={`flex flex-col h-full min-h-0 bg-surface-container-lowest ${className}`}>
      <PanelHeader count={rows.length} />
      <div className="flex-1 min-h-0 overflow-y-auto">
        {rows.length === 0 ? (
          <div className="p-4">
            <EmptyState
              title={t("conversations.artifactsEmpty")}
              description={t("conversations.artifactsEmptyDesc")}
              icon="inventory_2"
            />
            <div className="text-center mt-4">
              <Link
                to="/sessions/$sessionId"
                params={{ sessionId }}
                className="text-xs text-primary no-underline hover:underline inline-flex items-center gap-1"
              >
                <Icon name="open_in_new" size={14} />
                {t("conversations.openDetail")}
              </Link>
            </div>
          </div>
        ) : (
          <div className="py-2">
            {groups.map((group) => (
              <section key={group.id} className="mb-4">
                <h4 className="px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wide text-secondary m-0 flex items-center gap-1.5">
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
        )}
      </div>
    </aside>
  );
}

function PanelHeader({ count }: { count: number }) {
  const t = useT();
  return (
    <div className="px-3 py-2 text-xs font-semibold uppercase tracking-wide text-secondary border-b border-outline-variant bg-surface-container-low shrink-0 flex items-center justify-between gap-2">
      <span className="inline-flex items-center gap-1.5">
        <Icon name="inventory_2" size={14} />
        {t("conversations.artifactsPanel")}
      </span>
      {count > 0 && <span className="text-[10px] font-normal normal-case">{count}</span>}
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
