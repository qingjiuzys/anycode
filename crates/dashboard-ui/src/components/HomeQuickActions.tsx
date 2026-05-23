import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function HomeQuickActions({
  onNewProject,
}: {
  onNewProject: () => void;
}) {
  const t = useT();
  const qc = useQueryClient();
  const scan = useMutation({
    mutationFn: api.scanProjects,
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["projects"] });
      void qc.invalidateQueries({ queryKey: ["overview"] });
      void qc.invalidateQueries({ queryKey: ["bootstrap"] });
    },
  });

  return (
    <SectionCard title={t("home.quickActions")}>
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className="dw-btn-secondary"
          disabled={scan.isPending}
          onClick={() => scan.mutate()}
        >
          <Icon name="radar" size={16} />
          {scan.isPending ? t("common.loading") : t("home.actionScan")}
        </button>
        <button type="button" className="dw-btn-secondary" onClick={onNewProject}>
          <Icon name="add" size={16} />
          {t("home.actionNewProject")}
        </button>
        <Link to="/conversations" className="dw-btn-secondary no-underline">
          <Icon name="forum" size={16} />
          {t("home.actionConversations")}
        </Link>
        <Link to="/settings" className="dw-btn-secondary no-underline">
          <Icon name="settings" size={16} />
          {t("home.actionSettings")}
        </Link>
      </div>
    </SectionCard>
  );
}
