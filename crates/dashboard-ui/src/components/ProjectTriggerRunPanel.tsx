import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { api } from "@/api/client";
import type { WebChatResult } from "@/api/client/projects";
import { ConversationComposer } from "@/components/ConversationComposer";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function ProjectTriggerRunPanel({ projectId }: { projectId: string }) {
  const t = useT();
  const [lastChat, setLastChat] = useState<WebChatResult | null>(null);

  const recent = useQuery({
    queryKey: ["project-triggers", projectId],
    queryFn: () => api.listProjectTriggers(projectId),
    staleTime: 30_000,
  });

  return (
    <SectionCard title={t("projectDetail.triggerRun")}>
      <p className="text-xs text-secondary m-0 mb-3">{t("projectDetail.triggerRunHint")}</p>
      <ConversationComposer
        mode="start"
        projectId={projectId}
        compact
        onSuccess={({ chat }) => setLastChat(chat)}
      />

      {lastChat && (
        <div className="mt-3 p-3 rounded-md bg-surface-container-low text-sm">
          <div className="font-medium">{t("projectDetail.triggerStarted")}</div>
          <div className="text-secondary font-code text-xs mt-1 break-all">
            pid={lastChat.pid} · {lastChat.log_path}
          </div>
          <Link to="/conversations" className="text-primary text-xs hover:underline">
            {t("projectDetail.triggerWatchSessions")}
          </Link>
        </div>
      )}

      {(recent.data?.triggers ?? []).length > 0 && (
        <div className="mt-4">
          <div className="text-xs text-secondary mb-2">{t("projectDetail.triggerRecent")}</div>
          <ul className="m-0 pl-5 text-xs text-secondary space-y-1 font-code">
            {(recent.data?.triggers ?? []).slice(0, 5).map((tr) => (
              <li key={tr.trigger_id}>
                {tr.started_at} · {tr.kind} · pid {tr.pid}
              </li>
            ))}
          </ul>
        </div>
      )}
    </SectionCard>
  );
}
