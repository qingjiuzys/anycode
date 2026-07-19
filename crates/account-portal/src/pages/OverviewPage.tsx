import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../api";
import { ConsolePage } from "../components/ConsolePage";
import { useAuth } from "../hooks/useAuth";
import { formatMessage, useT } from "../i18n/context";

export function OverviewPage() {
  const t = useT();
  const { logout } = useAuth();
  const [data, setData] = useState<{
    plan: string;
    status: string;
    billingCycle: string;
    periodStart: string;
    periodEnd: string;
    tokensUsed: number;
    tokenLimit: number;
    apiKeyLimit: number;
    seatLimit: number;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = () => {
    setError(null);
    void api
      .bundle()
      .then((b) => {
        const sub = b.account.subscription as {
          plan: string;
          status: string;
          billing_cycle?: string;
          period_start?: string;
          period_end?: string;
        };
        const ent = b.account.entitlements as {
          token_limit: number;
          tokens_used: number;
          api_key_limit?: number;
          seat_limit?: number;
        };
        setData({
          plan: sub.plan,
          status: sub.status,
          billingCycle: sub.billing_cycle ?? "monthly",
          periodStart: sub.period_start ?? "—",
          periodEnd: sub.period_end ?? "—",
          tokensUsed: ent.tokens_used,
          tokenLimit: ent.token_limit,
          apiKeyLimit: ent.api_key_limit ?? 1,
          seatLimit: ent.seat_limit ?? 1,
        });
      })
      .catch((err) => {
        setData(null);
        setError(err instanceof Error ? err.message : String(err));
      });
  };

  useEffect(() => {
    load();
  }, []);

  if (error) {
    const isAuth = error.startsWith("401:");
    return (
      <ConsolePage title={t("console.overview")} description={t("overview.loading")}>
        <div className="nx-empty-state" role="alert">
          <strong>{isAuth ? t("common.sessionExpired") : t("common.loadError")}</strong>
          <p className="muted form-note">{error}</p>
          {isAuth ? (
            <button className="btn btn-primary btn-sm" type="button" onClick={() => logout()}>
              {t("common.signInAgain")}
            </button>
          ) : (
            <button className="btn btn-secondary btn-sm" type="button" onClick={load}>
              {t("common.retry")}
            </button>
          )}
        </div>
      </ConsolePage>
    );
  }

  if (!data) {
    return (
      <ConsolePage title={t("console.overview")} description={t("overview.loading")}>
        <p className="muted">{t("common.loading")}</p>
      </ConsolePage>
    );
  }

  const pct = data.tokenLimit > 0 ? Math.min(100, Math.round((data.tokensUsed / data.tokenLimit) * 100)) : 0;

  return (
    <ConsolePage
      title={t("console.overview")}
      description={formatMessage(t("overview.subtitle"), {
        plan: data.plan,
        start: data.periodStart,
        end: data.periodEnd,
      })}
    >
      <div className="overview-kpi-grid nx-kpi-grid">
        <Link className="card overview-kpi-card nx-kpi nx-kpi--cyan" to="/console/plans">
          <span className="nx-kpi__index">01</span>
          <small className="muted">{t("overview.kpiPlan")}</small>
          <strong>{data.plan}</strong>
          <span className="muted">{data.billingCycle}</span>
        </Link>
        <Link className="card overview-kpi-card nx-kpi nx-kpi--lime" to="/console/usage">
          <span className="nx-kpi__index">02</span>
          <small className="muted">{t("overview.kpiTokens")}</small>
          <strong>
            {data.tokensUsed.toLocaleString()} / {data.tokenLimit.toLocaleString()}
          </strong>
          <span className="muted">{pct}%</span>
        </Link>
        <Link className="card overview-kpi-card nx-kpi nx-kpi--amber" to="/console/api">
          <span className="nx-kpi__index">03</span>
          <small className="muted">{t("overview.kpiApi")}</small>
          <strong>{data.apiKeyLimit}</strong>
          <span className="muted">{t("overview.kpiApiHint")}</span>
        </Link>
        <Link className="card overview-kpi-card nx-kpi nx-kpi--rose" to="/console/settings">
          <span className="nx-kpi__index">04</span>
          <small className="muted">{t("overview.kpiStatus")}</small>
          <strong>{data.status}</strong>
          <span className="muted">{data.seatLimit} seats</span>
        </Link>
      </div>

      <div className="card nx-quota-card">
        <div className="nx-section-heading">
          <div>
            <span>USAGE WINDOW</span>
            <h3>{t("overview.quotaTitle")}</h3>
          </div>
          <strong>{pct}%</strong>
        </div>
        <div className="demo-usage-bar">
          <div className="demo-usage-fill" style={{ width: `${pct}%` }} />
        </div>
        <p className="muted console-meta">
          {data.periodStart} — {data.periodEnd}
        </p>
      </div>

      <div className="overview-actions">
        <Link className="btn btn-primary" to="/console/plans">
          {t("overview.ctaUpgrade")}
        </Link>
        <Link className="btn btn-secondary" to="/console/billing">
          {t("overview.ctaBilling")}
        </Link>
      </div>
    </ConsolePage>
  );
}
