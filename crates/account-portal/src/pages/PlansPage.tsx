import { useEffect, useState } from "react";
import { api, type PaymentOrder, type PaymentProvider } from "../api";
import { ConsolePage } from "../components/ConsolePage";
import { WeChatPayModal } from "../components/WeChatPayModal";
import { useT } from "../i18n/context";
import { usePlanTiers } from "../lib/plans";

export function PlansPage() {
  const t = useT();
  const { plans, loading: plansLoading } = usePlanTiers();
  const [plan, setPlan] = useState("free");
  const [status, setStatus] = useState("");
  const [msg, setMsg] = useState<string | null>(null);
  const [provider, setProvider] = useState<PaymentProvider>("wechat");
  const [billingCycle, setBillingCycle] = useState<"monthly" | "yearly">("monthly");
  const [wechatOrder, setWechatOrder] = useState<PaymentOrder | null>(null);

  const refresh = () => {
    void api.bundle().then((b) => {
      setPlan(b.account.subscription.plan);
      setStatus(b.account.subscription.status);
    });
  };

  useEffect(() => {
    refresh();
  }, []);

  const checkout = async (tier: string) => {
    setMsg(null);
    try {
      const res = await api.checkout(tier, provider, billingCycle);
      if (res.provider === "stripe" && res.checkout_url) {
        window.location.href = res.checkout_url;
        return;
      }
      if (res.provider === "wechat" && res.order) {
        setWechatOrder(res.order);
        return;
      }
      setMsg(t("plans.checkoutError"));
    } catch (e) {
      setMsg(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <ConsolePage title={t("console.plans")} description={t("plans.pageDesc")}>
      <div className="nx-plan-toolbar">
        <div>
          <span className="nx-section-label">CURRENT SUBSCRIPTION</span>
          <strong>{plan}</strong>
          <span className="nx-status-pill">{status}</span>
        </div>
        <p>{t("plans.localFreeNote")}</p>
      </div>
      <div className="pay-provider-bar nx-console-segment">
        <span className="muted">{t("plans.payProvider")}</span>
        <div className="pay-provider-toggle">
          <button
            type="button"
            className={`btn btn-sm${provider === "wechat" ? " btn-primary" : " btn-secondary"}`}
            onClick={() => setProvider("wechat")}
          >
            {t("plans.wechatPay")}
          </button>
          <button
            type="button"
            className={`btn btn-sm${provider === "stripe" ? " btn-primary" : " btn-secondary"}`}
            onClick={() => setProvider("stripe")}
          >
            {t("plans.stripePay")}
          </button>
        </div>
        <div className="pay-cycle-toggle">
          <label>
            <input
              type="radio"
              name="cycle"
              checked={billingCycle === "monthly"}
              onChange={() => setBillingCycle("monthly")}
            />
            {t("plans.cycleMonthly")}
          </label>
          <label>
            <input
              type="radio"
              name="cycle"
              checked={billingCycle === "yearly"}
              onChange={() => setBillingCycle("yearly")}
            />
            {t("plans.cycleYearly")}
          </label>
        </div>
      </div>
      {msg && <p className="form-note">{msg}</p>}
      {plansLoading && <p className="muted">{t("common.loading")}</p>}
      <div className="plan-grid plan-grid-console">
        {plans.map((p, index) => (
          <div
            className={`card plan-card nx-plan-card${p.id === plan ? " plan-card-current" : ""}${p.featured ? " plan-card-featured" : ""}`}
            key={p.id}
          >
            <span className="nx-plan-card__index">0{index + 1}</span>
            {(p.promoLabel || p.featured) && (
              <span className="plan-badge">{p.promoLabel || t("common.recommended")}</span>
            )}
            <h3>{p.name}</h3>
            <p className="plan-price">{p.price}</p>
            <p className="muted">{p.desc}</p>
            <ul className="plan-highlights">
              {p.highlights.map((h) => (
                <li key={h}>{h}</li>
              ))}
            </ul>
            <div className="plan-actions">
              {p.id !== "free" && (
                <button
                  className="btn btn-primary"
                  type="button"
                  onClick={() => void checkout(p.id)}
                >
                  {provider === "wechat" ? t("plans.wechatSubscribe") : t("plans.stripeSubscribe")}
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
      {wechatOrder && (
        <WeChatPayModal
          order={wechatOrder}
          onClose={() => setWechatOrder(null)}
          onPaid={() => {
            setWechatOrder(null);
            refresh();
            setMsg(t("plans.wechatPaid"));
          }}
        />
      )}
    </ConsolePage>
  );
}
