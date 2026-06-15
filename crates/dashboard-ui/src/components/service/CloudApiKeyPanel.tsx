import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { accountCloud } from "@/api/client/accountCloud";
import { CopyButton } from "@/components/ui/CopyButton";
import { SectionCard } from "@/components/ui/SectionCard";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function CloudApiKeyPanel() {
  const t = useT();
  const { baseUrl, refresh } = useAccountCloud();
  const qc = useQueryClient();
  const [name, setName] = useState("integration");
  const [expiresDays, setExpiresDays] = useState("");
  const [createdPlain, setCreatedPlain] = useState<string | null>(null);

  const keys = useQuery({
    queryKey: ["account-cloud-api-keys"],
    queryFn: () => accountCloud.listApiKeys(baseUrl!),
    enabled: Boolean(baseUrl),
  });

  const create = useMutation({
    mutationFn: () =>
      accountCloud.createApiKey(baseUrl!, {
        name,
        expires_days: expiresDays ? Number(expiresDays) : undefined,
      }),
    onSuccess: (data) => {
      setCreatedPlain(data.plaintext);
      refresh();
      void qc.invalidateQueries({ queryKey: ["account-cloud-api-keys"] });
    },
  });

  const revoke = useMutation({
    mutationFn: (id: string) => accountCloud.revokeApiKey(baseUrl!, id),
    onSuccess: () => {
      refresh();
      void qc.invalidateQueries({ queryKey: ["account-cloud-api-keys"] });
    },
  });

  return (
    <SectionCard title={t("service.api.cloudKeysTitle")}>
      <p className="text-sm text-secondary m-0 mb-3">{t("service.api.cloudKeysHint")}</p>
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
          disabled={create.isPending || !baseUrl}
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
              <th>{t("common.status")}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {(keys.data?.keys ?? []).map((tok) => (
              <tr key={tok.id}>
                <td>{tok.name}</td>
                <td>
                  <code className="font-code">{tok.prefix}</code>
                </td>
                <td className="text-secondary text-xs">{tok.created_at}</td>
                <td className="text-secondary text-xs">{tok.expires_at ?? "—"}</td>
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
            {(keys.data?.keys ?? []).length === 0 && (
              <tr>
                <td colSpan={6} className="text-secondary text-center py-4">
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
