import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

const AGENTS = [
  "general-purpose",
  "explore",
  "plan",
  "workspace-assistant",
  "goal",
] as const;

export function PromptPreviewPanel() {
  const t = useT();
  const qc = useQueryClient();
  const [agent, setAgent] = useState<string>("general-purpose");
  const [cwd, setCwd] = useState("");
  const [appendDraft, setAppendDraft] = useState("");
  const [overrideDraft, setOverrideDraft] = useState("");

  const preview = useQuery({
    queryKey: ["prompt-preview", agent, cwd],
    queryFn: () =>
      api.promptPreview({
        agent,
        cwd: cwd.trim() || undefined,
      }),
  });

  useEffect(() => {
    if (preview.data) {
      setAppendDraft(preview.data.system_prompt_append ?? "");
      setOverrideDraft(preview.data.system_prompt_override ?? "");
    }
  }, [preview.data?.system_prompt_append, preview.data?.system_prompt_override]);

  const save = useMutation({
    mutationFn: () =>
      api.setPromptSettings({
        system_prompt_append: appendDraft.trim() || null,
        system_prompt_override: overrideDraft.trim() || null,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["prompt-preview"] });
    },
  });

  const segments = preview.data?.segments ?? [];
  const composed = preview.data?.composed ?? "";

  return (
    <SectionCard title={t("settings.promptPreview.title")}>
      <p className="text-sm text-secondary m-0 mb-3">{t("settings.promptPreview.hint")}</p>
      <div className="flex flex-wrap gap-2 mb-3">
        <label className="flex flex-col gap-1 text-xs text-secondary">
          {t("settings.promptPreview.agent")}
          <select
            className="dw-input text-sm min-w-[10rem]"
            value={agent}
            onChange={(e) => setAgent(e.target.value)}
          >
            {AGENTS.map((id) => (
              <option key={id} value={id}>
                {id}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-xs text-secondary flex-1 min-w-[14rem]">
          {t("settings.promptPreview.cwd")}
          <input
            className="dw-input text-sm font-code"
            placeholder={t("settings.promptPreview.cwdPlaceholder")}
            value={cwd}
            onChange={(e) => setCwd(e.target.value)}
          />
        </label>
        <button
          type="button"
          className="dw-btn-secondary text-sm self-end"
          disabled={preview.isFetching}
          onClick={() => void preview.refetch()}
        >
          {preview.isFetching ? "…" : t("settings.promptPreview.refresh")}
        </button>
      </div>

      <div className="grid gap-3 mb-3 md:grid-cols-2">
        <label className="flex flex-col gap-1 text-xs text-secondary">
          {t("settings.promptPreview.appendLabel")}
          <textarea
            className="dw-input font-code text-xs min-h-[5rem]"
            value={appendDraft}
            onChange={(e) => setAppendDraft(e.target.value)}
            spellCheck={false}
          />
        </label>
        <label className="flex flex-col gap-1 text-xs text-secondary">
          {t("settings.promptPreview.overrideLabel")}
          <textarea
            className="dw-input font-code text-xs min-h-[5rem]"
            value={overrideDraft}
            onChange={(e) => setOverrideDraft(e.target.value)}
            spellCheck={false}
            placeholder={t("settings.promptPreview.overridePlaceholder")}
          />
        </label>
      </div>
      <div className="flex items-center gap-2 mb-3">
        <button
          type="button"
          className="dw-btn-primary inline-flex items-center gap-2 text-sm"
          disabled={save.isPending}
          onClick={() => save.mutate()}
        >
          <Icon name="save" size={16} />
          {t("settings.promptPreview.save")}
        </button>
        {save.isSuccess && (
          <span className="text-xs text-secondary">{t("settings.promptPreview.saved")}</span>
        )}
      </div>

      {preview.isError && (
        <p className="text-sm text-error m-0 mb-2">{(preview.error as Error).message}</p>
      )}

      {segments.length > 0 && (
        <div className="flex flex-col gap-2 mb-3">
          {segments.map((seg) => (
            <details key={seg.id} className="border border-outline-variant rounded-md p-2">
              <summary className="text-xs font-code cursor-pointer">
                {seg.id} · {seg.chars} {t("settings.promptPreview.chars")}
              </summary>
              <pre className="text-xs whitespace-pre-wrap mt-2 m-0 text-secondary max-h-48 overflow-auto">
                {seg.text}
              </pre>
            </details>
          ))}
        </div>
      )}

      <label className="flex flex-col gap-1 text-xs text-secondary">
        {t("settings.promptPreview.composed")}
        <textarea
          className="dw-input font-code text-xs min-h-[14rem] w-full"
          readOnly
          value={composed}
          spellCheck={false}
        />
      </label>
    </SectionCard>
  );
}
