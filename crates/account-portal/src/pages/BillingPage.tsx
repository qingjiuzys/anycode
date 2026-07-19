import { useEffect, useState } from "react";
import { api } from "../api";
import { ConsolePage } from "../components/ConsolePage";
import { useLocale, useT } from "../i18n/context";
import { formatMoney } from "../lib/money";

type InvoiceRow = {
  number: string;
  amount_fen: number;
  currency: "CNY";
  status: string;
};

export function BillingPage() {
  const t = useT();
  const locale = useLocale();
  const [invoices, setInvoices] = useState<InvoiceRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [tipNumber, setTipNumber] = useState<string | null>(null);

  const load = () => {
    setLoading(true);
    setError(false);
    void api
      .bundle()
      .then((b) => setInvoices(b.account.invoices))
      .catch(() => setError(true))
      .finally(() => setLoading(false));
  };

  useEffect(load, []);

  const formatAmount = (inv: InvoiceRow) =>
    formatMoney(inv.amount_fen, locale === "zh" ? "zh-CN" : "en-CN");

  return (
    <ConsolePage title={t("console.billing")} description={t("billing.description")}>
      <div className="nx-billing-strip">
        <span>
          <small>INVOICES</small>
          <strong>{invoices.length}</strong>
        </span>
        <span>
          <small>PAID</small>
          <strong>{invoices.filter((invoice) => invoice.status === "paid").length}</strong>
        </span>
        <span>
          <small>CURRENCY</small>
          <strong>CNY</strong>
        </span>
      </div>
      {error && (
        <div className="nx-empty-state" role="alert">
          <strong>{t("common.loadError")}</strong>
          <button className="btn btn-secondary btn-sm" type="button" onClick={load}>
            {t("common.retry")}
          </button>
        </div>
      )}
      <div className="card table-wrap nx-data-panel">
        <div className="nx-table-heading">
          <div>
            <span>BILLING LEDGER</span>
            <h3>{t("billing.invoicesTitle")}</h3>
          </div>
        </div>
        <table>
          <thead>
            <tr>
              <th>{t("billing.colNumber")}</th>
              <th>{t("billing.colAmount")}</th>
              <th>{t("billing.colStatus")}</th>
              <th>{t("billing.colAction")}</th>
            </tr>
          </thead>
          <tbody>
            {!error && !loading && invoices.length === 0 && (
              <tr>
                <td colSpan={4} className="nx-table-empty">
                  {t("common.empty")}
                </td>
              </tr>
            )}
            {loading && (
              <tr>
                <td colSpan={4} className="nx-table-empty">
                  {t("common.loading")}
                </td>
              </tr>
            )}
            {invoices.map((inv) => (
              <tr key={inv.number}>
                <td>{inv.number}</td>
                <td>{formatAmount(inv)}</td>
                <td>{inv.status}</td>
                <td>
                  <button
                    type="button"
                    className="btn btn-secondary btn-sm"
                    onClick={() => setTipNumber(inv.number)}
                  >
                    {t("billing.download")}
                  </button>
                  {tipNumber === inv.number && (
                    <span className="muted" style={{ marginLeft: 8 }}>
                      {t("billing.downloadUnavailable")}
                    </span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </ConsolePage>
  );
}
