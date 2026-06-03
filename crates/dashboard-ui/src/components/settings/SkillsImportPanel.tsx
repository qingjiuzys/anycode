import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function SkillsImportPanel() {
  const t = useT();
  const qc = useQueryClient();
  const [source, setSource] = useState("");
  const [msg, setMsg] = useState("");

  const importSkill = useMutation({
    mutationFn: () => api.importSkill(source.trim()),
    onSuccess: (data) => {
      setMsg(`${t("settings.skillsGov.importOk")}: ${data.id}`);
      void qc.invalidateQueries({ queryKey: ["skills"] });
      setSource("");
    },
    onError: (e: Error) => setMsg(e.message),
  });

  return (
    <SectionCard title={t("settings.skillsGov.importTitle")}>
      <p className="text-sm text-secondary m-0 mb-3">{t("settings.skillsGov.importHint")}</p>
      <div className="flex flex-wrap gap-2">
        <input
          className="dw-input flex-1 min-w-[200px]"
          placeholder={t("settings.skillsGov.importPlaceholder")}
          value={source}
          onChange={(e) => setSource(e.target.value)}
        />
        <button
          type="button"
          className="dw-btn-secondary"
          disabled={!source.trim() || importSkill.isPending}
          onClick={() => importSkill.mutate()}
        >
          {importSkill.isPending ? "…" : t("settings.skillsGov.importBtn")}
        </button>
      </div>
      {msg && <p className="text-sm text-secondary mt-2 m-0">{msg}</p>}
    </SectionCard>
  );
}
