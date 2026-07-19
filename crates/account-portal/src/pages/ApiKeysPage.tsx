import { useEffect, useState } from "react";
import { api } from "../api";
import { ConsolePage } from "../components/ConsolePage";
import { useT } from "../i18n/context";

type ApiKey = {
  id: string;
  name: string;
  prefix: string;
  created_at: string;
  expires_at: string | null;
  last_used_at: string | null;
  revoked: boolean;
};

export function ApiKeysPage() {
  const t = useT();
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [name, setName] = useState("");
  const [plaintext, setPlaintext] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [revokeTarget, setRevokeTarget] = useState<ApiKey | null>(null);

  const refresh = () => {
    void api.listApiKeys().then((r) => setKeys(r.keys)).catch(() => setKeys([]));
  };

  useEffect(() => {
    refresh();
  }, []);

  const createKey = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    setPending(true);
    setMsg(null);
    setPlaintext(null);
    try {
      const res = await api.createApiKey({ name: name.trim() });
      setPlaintext(res.plaintext);
      setName("");
      refresh();
    } catch (err) {
      setMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setPending(false);
    }
  };

  return (
    <ConsolePage title={t("console.api")} description={t("api.description")}>
      <div className="card nx-api-create">
        <div className="nx-api-create__intro"><span>ACCESS CREDENTIAL</span><h3>{t("api.createTitle")}</h3><p className="muted">{t("api.copyOnce")}</p></div>
        <form className="api-key-form" onSubmit={createKey}>
          <label className="field-label">{t("api.nameLabel")}</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t("api.namePlaceholder")}
            required
          />
          <button className="btn btn-primary" type="submit" disabled={pending}>
            {pending ? t("api.creating") : t("api.createAction")}
          </button>
        </form>
        {plaintext && (
          <div className="api-key-reveal">
            <p className="form-note">{t("api.copyOnce")}</p>
            <code className="api-key-plaintext">{plaintext}</code>
          </div>
        )}
        {msg && <p className="form-error">{msg}</p>}
      </div>

      <div className="card table-wrap nx-data-panel">
        <table>
          <thead>
            <tr>
              <th>{t("api.colName")}</th>
              <th>{t("api.colPrefix")}</th>
              <th>{t("api.colCreated")}</th>
              <th>{t("api.colLastUsed")}</th>
              <th>{t("api.colStatus")}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {keys.length === 0 && <tr><td colSpan={6} className="nx-table-empty">{t("common.empty")}</td></tr>}
            {keys.map((k) => (
              <tr key={k.id}>
                <td>{k.name}</td>
                <td>
                  <code>{k.prefix}…</code>
                </td>
                <td>{k.created_at}</td>
                <td>{k.last_used_at ?? "—"}</td>
                <td>{k.revoked ? t("api.statusRevoked") : t("api.statusActive")}</td>
                <td>
                  {!k.revoked && (
                    <button
                      className="btn btn-ghost btn-sm"
                      type="button"
                      onClick={() => setRevokeTarget(k)}
                    >
                      {t("api.revoke")}
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {revokeTarget && (
        <div className="nx-modal-backdrop" role="presentation" onMouseDown={() => setRevokeTarget(null)}>
          <div className="nx-confirm-modal" role="dialog" aria-modal="true" aria-labelledby="revoke-title" onMouseDown={(event) => event.stopPropagation()}>
            <span>REVOKE CREDENTIAL</span>
            <h3 id="revoke-title">{t("api.revoke")}: {revokeTarget.name}</h3>
            <code>{revokeTarget.prefix}…</code>
            <div><button className="btn btn-secondary" type="button" onClick={() => setRevokeTarget(null)}>{t("common.cancel")}</button><button className="btn btn-primary" type="button" onClick={() => { void api.revokeApiKey(revokeTarget.id).then(() => { setRevokeTarget(null); refresh(); }); }}>{t("common.confirm")}</button></div>
          </div>
        </div>
      )}
    </ConsolePage>
  );
}
