import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { accountCloud } from "@/api/client/accountCloud";
import { CopyButton } from "@/components/ui/CopyButton";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
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
  const [pendingRevokeId, setPendingRevokeId] = useState<string | null>(null);
  const [pendingRevokeName, setPendingRevokeName] = useState("");

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
      setPendingRevokeId(null);
      setPendingRevokeName("");
      refresh();
      void qc.invalidateQueries({ queryKey: ["account-cloud-api-keys"] });
    },
  });

  return (
    <>
      <SectionCard title={t("service.api.cloudKeysTitle")}>
        <p className="text-sm text-secondary m-0 mb-2">{t("service.api.subtitle")}</p>
        <p className="text-xs text-secondary m-0 mb-3">{t("service.api.notLlmKey")}</p>
        <div className="flex flex-wrap gap-2 mb-4">
          <input
            className="dw-input flex-1 min-w-[120px]"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t("settings.tokenNamePlaceholder")}
            aria-label={t("settings.tokenNamePlaceholder")}
          />
          <input
            className="dw-input w-28"
            type="number"
            min={1}
            placeholder={t("settings.tokenExpiresDays")}
            value={expiresDays}
            onChange={(e) => setExpiresDays(e.target.value)}
            aria-label={t("settings.tokenExpiresDays")}
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
          <div className="dw-alert-warn mb-4 flex flex-wrap items-center gap-2" role="alert">
            <span>{t("service.api.saveOnce")}:</span>
            <code className="font-code text-xs break-all flex-1">{createdPlain}</code>
            <CopyButton text={createdPlain} />
            <button
              type="button"
              className="dw-btn-ghost text-xs"
              onClick={() => setCreatedPlain(null)}
            >
              {t("service.api.savedDismiss")}
            </button>
          </div>
        )}
        <div className="overflow-x-auto -mx-4 px-4">
          <table className="dw-table">
            <thead>
              <tr>
                <th scope="col">{t("common.name")}</th>
                <th scope="col">{t("common.prefix")}</th>
                <th scope="col">{t("audit.time")}</th>
                <th scope="col">{t("settings.tokenExpires")}</th>
                <th scope="col">{t("common.status")}</th>
                <th scope="col" />
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
                        className="dw-btn-ghost text-xs"
                        aria-label={t("service.api.revokeAria").replace("{name}", tok.name)}
                        onClick={() => {
                          setPendingRevokeId(tok.id);
                          setPendingRevokeName(tok.name);
                        }}
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

      <ModalOverlay
        open={pendingRevokeId != null}
        onClose={() => {
          setPendingRevokeId(null);
          setPendingRevokeName("");
        }}
        labelledBy="revoke-key-title"
      >
        <div className="glass-modal rounded-xl p-6 max-w-md">
          <h2 id="revoke-key-title" className="text-lg font-semibold m-0 mb-2">
            {t("service.api.revokeTitle")}
          </h2>
          <p className="text-sm text-secondary m-0 mb-4">
            {t("service.api.revokeBody").replace("{name}", pendingRevokeName)}
          </p>
          <div className="flex flex-wrap gap-2 justify-end">
            <button
              type="button"
              className="dw-btn-secondary"
              onClick={() => {
                setPendingRevokeId(null);
                setPendingRevokeName("");
              }}
            >
              {t("service.plan.cancel")}
            </button>
            <button
              type="button"
              className="dw-btn-primary"
              disabled={revoke.isPending}
              onClick={() => pendingRevokeId && revoke.mutate(pendingRevokeId)}
            >
              {t("service.api.revokeConfirm")}
            </button>
          </div>
        </div>
      </ModalOverlay>
    </>
  );
}
