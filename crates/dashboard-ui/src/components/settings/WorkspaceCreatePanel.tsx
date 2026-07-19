import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Icon } from "@/components/Icon";
import { NewProjectDialog } from "@/components/NewProjectDialog";
import { SectionCard } from "@/components/ui/SectionCard";
import { useControlCenter } from "@/context/ControlCenterContext";
import { useT } from "@/i18n/context";
import { buildCreateViaChatPrompt } from "@/lib/automationChatPrompts";
import { setComposerSeed } from "@/lib/composerSeed";

/** Create project / cron from Settings (cron opens chat seed, not a form). */
export function WorkspaceCreatePanel() {
  const t = useT();
  const navigate = useNavigate();
  const { closeControlCenter } = useControlCenter();
  const [projectOpen, setProjectOpen] = useState(false);

  const openCronViaChat = () => {
    setComposerSeed(buildCreateViaChatPrompt(t));
    closeControlCenter();
    void navigate({ to: "/" });
  };

  return (
    <SectionCard title={t("settings.workspaceCreate.title")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("settings.workspaceCreate.hint")}</p>
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className="dw-btn-secondary text-sm inline-flex items-center gap-2"
          onClick={() => setProjectOpen(true)}
        >
          <Icon name="folder" size={16} />
          {t("layout.newProject")}
        </button>
        <button
          type="button"
          className="dw-btn-secondary text-sm inline-flex items-center gap-2"
          onClick={openCronViaChat}
        >
          <Icon name="settings_suggest" size={16} />
          {t("layout.newCronJob")}
        </button>
      </div>
      <NewProjectDialog open={projectOpen} onClose={() => setProjectOpen(false)} />
    </SectionCard>
  );
}
