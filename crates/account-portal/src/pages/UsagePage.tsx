import { useEffect, useState } from "react";
import { api } from "../api";
import { ConsolePage } from "../components/ConsolePage";
import { useAuth } from "../hooks/useAuth";
import { useT } from "../i18n/context";

type Model = {
  id: string;
  display_name: string;
  provider_id: string;
  min_plan: string;
  available: boolean;
  price_per_1m_input_cny: number;
};

export function UsagePage() {
  const t = useT();
  const { logout } = useAuth();
  const [usage, setUsage] = useState<{
    tokens_used: number;
    token_limit: number;
    by_model: Array<{ model_id: string; total_tokens: number }>;
  } | null>(null);
  const [models, setModels] = useState<Model[]>([]);
  const [usageError, setUsageError] = useState<string | null>(null);
  const [modelsError, setModelsError] = useState<string | null>(null);

  const load = () => {
    setUsageError(null);
    setModelsError(null);
    void api
      .usage()
      .then((usageResponse) => setUsage(usageResponse.usage))
      .catch((err) => {
        setUsage(null);
        setUsageError(err instanceof Error ? err.message : String(err));
      });
    void api
      .models()
      .then((modelResponse) => setModels(modelResponse.models))
      .catch((err) => {
        setModels([]);
        setModelsError(err instanceof Error ? err.message : String(err));
      });
  };

  useEffect(load, []);

  const percentage = usage
    ? Math.min(100, Math.round((usage.tokens_used / Math.max(usage.token_limit, 1)) * 100))
    : 0;

  const authError = usageError?.startsWith("401:") || modelsError?.startsWith("401:");

  return (
    <ConsolePage title={t("console.usage")} description={t("usage.description")}>
      {authError && (
        <div className="nx-empty-state" role="alert">
          <strong>{t("common.sessionExpired")}</strong>
          <button className="btn btn-primary btn-sm" type="button" onClick={() => logout()}>
            {t("common.signInAgain")}
          </button>
        </div>
      )}

      {usageError && !authError && (
        <div className="nx-empty-state" role="alert">
          <strong>{t("usage.summaryError")}</strong>
          <p className="muted form-note">{usageError}</p>
          <button className="btn btn-secondary btn-sm" type="button" onClick={load}>
            {t("common.retry")}
          </button>
        </div>
      )}

      {!usage && !usageError && <p className="muted">{t("common.loading")}</p>}

      {usage && (
        <div className="card nx-usage-summary">
          <div className="nx-section-heading">
            <div>
              <span>TOKEN TELEMETRY</span>
              <h3>{t("usage.summaryTitle")}</h3>
            </div>
            <strong>{percentage}%</strong>
          </div>
          <p className="usage-line">
            {usage.tokens_used.toLocaleString()} / {usage.token_limit.toLocaleString()} tokens
          </p>
          <div className="demo-usage-bar">
            <div className="demo-usage-fill" style={{ width: `${percentage}%` }} />
          </div>
        </div>
      )}

      {usage && usage.by_model.length > 0 && (
        <div className="card table-wrap nx-data-panel">
          <h3>{t("usage.byModelTitle")}</h3>
          <table>
            <thead>
              <tr>
                <th>{t("models.colModel")}</th>
                <th>{t("usage.colTokens")}</th>
              </tr>
            </thead>
            <tbody>
              {usage.by_model.map((row) => (
                <tr key={row.model_id}>
                  <td>{row.model_id}</td>
                  <td>{row.total_tokens.toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="card table-wrap nx-data-panel">
        <div className="nx-table-heading">
          <div>
            <span>MODEL ACCESS MATRIX</span>
            <h3>{t("usage.catalogTitle")}</h3>
          </div>
          <strong>{models.length}</strong>
        </div>
        {modelsError && !authError && (
          <p className="form-note muted" role="status">
            {t("usage.catalogError")}
          </p>
        )}
        <table>
          <thead>
            <tr>
              <th>{t("models.colModel")}</th>
              <th>{t("models.colProvider")}</th>
              <th>{t("models.colMinPlan")}</th>
              <th>{t("models.colPrice")}</th>
              <th>{t("models.colStatus")}</th>
            </tr>
          </thead>
          <tbody>
            {models.length === 0 && (
              <tr>
                <td colSpan={5} className="nx-table-empty">
                  {t("common.empty")}
                </td>
              </tr>
            )}
            {models.map((m) => (
              <tr key={m.id}>
                <td>{m.display_name}</td>
                <td>{m.provider_id}</td>
                <td>{m.min_plan}</td>
                <td>¥{m.price_per_1m_input_cny.toFixed(2)}</td>
                <td>
                  <span className={m.available ? "status-ok" : "status-muted"}>
                    {m.available ? t("models.available") : t("models.upgradeRequired")}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </ConsolePage>
  );
}
