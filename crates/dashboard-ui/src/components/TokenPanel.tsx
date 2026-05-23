import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { CopyButton } from "@/components/ui/CopyButton";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function TokenPanel() {
  const t = useT();
  const qc = useQueryClient();
  const [name, setName] = useState("dashboard");
  const [expiresDays, setExpiresDays] = useState("");
  const [createdPlain, setCreatedPlain] = useState<string | null>(null);
  const tokens = useQuery({ queryKey: ["api-tokens"], queryFn: api.apiTokens });
  const runtime = useQuery({ queryKey: ["runtime-settings"], queryFn: api.runtimeSettings });

  const create = useMutation({
    mutationFn: () =>
      api.createToken(name, expiresDays ? Number(expiresDays) : undefined),
    onSuccess: (data) => {
      setCreatedPlain(data.plaintext);
      qc.invalidateQueries({ queryKey: ["api-tokens"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
    },
  });
  const revoke = useMutation({
    mutationFn: (id: string) => api.revokeToken(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["api-tokens"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  const authMode = runtime.data?.runtime.auth_mode;
  const needsToken = authMode === "token_required";

  return (
    <SectionCard title={t("settings.apiToken")}>
      {needsToken && (
        <div className="dw-alert-warn mb-3">{t("settings.nonLoopbackWarn")}</div>
      )}
      <p className="text-sm text-secondary m-0 mb-3">{t("settings.tokenHint")}</p>
      <div className="flex flex-wrap gap-2 mb-4">
        <input
          className="dw-input flex-1 min-w-[120px]"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t("settings.tokenNamePlaceholder")}
        />
        <input
          className="dw-input w-28"
          type="number"
          min={1}
          placeholder={t("settings.tokenExpiresDays")}
          value={expiresDays}
          onChange={(e) => setExpiresDays(e.target.value)}
        />
        <button
          type="button"
          className="dw-btn-secondary"
          disabled={create.isPending}
          onClick={() => create.mutate()}
        >
          {t("common.create")}
        </button>
      </div>
      {createdPlain && (
        <div className="dw-alert-error mb-4 flex flex-wrap items-center gap-2">
          <span>{t("settings.tokenSaveOnce")}:</span>
          <code className="font-code text-xs break-all flex-1">{createdPlain}</code>
          <CopyButton text={createdPlain} />
        </div>
      )}
      <div className="overflow-x-auto -mx-4 px-4">
        <table className="dw-table">
          <thead>
            <tr>
              <th>{t("common.name")}</th>
              <th>{t("common.prefix")}</th>
              <th>{t("audit.time")}</th>
              <th>{t("settings.tokenExpires")}</th>
              <th>{t("settings.tokenLastUsed")}</th>
              <th>{t("common.status")}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {(tokens.data?.tokens ?? []).map((tok) => (
              <tr key={tok.id}>
                <td>{tok.name}</td>
                <td>
                  <code className="font-code">{tok.prefix}</code>
                </td>
                <td className="text-secondary text-xs">{tok.created_at}</td>
                <td className="text-secondary text-xs">{tok.expires_at ?? "—"}</td>
                <td className="text-secondary text-xs">{tok.last_used_at ?? "—"}</td>
                <td>{tok.revoked ? t("common.revoked") : t("common.valid")}</td>
                <td>
                  {!tok.revoked && (
                    <button
                      type="button"
                      className="dw-btn-ghost"
                      onClick={() => revoke.mutate(tok.id)}
                    >
                      {t("common.revoke")}
                    </button>
                  )}
                </td>
              </tr>
            ))}
            {(tokens.data?.tokens ?? []).length === 0 && (
              <tr>
                <td colSpan={7} className="text-secondary text-center py-4">
                  {t("settings.noTokens")}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </SectionCard>
  );
}
