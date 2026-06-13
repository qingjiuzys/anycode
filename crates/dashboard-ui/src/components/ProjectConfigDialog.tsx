import { useEffect, useState } from "react";
import { Icon } from "@/components/Icon";
import { ProjectGateConfigPanel } from "@/components/project/ProjectGateConfigPanel";
import { ProjectKnowledgeConfigPanel } from "@/components/project/ProjectKnowledgeConfigPanel";
import { ProjectPipelineConfigPanel } from "@/components/project/ProjectPipelineConfigPanel";
import { useT } from "@/i18n/context";
import { ModalOverlay } from "@/components/ui/ModalOverlay";

type Tab = "knowledge" | "gates" | "pipeline";

const TAB_ICONS: Record<Tab, string> = {
  knowledge: "folder",
  gates: "verified",
  pipeline: "account_tree",
};

export function ProjectConfigDialog({
  projectId,
  open,
  initialTab,
  onClose,
}: {
  projectId: string;
  open: boolean;
  initialTab?: Tab;
  onClose: () => void;
}) {
  const t = useT();
  const [tab, setTab] = useState<Tab>(initialTab ?? "knowledge");
  const [knowledgeDirty, setKnowledgeDirty] = useState(false);

  useEffect(() => {
    if (open && initialTab) {
      setTab(initialTab);
    }
  }, [open, initialTab]);

  if (!open) return null;

  const tabs: { id: Tab; label: string }[] = [
    { id: "knowledge", label: t("projectDetail.config.tabs.knowledge") },
    { id: "gates", label: t("projectDetail.config.tabs.gates") },
    { id: "pipeline", label: t("projectDetail.config.tabs.pipeline") },
  ];

  return (
    <ModalOverlay
      open={open}
      onClose={onClose}
      labelledBy="project-config-title"
      className="w-full max-w-3xl"
    >
      <div className="glass-modal max-h-[min(90dvh,720px)] flex flex-col rounded-xl overflow-hidden">
        <div className="flex items-start justify-between gap-4 px-6 pt-6 pb-3 shrink-0">
          <div>
            <h2 id="project-config-title" className="text-lg font-semibold m-0 text-on-surface">
              {t("projectDetail.config.title")}
            </h2>
            <p className="text-sm text-secondary m-0 mt-1">{t("projectDetail.config.subtitle")}</p>
          </div>
          <button
            type="button"
            className="dw-btn-ghost p-1 shrink-0"
            onClick={onClose}
            aria-label={t("projectDetail.config.close")}
          >
            <Icon name="close" size={20} />
          </button>
        </div>

        <div className="flex gap-1 px-6 border-b border-outline-variant shrink-0">
          {tabs.map((item) => (
            <button
              key={item.id}
              type="button"
              className={`inline-flex items-center gap-1.5 px-3 py-2 text-sm border-b-2 -mb-px transition-colors ${
                tab === item.id
                  ? "border-primary text-primary font-medium"
                  : "border-transparent text-secondary hover:text-on-surface"
              }`}
              onClick={() => setTab(item.id)}
            >
              <Icon name={TAB_ICONS[item.id]} size={16} />
              {item.label}
              {item.id === "knowledge" && knowledgeDirty && (
                <span
                  className="w-1.5 h-1.5 rounded-full bg-warn"
                  title={t("projectDetail.config.unsavedDot")}
                />
              )}
            </button>
          ))}
        </div>

        <div className="flex-1 min-h-0 overflow-y-auto px-6 py-4">
          {tab === "knowledge" && (
            <ProjectKnowledgeConfigPanel
              projectId={projectId}
              onDirtyChange={setKnowledgeDirty}
            />
          )}
          {tab === "gates" && <ProjectGateConfigPanel projectId={projectId} />}
          {tab === "pipeline" && <ProjectPipelineConfigPanel projectId={projectId} />}
        </div>

        <div className="px-6 py-3 border-t border-outline-variant shrink-0 flex justify-end">
          <button type="button" className="dw-btn-primary" onClick={onClose}>
            {t("projectDetail.config.close")}
          </button>
        </div>
      </div>
    </ModalOverlay>
  );
}
