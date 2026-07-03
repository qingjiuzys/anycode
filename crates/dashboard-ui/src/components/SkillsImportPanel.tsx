import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { ExternalNavLink } from "@/components/ExternalNavLink";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

export function SkillsImportPanel() {
  const t = useT();
  const qc = useQueryClient();
  const [source, setSource] = useState("");
  const [msg, setMsg] = useState("");
  const [isError, setIsError] = useState(false);

  const importSkill = useMutation({
    mutationFn: () => api.importSkill(source.trim()),
    onSuccess: (data) => {
      setIsError(false);
      setMsg(`${t("settings.skillsGov.importOk")}: ${data.id}`);
      void qc.invalidateQueries({ queryKey: ["skills"] });
      void qc.invalidateQueries({ queryKey: ["skill-market"] });
      void qc.invalidateQueries({ queryKey: ["skill-suggestions"] });
      void qc.invalidateQueries({ queryKey: ["overview"] });
      setSource("");
    },
    onError: (e: Error) => {
      setIsError(true);
      setMsg(e.message);
    },
  });

  return (
    <div className="dw-agents-import-panel">
      <p className="text-sm text-secondary m-0">{t("agents.importTabHint")}</p>

      <div className="rounded-lg border border-outline-variant/60 bg-surface-container-low px-4 py-3 text-sm text-secondary">
        <p className="m-0 font-medium text-on-surface">{t("agents.skillMarketDiscoverTitle")}</p>
        <p className="m-0 mt-1">{t("agents.skillMarketDiscoverHint")}</p>
        <div className="flex flex-wrap items-center gap-2 mt-2">
          <ExternalNavLink
            href="https://skills.sh"
            className="inline-flex items-center gap-1 text-primary no-underline hover:underline"
          >
            skills.sh
            <Icon name="open_in_new" size={14} />
          </ExternalNavLink>
        </div>
        <p className="m-0 mt-2 font-code text-[11px] text-outline break-all">
          {t("agents.skillMarketImportExample")}
        </p>
      </div>

      <div className="flex flex-wrap gap-2">
        <input
          className="dw-input flex-1 min-w-[240px]"
          placeholder={t("settings.skillsGov.importPlaceholder")}
          value={source}
          onChange={(e) => setSource(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && source.trim()) importSkill.mutate();
          }}
        />
        <button
          type="button"
          className="dw-btn-primary shrink-0"
          disabled={!source.trim() || importSkill.isPending}
          onClick={() => importSkill.mutate()}
        >
          <Icon name="upload" size={16} />
          {importSkill.isPending ? t("agents.rescanning") : t("settings.skillsGov.importBtn")}
        </button>
      </div>

      {msg && (
        <p className={`text-sm m-0 ${isError ? "text-error" : "text-secondary"}`} role="status">
          {msg}
        </p>
      )}
    </div>
  );
}
