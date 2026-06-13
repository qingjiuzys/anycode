import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import { ModalOverlay } from "@/components/ui/ModalOverlay";

export function SkillsImportDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const t = useT();
  const qc = useQueryClient();
  const [source, setSource] = useState("");
  const [msg, setMsg] = useState("");

  const importSkill = useMutation({
    mutationFn: () => api.importSkill(source.trim()),
    onSuccess: (data) => {
      setMsg(`${t("settings.skillsGov.importOk")}: ${data.id}`);
      void qc.invalidateQueries({ queryKey: ["skills"] });
      void qc.invalidateQueries({ queryKey: ["skill-market"] });
      void qc.invalidateQueries({ queryKey: ["skill-suggestions"] });
      void qc.invalidateQueries({ queryKey: ["overview"] });
      setSource("");
    },
    onError: (e: Error) => setMsg(e.message),
  });

  if (!open) return null;

  return (
    <ModalOverlay open={open} onClose={onClose} labelledBy="skills-import-title" className="w-full max-w-lg">
      <div className="glass-modal rounded-xl p-6">
        <div className="flex items-start justify-between gap-4 mb-4">
          <div>
            <h2 id="skills-import-title" className="text-lg font-semibold m-0 text-on-surface">
              {t("settings.skillsGov.importTitle")}
            </h2>
            <p className="text-sm text-secondary m-0 mt-1">{t("settings.skillsGov.importHint")}</p>
            <p className="text-[11px] font-code text-outline m-0 mt-2 break-all">
              {t("agents.skillMarketImportExample")}
            </p>
          </div>
          <button type="button" className="dw-btn-ghost p-1" onClick={onClose} aria-label={t("common.cancel")}>
            <Icon name="close" size={20} />
          </button>
        </div>
        <div className="flex flex-wrap gap-2">
          <input
            className="dw-input flex-1 min-w-[200px]"
            placeholder={t("settings.skillsGov.importPlaceholder")}
            value={source}
            onChange={(e) => setSource(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && source.trim()) importSkill.mutate();
            }}
          />
          <button
            type="button"
            className="dw-btn-primary"
            disabled={!source.trim() || importSkill.isPending}
            onClick={() => importSkill.mutate()}
          >
            {importSkill.isPending ? "…" : t("settings.skillsGov.importBtn")}
          </button>
        </div>
        {msg && (
          <p className={`text-sm mt-3 m-0 ${importSkill.isError ? "text-error" : "text-secondary"}`}>
            {msg}
          </p>
        )}
      </div>
    </ModalOverlay>
  );
}
